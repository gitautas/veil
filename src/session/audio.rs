use std::{
    path::Path,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

use crossbeam_channel as crossbeam;

use crate::session::SessionHandle;
use crate::waking_sender::WakingSender;

mod buffer;
mod pulse;

use anyhow::Context as _;
use pulse::{CAPTURE_CHANNEL_COUNT, CAPTURE_SAMPLE_RATE, PulseServer};
use tracing::{debug, error, info};

/// Opus bitrate for the mixed stereo stream.
const OPUS_BITRATE: i32 = 128_000;
/// Frames in flight between the PulseAudio server and the encoder. The server
/// drops audio when none are free, so this bounds how far encoding can lag.
const FRAMES_IN_FLIGHT: usize = 8;

use crate::session::control::AudioStreamParams;

struct EncodeFrame {
    buf: Vec<f32>,
    capture_ts: u64,
}

struct Encoder {
    thread_handle: Option<std::thread::JoinHandle<anyhow::Result<()>>>,
    close_tx: mpsc::Sender<()>,
}

impl Drop for Encoder {
    fn drop(&mut self) {
        if let Some(handle) = self.thread_handle.take() {
            let _ = self.close_tx.send(());

            match handle.join() {
                Ok(Ok(())) => (),
                Ok(Err(e)) => error!("audio encoder thread died: {}", e),
                Err(_) => error!("audio encoder thread panicked"),
            }
        }
    }
}

pub struct EncodePipeline {
    server_thread_handle: Option<std::thread::JoinHandle<anyhow::Result<()>>>,
    server_close_tx: WakingSender<()>,

    compositor: SessionHandle,

    /// Running while streaming.
    encoder: Option<Encoder>,
    /// Empty frames go back to the PulseAudio server on this channel...
    done_tx: crossbeam::Sender<EncodeFrame>,
    /// ...and come back filled with 10 ms of mixed audio on this one.
    unencoded_rx: Arc<Mutex<crossbeam::Receiver<EncodeFrame>>>,
}

impl EncodePipeline {
    pub fn new(
        compositor: SessionHandle,
        xdg_runtime_dir: &Path,
    ) -> anyhow::Result<EncodePipeline> {
        // The PulseAudio server runs for the whole session, so apps can always
        // connect; audio is only encoded while streaming.
        std::fs::create_dir_all(Path::join(xdg_runtime_dir, "pulse"))?;
        let socket_name = Path::join(xdg_runtime_dir, "pulse/native");

        // The pulse server reads empty frames from the done channel, fills
        // them, and sends them back over the unencoded channel.
        let (unencoded_tx, unencoded_rx) = crossbeam::unbounded();
        let (done_tx, done_rx) = crossbeam::unbounded();

        let (mut server, close_tx) = PulseServer::new(&socket_name, unencoded_tx, done_rx)
            .context("creating PulseAudio server")?;

        let server_handle = std::thread::Builder::new()
            .name(format!("pulse server ({})", socket_name.to_string_lossy()))
            .spawn(move || server.run())?;

        Ok(Self {
            server_thread_handle: Some(server_handle),
            server_close_tx: close_tx,

            compositor,

            encoder: None,
            done_tx,
            unencoded_rx: Arc::new(Mutex::new(unencoded_rx)),
        })
    }

    pub fn stop_stream(&mut self) {
        self.encoder = None;
    }

    /// Start encoding (or restart, if already running).
    pub fn restart_stream(&mut self, params: AudioStreamParams) -> anyhow::Result<()> {
        self.encoder = None;

        anyhow::ensure!(
            params.sample_rate == CAPTURE_SAMPLE_RATE && params.channels == CAPTURE_CHANNEL_COUNT,
            "unsupported audio params {params:?}"
        );

        // Drop audio captured while we weren't encoding.
        let rx = self.unencoded_rx.clone();
        while rx.lock().unwrap().try_recv().is_ok() {}

        let mut opus = opus::Encoder::new(
            CAPTURE_SAMPLE_RATE,
            opus::Channels::Stereo,
            opus::Application::LowDelay,
        )
        .context("creating Opus encoder")?;
        opus.set_bitrate(opus::Bitrate::Bits(OPUS_BITRATE))?;

        // Give the server frames to fill.
        for _ in 0..FRAMES_IN_FLIGHT {
            let _ = self.done_tx.send(EncodeFrame { buf: Vec::new(), capture_ts: 0 });
        }

        let (close_tx, close_rx) = mpsc::channel();
        let done_tx = self.done_tx.clone();
        let sink = self.compositor.clone();
        let thread_handle = std::thread::Builder::new()
            .name("audio encoder".to_owned())
            .spawn(move || -> anyhow::Result<()> {
                let mut packet = vec![0u8; 4000];
                let mut packets = 0u64;
                loop {
                    if close_rx.try_recv().is_ok() {
                        return Ok(());
                    }
                    let frame = match rx.lock().unwrap().recv_timeout(Duration::from_millis(100)) {
                        Ok(frame) => frame,
                        Err(crossbeam::RecvTimeoutError::Timeout) => continue,
                        Err(crossbeam::RecvTimeoutError::Disconnected) => return Ok(()),
                    };

                    let len = opus
                        .encode_float(&frame.buf, &mut packet)
                        .context("Opus encode")?;
                    sink.dispatch_audio_frame(
                        frame.capture_ts,
                        bytes::Bytes::copy_from_slice(&packet[..len]),
                        false,
                    );
                    packets += 1;
                    if packets % 1000 == 0 {
                        debug!(packets, len, "audio packets encoded");
                    }

                    // Hand the buffer back to be refilled.
                    if done_tx.send(frame).is_err() {
                        return Ok(());
                    }
                }
            })?;

        info!(bitrate = OPUS_BITRATE, "audio encoding started (Opus, 48 kHz stereo, 10 ms)");
        self.encoder = Some(Encoder { thread_handle: Some(thread_handle), close_tx });
        Ok(())
    }
}

impl Drop for EncodePipeline {
    fn drop(&mut self) {
        let _ = self.server_close_tx.send(());

        if let Some(handle) = self.server_thread_handle.take() {
            match handle.join() {
                Ok(Ok(())) => (),
                Ok(Err(e)) => error!("pulseaudio server error: {}", e),
                Err(_) => error!("pulseaudio server panicked"),
            }
        }
    }
}
