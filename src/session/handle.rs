use std::sync::Arc;

use tokio::sync::broadcast;

use super::control::SessionEvent;

/// A cheap-to-clone handle for broadcasting session events from the compositor
/// reactor to all connected transport clients.
///
/// The underlying broadcast channel is created by the caller and shared
/// between the reactor (writer) and the transport server (readers via
/// `subscribe()`).  Sending is synchronous and safe to call from any thread,
/// including non-tokio threads.
#[derive(Clone)]
pub struct SessionHandle {
    tx: Arc<broadcast::Sender<SessionEvent>>,
    waker: Arc<mio::Waker>,
}

impl SessionHandle {
    pub fn new(waker: Arc<mio::Waker>, tx: broadcast::Sender<SessionEvent>) -> Self {
        Self {
            tx: Arc::new(tx),
            waker,
        }
    }

    /// Fan-out an event to all currently subscribed clients.
    pub fn dispatch(&self, event: SessionEvent) {
        let _ = self.tx.send(event);
    }

    /// Send `SessionEvent::Shutdown` to all clients, causing them to
    /// disconnect gracefully.
    pub fn kick_clients(&self) {
        let _ = self.tx.send(SessionEvent::Shutdown);
    }

    /// Called by the video encoder sink to deliver an encoded frame.
    pub fn dispatch_video_frame(
        &self,
        pts: u64,
        frame: bytes::Bytes,
        _hierarchical_layer: u32,
        is_keyframe: bool,
    ) {
        let _ = self.tx.send(SessionEvent::VideoFrame { seq: pts, is_keyframe, frame });
    }

    /// Called by the audio encoder to deliver an Opus packet. `pts` is
    /// milliseconds since `EPOCH`, the same clock as video.
    pub fn dispatch_audio_frame(&self, pts: u64, frame: bytes::Bytes, _stream_restart: bool) {
        let _ = self.tx.send(SessionEvent::AudioFrame { seq: pts, frame });
    }

    /// Wake the reactor's mio poll loop (used after delivering a frame so the
    /// compositor can release Wayland buffers and send presentation feedback).
    pub fn wake(&self) -> std::io::Result<()> {
        self.waker.wake()
    }
}
