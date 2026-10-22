//! QUIC / WebTransport media transport layer.
//!
//! Replaces the old WebRTC transport with a direct QUIC stack that browsers
//! reach via the [WebTransport API][wt].  No SDP, no ICE, no STUN/TURN —
//! clients connect directly over HTTP/3 (QUIC).
//!
//! ## TLS
//! A short-lived self-signed certificate is generated at startup via
//! `wtransport::Identity::self_signed`.  Its SHA-256 fingerprint is served
//! over plain HTTP on `<quic_port + 1>` so browsers can pass it in
//! `serverCertificateHashes`.
//!
//! ## Ports (defaults)
//! - `7801` — QUIC / WebTransport (`https://host:7801/`)
//! - `7802` — HTTP cert-hash endpoint (`http://host:7802/cert-hash`)

pub mod proto;
mod session;
mod viewers;

use std::net::SocketAddr;

use anyhow::{Context, Result};
use axum::{Json, Router, routing::get};
use sha2::{Digest, Sha256};
use tokio::sync::{broadcast, watch};
use tower_http::cors::CorsLayer;
use tracing::{info, warn};
use wtransport::{Identity, ServerConfig, endpoint::SessionRequest};

use crate::{
    session::control::{ControlMessage, SessionEvent, VideoStreamParams},
    waking_sender::WakingSender,
};

/// WebTransport media server.
///
/// Call [`TransportServer::new`] to create, [`TransportServer::start_http`] to
/// start serving the cert-hash endpoint early, then [`TransportServer::run`] to
/// start accepting WebTransport clients once the compositor is ready.
pub struct TransportServer {
    quic_addr: SocketAddr,
    http_addr: SocketAddr,
    identity: Identity,
    cert_fingerprint: String,
    video_params: VideoStreamParams,
}

impl TransportServer {
    /// Create a server, generating a fresh self-signed TLS certificate.
    ///
    /// The certificate is valid for 14 days, which is the maximum period
    /// browsers accept for `serverCertificateHashes`.
    ///
    /// Does **not** bind any ports. Call [`start_http`](Self::start_http) to
    /// serve the cert-hash endpoint and [`run`](Self::run) to accept clients.
    pub fn new(
        quic_addr: SocketAddr,
        video_params: VideoStreamParams,
    ) -> Result<Self> {
        let http_port = quic_addr.port().saturating_add(1);
        let http_addr = SocketAddr::new(quic_addr.ip(), http_port);

        // Use wtransport's built-in self-signed cert generation.
        // The cert automatically has a short validity period for browser compat.
        // Browsers verify via serverCertificateHashes so hostname matching is
        // not required, but we include localhost for completeness.
        let san = vec!["localhost", "127.0.0.1"];

        let identity = Identity::self_signed(san)
            .context("failed to generate self-signed TLS identity")?;

        // Compute SHA-256 fingerprint of the leaf certificate DER.
        let cert_der = identity
            .certificate_chain()
            .as_slice()
            .first()
            .context("empty certificate chain")?
            .der();

        let cert_fingerprint = hex::encode(Sha256::digest(cert_der));

        info!(
            quic = %quic_addr,
            http = %http_addr,
            fingerprint = %cert_fingerprint,
            "TransportServer initialised"
        );
        info!("  WebTransport URL : https://{}", quic_addr);
        info!("  Cert fingerprint : {}", cert_fingerprint);
        info!("  Fetch via HTTP   : http://{}/cert-hash", http_addr);

        Ok(Self {
            quic_addr,
            http_addr,
            identity,
            cert_fingerprint,
            video_params,
        })
    }

    /// Bind and start the HTTP cert-hash / health server on `<quic_port+1>`.
    ///
    /// Should be called **early** (before the compositor is ready) so that the
    /// browser can fetch the fingerprint while the app is loading.  Safe to
    /// call multiple times — subsequent calls are no-ops if the port is already
    /// bound (the task is fire-and-forget).
    pub async fn start_http(&self) -> Result<()> {
        let fp = self.cert_fingerprint.clone();
        let http_router = Router::new()
            .route(
                "/cert-hash",
                get({
                    let fp = fp.clone();
                    move || {
                        let fp = fp.clone();
                        async move {
                            Json(serde_json::json!({
                                "sha256_hex": fp,
                                "algorithm":  "sha-256"
                            }))
                        }
                    }
                }),
            )
            .route(
                "/health",
                get(|| async { Json(serde_json::json!({ "status": "ok" })) }),
            )
            .layer(CorsLayer::permissive());

        let http_addr = self.http_addr;
        let listener = tokio::net::TcpListener::bind(http_addr)
            .await
            .with_context(|| format!("binding cert-hash HTTP server on {http_addr}"))?;
        info!("HTTP cert-hash server on {}", http_addr);
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, http_router).await {
                warn!("HTTP server error: {e}");
            }
        });
        Ok(())
    }

    /// Start the QUIC server and accept WebTransport clients.
    ///
    /// Can be called **before** the compositor is ready.  `ctrl_rx` is a watch
    /// channel that delivers the reactor's control handle once it becomes
    /// available; client input is silently discarded until that point.
    /// `events_rx` carries [`SessionEvent`]s (video/audio/ctrl) to clients.
    ///
    /// Call [`start_http`](Self::start_http) first so the cert-hash endpoint is
    /// reachable from the moment the process starts.
    ///
    /// Returns once `shutdown` fires (or its sender is dropped), after closing
    /// the endpoint and giving connections a moment to shut down.
    pub async fn run(
        self,
        mut ctrl_rx: watch::Receiver<Option<WakingSender<ControlMessage>>>,
        events_tx: broadcast::Sender<SessionEvent>,
        mut shutdown: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<()> {
        // ── QUIC / WebTransport endpoint ──────────────────────────────────────
        let quic_config = ServerConfig::builder()
            .with_bind_address(self.quic_addr)
            .with_identity(self.identity)
            .build();

        let endpoint = wtransport::Endpoint::server(quic_config)
            .context("failed to create WebTransport QUIC endpoint")?;

        info!("WebTransport listening on {}", self.quic_addr);

        // Refresh the viewer list now and then, so durations and "controlling"
        // stay current. The task ends with the runtime.
        let viewers = viewers::Viewers::new(events_tx.clone());
        {
            let viewers = viewers.clone();
            tokio::spawn(async move {
                let mut tick = tokio::time::interval(std::time::Duration::from_secs(2));
                loop {
                    tick.tick().await;
                    if !viewers.is_empty() {
                        viewers.broadcast();
                    }
                }
            });
        }

        // Accept loop — runs until shutdown.
        // wtransport 0.7: accept() always returns an IncomingSession (never None/Option).
        loop {
            let incoming = tokio::select! {
                incoming = endpoint.accept() => incoming,
                _ = &mut shutdown => break,
            };

            let bcast_sub = events_tx.subscribe();
            let video_params = self.video_params;
            // Clone the watch receiver so each session can independently wait
            // for the control channel to become available.
            let ctrl_rx_clone = ctrl_rx.clone();
            let viewers = viewers.clone();

            tokio::spawn(async move {
                // Await the incoming session request.
                let session_req: SessionRequest = match incoming.await {
                    Ok(r) => r,
                    Err(e) => {
                        warn!("Incoming session error: {e}");
                        return;
                    }
                };

                // Accept the WebTransport session (performs HTTP/3 upgrade).
                let conn = match session_req.accept().await {
                    Ok(c) => c,
                    Err(e) => {
                        warn!("Session accept error: {e}");
                        return;
                    }
                };

                let viewer = viewers.join(conn.remote_address());
                let cfg = session::SessionConfig {
                    video_params,
                    events_rx: bcast_sub,
                    ctrl_rx: ctrl_rx_clone,
                    viewers: viewers.clone(),
                    viewer,
                };

                if let Err(e) = session::run_client_session(conn, cfg).await {
                    warn!("Client session error: {e}");
                }
            });

            // Keep the local ctrl_rx marked-as-seen so clones start fresh.
            let _ = ctrl_rx.borrow_and_update();
        }

        info!("WebTransport server shutting down");
        endpoint.close(wtransport::VarInt::from_u32(0), b"server shutting down");
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), endpoint.wait_idle()).await;
        Ok(())
    }

    pub fn quic_addr(&self) -> SocketAddr {
        self.quic_addr
    }

    pub fn http_addr(&self) -> SocketAddr {
        self.http_addr
    }

    pub fn cert_fingerprint(&self) -> &str {
        &self.cert_fingerprint
    }
}
