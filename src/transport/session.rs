//! Per-client WebTransport session handler.
//!
//! For each browser that connects, one `ClientSession` is spawned as a Tokio
//! task.  It:
//!
//! 1. Sends a **hello** stream announcing codec / resolution.
//! 2. Receives [`SessionEvent`]s from the compositor reactor and serialises
//!    them as QUIC unidirectional streams (video / audio / control).
//! 3. Receives QUIC datagrams from the client (input events) and forwards
//!    them to the reactor via a cloned [`WakingSender`].

use anyhow::{Context, Result};
use bytes::Bytes;
use tokio::sync::{broadcast, watch};
use tracing::{debug, info, warn};
use wtransport::Connection;

use crate::{
    codec::VideoCodec,
    session::control::{ControlMessage, SessionEvent, VideoStreamParams},
    transport::proto::{self, CODEC_AV1, CODEC_H264, CODEC_H265},
    waking_sender::WakingSender,
};

// ─── Configuration ───────────────────────────────────────────────────────────

pub struct SessionConfig {
    pub video_params: VideoStreamParams,
    pub events_rx: broadcast::Receiver<SessionEvent>,
    /// Watch channel for the reactor's control handle.
    /// Starts as `None` and becomes `Some` once the compositor is ready.
    /// Input datagrams are discarded until then.
    pub ctrl_rx: watch::Receiver<Option<WakingSender<ControlMessage>>>,
    pub viewers: std::sync::Arc<super::viewers::Viewers>,
    /// This session's entry in `viewers`; removed when the session ends.
    pub viewer: super::viewers::ViewerGuard,
}

// ─── Stream send helper ──────────────────────────────────────────────────────

/// Write `data` as a single QUIC unidirectional stream and send FIN.
///
/// Returns once the data is queued. Waiting for the peer to acknowledge it
/// (which `finish()` does) would limit us to one message per round trip, so
/// that part runs in the background.
async fn send_stream(conn: &Connection, data: Bytes) -> Result<()> {
    // In wtransport 0.7, open_uni() returns an OpeningUniStream future that must
    // be awaited twice: once to submit the open request, once to get the SendStream.
    let mut stream = conn
        .open_uni()
        .await
        .context("failed to open QUIC uni stream")?
        .await
        .context("failed to obtain send stream")?;
    stream
        .write_all(&data)
        .await
        .context("failed to write stream payload")?;
    tokio::spawn(async move {
        if let Err(e) = stream.finish().await {
            debug!("failed to finish QUIC stream: {e}");
        }
    });
    Ok(())
}

// ─── Session entry point ─────────────────────────────────────────────────────

/// Run a single connected client's WebTransport session to completion.
pub async fn run_client_session(conn: Connection, mut cfg: SessionConfig) -> Result<()> {
    let peer = conn.remote_address().to_string();
    info!(peer = %peer, "WebTransport client connected");

    let codec_byte = match cfg.video_params.codec {
        VideoCodec::H265 => CODEC_H265,
        VideoCodec::Av1 => CODEC_AV1,
        VideoCodec::H264 => CODEC_H264,
    };

    // ── Hello ────────────────────────────────────────────────────────────────
    let profile_idc = cfg.video_params.h264_profile_idc;
    let level_idc = cfg.video_params.h264_level_idc;
    let gop_size = cfg.video_params.gop_size;
    let in_flight = cfg.video_params.in_flight_frames.min(255) as u8;
    let hello = proto::encode_hello(
        codec_byte,
        cfg.video_params.width,
        cfg.video_params.height,
        cfg.video_params.framerate,
        profile_idc,
        level_idc,
        gop_size,
        in_flight,
    );
    if let Err(e) = send_stream(&conn, hello).await {
        warn!(peer = %peer, "Failed to send hello: {e}");
        return Err(e);
    }

    let refresh_rx = cfg.ctrl_rx.clone();
    let mut ctrl_rx = cfg.ctrl_rx;

    // ── Input datagram receiver ───────────────────────────────────────────────
    // The WakingSender is Send + Clone so we can move a clone into this task.
    let my_id = cfg.viewer.id;
    // Pad indices this viewer has used, so they can be unplugged when it
    // leaves (otherwise a held button would stay held).
    let my_pads = std::sync::Arc::new(std::sync::Mutex::new(std::collections::BTreeSet::<u8>::new()));
    let input_task = {
        let conn_ref = conn.clone();
        let viewers = cfg.viewers.clone();
        let my_pads = my_pads.clone();
        tokio::spawn(async move {
            // Wait until the reactor control channel is available.
            // Datagrams that arrive before the compositor is ready are dropped.
            let ctrl = loop {
                {
                    let val = ctrl_rx.borrow_and_update().clone();
                    if let Some(c) = val {
                        break c;
                    }
                }
                if ctrl_rx.changed().await.is_err() {
                    return; // sender dropped — server shutting down
                }
            };

            // Sync state: push the current window list and force an IDR keyframe
            // so the new client can start decoding immediately without waiting
            // for the next natural keyframe interval.
            let _ = ctrl.send(ControlMessage::RefreshWindowList);
            let _ = ctrl.send(ControlMessage::RefreshStreamInfo);
            let _ = ctrl.send(ControlMessage::RefreshVideo);

            loop {
                match conn_ref.receive_datagram().await {
                    Ok(dgram) => {
                        // Fast path: echo ping directly without routing through the
                        // reactor → main_loop → transport event queue.  That path
                        // serialises pong behind every queued video frame (2–3 s at
                        // 60 fps), making RTT appear enormous.  Handling it here
                        // cuts pong latency to just the QUIC round-trip.
                        if dgram.first() == Some(&proto::INPUT_PING) {
                            if dgram.len() >= 9 {
                                let ts = u64::from_be_bytes([
                                    dgram[1], dgram[2], dgram[3], dgram[4],
                                    dgram[5], dgram[6], dgram[7], dgram[8],
                                ]);
                                let _ = conn_ref.send_datagram(
                                    proto::encode_ctrl_pong(ts).as_ref(),
                                );
                            }
                            continue; // skip reactor for ping
                        }
                        if let Some(mut msg) = proto::parse_input_datagram(&dgram) {
                            if let Some(index) = scope_pad_id(&mut msg, my_id) {
                                let mut pads = my_pads.lock().unwrap();
                                if matches!(msg, ControlMessage::GamepadUnavailable(_)) {
                                    pads.remove(&index);
                                } else {
                                    pads.insert(index);
                                }
                            }
                            if is_user_input(&msg) {
                                viewers.input(my_id);
                            }
                            if let Err(e) = ctrl.send(msg) {
                                debug!("Control channel closed: {e}");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Datagram receive ended: {e}");
                        break;
                    }
                }
            }
        })
    };

    // ── Session event → QUIC stream loop ─────────────────────────────────────
    // Delta frames are useless to the decoder once a frame they depend on has
    // been skipped, so after a skip (and before the first keyframe) we hold
    // video back until the next keyframe.
    let mut awaiting_keyframe = true;
    loop {
        let event = match cfg.events_rx.recv().await {
            Ok(ev) => ev,
            Err(broadcast::error::RecvError::Lagged(n)) => {
                warn!(
                    peer = %peer,
                    skipped = n,
                    "client fell behind; skipped events, requesting keyframe"
                );
                if !awaiting_keyframe {
                    awaiting_keyframe = true;
                    if let Some(ctrl) = refresh_rx.borrow().as_ref() {
                        let _ = ctrl.send(ControlMessage::RefreshVideo);
                    }
                }
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => {
                info!(peer = %peer, "Event channel closed");
                break;
            }
        };

        let result = match event {
            SessionEvent::VideoFrame { seq, is_keyframe, frame } => {
                if awaiting_keyframe && !is_keyframe {
                    continue;
                }
                awaiting_keyframe = false;
                let pts_ms = seq as u32; // seq is already wall-clock PTS in ms
                let payload = proto::encode_video(is_keyframe, seq, pts_ms, &frame);
                send_stream(&conn, payload).await
            }

            SessionEvent::AudioFrame { seq, frame } => {
                // `seq` is the capture time in ms since EPOCH (as for video),
                // so it doubles as the timestamp.
                let pts_ms = seq as u32;
                let payload = proto::encode_audio(seq, pts_ms, &frame);
                if payload.len() <= 1200 {
                    // Fits in a QUIC datagram — lower latency than a stream.
                    // send_datagram is synchronous in wtransport 0.7.
                    conn.send_datagram(payload.as_ref())
                        .context("audio datagram send failed")
                } else {
                    send_stream(&conn, payload).await
                }
            }

            SessionEvent::DisplayParamsChanged { params, .. } => {
                let payload =
                    proto::encode_ctrl_display(params.width, params.height, params.framerate);
                send_stream(&conn, payload).await
            }

            SessionEvent::CursorUpdate { image, icon, hotspot_x, hotspot_y } => {
                let icon_name = icon.map(|i| i.to_string());
                let payload = proto::encode_ctrl_cursor(
                    image.as_deref(),
                    icon_name.as_deref(),
                    hotspot_x,
                    hotspot_y,
                );
                send_stream(&conn, payload).await
            }

            SessionEvent::PointerLocked(x, y) => {
                send_stream(&conn, proto::encode_ctrl_pointer_lock(x, y)).await
            }

            SessionEvent::PointerReleased => {
                send_stream(&conn, proto::encode_ctrl_pointer_release()).await
            }

            SessionEvent::Pong(ts) => {
                // Send as datagram to bypass the video-frame stream queue.
                conn.send_datagram(proto::encode_ctrl_pong(ts).as_ref())
                    .context("pong datagram send failed")
            }

            SessionEvent::WindowList(windows) => {
                let payload = proto::encode_window_list(&windows);
                send_stream(&conn, payload).await
            }

            SessionEvent::StreamInfo(info) => {
                // The codec or size may have changed, so re-send the hello the
                // client configures its decoder from, then hold video back
                // until the new pipeline's first keyframe.
                let codec = match info.codec {
                    VideoCodec::H265 => CODEC_H265,
                    VideoCodec::Av1 => CODEC_AV1,
                    VideoCodec::H264 => CODEC_H264,
                };
                let hello = proto::encode_hello(
                    codec,
                    info.width,
                    info.height,
                    info.framerate,
                    cfg.video_params.h264_profile_idc,
                    cfg.video_params.h264_level_idc,
                    info.gop_size,
                    info.in_flight_frames.min(255) as u8,
                );
                awaiting_keyframe = true;
                match send_stream(&conn, hello).await {
                    Ok(()) => {
                        send_stream(&conn, proto::encode_json(proto::MSG_SERVER_INFO, &*info))
                            .await
                    }
                    Err(e) => Err(e),
                }
            }

            SessionEvent::EncoderStats(stats) => {
                send_stream(&conn, proto::encode_json(proto::MSG_SERVER_STATS, &stats)).await
            }

            SessionEvent::GamepadStatus(pads) => {
                send_stream(&conn, proto::encode_json(proto::MSG_GAMEPAD_STATUS, &pads)).await
            }

            SessionEvent::GamepadRumble { id, strong, weak, duration_ms } => {
                let (viewer, index) = crate::session::split_pad_id(id);
                if viewer != my_id {
                    continue;
                }
                send_stream(&conn, proto::encode_gamepad_rumble(index, strong, weak, duration_ms)).await
            }

            SessionEvent::Viewers(list) => {
                let payload = serde_json::json!({ "you": my_id, "viewers": &*list });
                send_stream(&conn, proto::encode_json(proto::MSG_VIEWERS, &payload)).await
            }

            SessionEvent::CompositorState(state) => {
                send_stream(&conn, proto::encode_json(proto::MSG_COMPOSITOR_STATE, &*state)).await
            }

            SessionEvent::Shutdown => {
                info!(peer = %peer, "Shutdown received");
                break;
            }
        };

        if let Err(e) = result {
            warn!(peer = %peer, "Send error: {e}");
            break;
        }
    }

    input_task.abort();
    // Unplug this viewer's pads.
    if let Some(ctrl) = refresh_rx.borrow().as_ref() {
        for index in std::mem::take(&mut *my_pads.lock().unwrap()) {
            let _ = ctrl.send(ControlMessage::GamepadUnavailable(((my_id as u64) << 8) | index as u64));
        }
    }
    info!(peer = %peer, "Client session ended");
    Ok(())
}

/// Input from a person (as opposed to housekeeping like keyframe requests or
/// gamepad hotplug).
fn is_user_input(msg: &ControlMessage) -> bool {
    matches!(
        msg,
        ControlMessage::KeyboardInput { .. }
            | ControlMessage::PointerMotion(..)
            | ControlMessage::RelativePointerMotion(..)
            | ControlMessage::PointerInput { .. }
            | ControlMessage::PointerAxis(..)
            | ControlMessage::PointerAxisDiscrete(..)
            | ControlMessage::GamepadAxis { .. }
            | ControlMessage::GamepadTrigger { .. }
            | ControlMessage::GamepadInput { .. }
            | ControlMessage::GamepadState { .. }
    )
}

/// Browsers number pads from 0, so give each viewer its own range of IDs
/// (viewer << 8 | index); otherwise two viewers' pad 0 would drive the same
/// virtual device. Returns the browser's index for gamepad messages.
fn scope_pad_id(msg: &mut ControlMessage, viewer: u32) -> Option<u8> {
    let id = match msg {
        ControlMessage::GamepadAvailable(id, _)
        | ControlMessage::GamepadUnavailable(id)
        | ControlMessage::GamepadAxis { id, .. }
        | ControlMessage::GamepadTrigger { id, .. }
        | ControlMessage::GamepadInput { id, .. }
        | ControlMessage::GamepadState { id, .. } => id,
        _ => return None,
    };
    let index = (*id & 0xff) as u8;
    *id = ((viewer as u64) << 8) | index as u64;
    Some(index)
}
