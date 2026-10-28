use std::{
    ffi::OsString,
    fs::File,
    io::{BufWriter, Write},
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, mpsc},
    time,
};

use anyhow::{Context as _, bail};
use tracing::info;

mod codec;
mod color;
mod encoder;
mod pixel_scale;
mod runtime;
mod session;
mod transport;
mod vulkan;
mod waking_sender;

use crate::{
    codec::VideoCodec,
    color::VideoProfile,
    pixel_scale::PixelScale,
    session::{
        AppConfig, GamepadLayout, Reactor,
        control::{ControlMessage, DisplayParams, SessionEvent, VideoStreamParams},
    },
    vulkan::VkContext,
    waking_sender::WakingSender,
};

struct RunOptions {
    command: Vec<OsString>,
    /// Where to write the encoded elementary stream (optional).
    output_path: Option<PathBuf>,

    width: u32,
    height: u32,
    framerate: u32,

    codec: VideoCodec,
    /// Pick the codec from what the viewers decode (`codec` is where it
    /// starts).
    auto_codec: bool,
    preset: u32,
    profile: VideoProfile,
    /// IDR interval in frames (0 = use encoder default of 60).
    gop_size: u32,
    /// Number of encoder output buffers in flight (1 = min latency, 0 = use default of 2).
    in_flight_frames: u32,

    xwayland: bool,
    force_1x_scale: bool,

    /// Stop after this duration (for non-interactive testing).
    max_duration: Option<time::Duration>,

    h264_profile_idc: u8,
    h264_level_idc: u8,

    /// Enable the QUIC/WebTransport streaming server.
    enable_transport: bool,
    /// QUIC/WebTransport server bind address.
    transport_addr: SocketAddr,
}

fn run(opts: RunOptions) -> anyhow::Result<()> {
    if opts.command.is_empty() {
        bail!("no command provided");
    }

    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    // The GPU contexts in VkContext need a running client.
    #[cfg(feature = "tracy")]
    let _tracy = tracy_client::Client::start();

    let vk = Arc::new(VkContext::new(cfg!(debug_assertions))?);

    let mut out: Option<BufWriter<File>> = if let Some(ref path) = opts.output_path {
        let file = File::create(path)
            .with_context(|| format!("creating output file {}", path.display()))?;
        Some(BufWriter::new(file))
    } else {
        None
    };

    let app_cfg = AppConfig {
        command: opts.command.clone(),
        env: Default::default(),
        xwayland: opts.xwayland,
        force_1x_scale: opts.force_1x_scale,
    };

    let display_params = DisplayParams {
        width: opts.width,
        height: opts.height,
        framerate: opts.framerate,
        ui_scale: PixelScale::ONE,
    };

    let video_params = VideoStreamParams {
        width: opts.width,
        height: opts.height,
        codec: opts.codec,
        preset: opts.preset,
        profile: opts.profile,
        framerate: opts.framerate,
        gop_size: opts.gop_size,
        in_flight_frames: if opts.in_flight_frames == 0 { 2 } else { opts.in_flight_frames },
        h264_profile_idc: opts.h264_profile_idc,
        h264_level_idc: opts.h264_level_idc,
    };

    // ── Early transport init ────────────────────────────────────────────────
    // Start the cert-hash HTTP server and the QUIC server immediately so
    // the browser can connect at any time while the compositor loads.
    //
    // The reactor control channel (needed to forward client input events to
    // the compositor) is not available until the compositor signals ready.
    // We deliver it via a watch channel that starts as None and is set to
    // Some once the compositor is ready; input datagrams are silently
    // discarded until then.
    let (ctrl_watch_tx, ctrl_watch_rx) =
        tokio::sync::watch::channel(None::<WakingSender<ControlMessage>>);

    let (events_bcast_tx, _events_bcast_rx) = tokio::sync::broadcast::channel::<SessionEvent>(512);

    let transport_thread = if opts.enable_transport {
        let rt = tokio::runtime::Runtime::new()
            .context("failed to create Tokio runtime")?;

        let server = transport::TransportServer::new(opts.transport_addr, video_params, opts.auto_codec)
            .context("failed to create TransportServer")?;

        info!(
            quic = %server.quic_addr(),
            http = %server.http_addr(),
            fingerprint = server.cert_fingerprint(),
            "WebTransport server ready"
        );

        // cert-hash HTTP — up immediately.
        rt.block_on(server.start_http())
            .context("failed to start cert-hash HTTP server")?;

        // QUIC — up immediately; input is buffered until ctrl_watch fires.
        let bcast_for_transport = events_bcast_tx.clone();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let handle = std::thread::Builder::new()
            .name("veil-transport".into())
            .spawn(move || -> anyhow::Result<()> {
                rt.block_on(server.run(ctrl_watch_rx, bcast_for_transport, shutdown_rx))
            })
            .context("spawning transport thread")?;
        Some(TransportThread { handle, shutdown_tx })
    } else {
        None
    };

    // ── Spawn the Wayland compositor (reactor) thread ────────────────────────
    let (ready_send, ready_recv) = mpsc::channel();
    let (error_send, error_recv) = mpsc::channel::<anyhow::Error>();
    let vk_clone = vk.clone();
    let app_cfg_clone = app_cfg.clone();
    let events_bcast_tx_for_reactor = events_bcast_tx.clone();
    let reactor_thread = std::thread::Builder::new()
        .name("veil-reactor".into())
        .spawn(move || {
            let gamepads: Vec<(u64, GamepadLayout)> = Vec::new();
            let result = Reactor::run(
                vk_clone,
                app_cfg_clone,
                display_params,
                gamepads,
                None,
                ready_send,
                events_bcast_tx_for_reactor,
            );
            if let Err(e) = result {
                let _ = error_send.send(e);
            }
        })
        .context("spawning reactor thread")?;

    // Wait for the compositor to become ready; it hands back its control channel.
    let control = match ready_recv.recv_timeout(time::Duration::from_secs(120)) {
        Ok(ctrl) => ctrl,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            if let Ok(err) = error_recv.try_recv() {
                return Err(err).context("reactor failed during init");
            }
            bail!("timed out waiting for reactor to become ready");
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            // There is a small race: ready_send is dropped (disconnecting the
            // channel) inside Reactor::run() before the spawned closure gets to
            // call error_send.send(). Give the thread up to 2 s to deliver the
            // error so we can surface the real cause instead of a generic message.
            let err = error_recv
                .recv_timeout(time::Duration::from_secs(2))
                .ok();
            if let Some(err) = err {
                return Err(err).context("reactor failed during init");
            }
            bail!("reactor thread ended unexpectedly");
        }
    };

    let (ready_tx, ready_rx) = oneshot::channel();
    control
        .send(ControlMessage::StartStream {
            video_params,
            ready: ready_tx,
        })
        .context("failed to send StartStream to reactor")?;
    let _ = ready_rx
        .recv_timeout(time::Duration::from_secs(10))
        .context("reactor denied StartStream")?;

    // Compositor is ready — unblock client input forwarding.
    let _ = ctrl_watch_tx.send(Some(control.clone()));

    info!(
        command = ?opts.command,
        width = opts.width,
        height = opts.height,
        framerate = opts.framerate,
        codec = ?opts.codec,
        auto_codec = opts.auto_codec,
        transport = opts.enable_transport,
        "Veil running"
    );

    // ── Main event loop (file output only) ──────────────────────────────────
    // Events flow directly from the reactor → broadcast → transport clients.
    // The only work here is optionally writing encoded frames to a file.
    let started_at = time::Instant::now();

    if out.is_some() || opts.max_duration.is_some() {
        let mut file_rx = events_bcast_tx.subscribe();

        'file_loop: loop {
            // The broadcast channel stays open (we hold a sender), so a dead
            // reactor has to be detected explicitly.
            if let Ok(err) = error_recv.try_recv() {
                return Err(err).context("reactor failed");
            }
            if reactor_thread.is_finished() {
                info!("reactor exited; stopping");
                break 'file_loop;
            }

            if let Some(max) = opts.max_duration {
                if started_at.elapsed() >= max {
                    info!("max duration reached; stopping");
                    let _ = control.send(ControlMessage::Stop);
                    break 'file_loop;
                }
            }

            match file_rx.try_recv() {
                Ok(SessionEvent::VideoFrame { frame, .. }) => {
                    if let Some(ref mut w) = out {
                        w.write_all(&frame)?;
                        w.flush()?;
                    }
                }
                Ok(SessionEvent::Shutdown) => break 'file_loop,
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::TryRecvError::Closed) => break 'file_loop,
            }
        }
    } else {
        // No file output and no time limit — just wait for the reactor to finish.
        if reactor_thread.join().is_err() {
            bail!("reactor thread panicked");
        }
        if let Ok(err) = error_recv.try_recv() {
            return Err(err).context("reactor failed");
        }
        exec_restart_if_requested()?;
        if let Some(t) = transport_thread {
            t.stop();
        }
        return Ok(());
    }

    if reactor_thread.join().is_err() {
        bail!("reactor thread panicked");
    }
    exec_restart_if_requested()?;
    if let Some(t) = transport_thread {
        t.stop();
    }
    Ok(())
}

struct TransportThread {
    handle: std::thread::JoinHandle<anyhow::Result<()>>,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl TransportThread {
    /// Stop accepting clients, close connections, and wait for the thread.
    fn stop(self) {
        let _ = self.shutdown_tx.send(());
        match self.handle.join() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!("transport server failed: {e:#}"),
            Err(_) => tracing::warn!("transport thread panicked"),
        }
    }
}

/// If a client asked to switch GPUs, replace this process with a new Veil on
/// that GPU, carrying over the current stream settings. Only returns on
/// failure (or if no restart was requested).
fn exec_restart_if_requested() -> anyhow::Result<()> {
    use std::os::unix::process::CommandExt as _;

    let Some(req) = session::control::RESTART.lock().unwrap().take() else {
        return Ok(());
    };

    // Our own arguments, with overrides inserted before the `--` that starts
    // the app command. Later flags win, so the overrides take effect.
    let mut args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let dashdash = match args.iter().position(|a| a == "--") {
        Some(pos) => pos,
        None => {
            args.insert(0, "--".into());
            0
        }
    };
    let mut overrides = Vec::<OsString>::new();
    if let Some(p) = req.video_params {
        let codec = match p.codec {
            _ if transport::auto_codec_enabled() => "auto",
            VideoCodec::H265 => "h265",
            VideoCodec::Av1 => "av1",
            VideoCodec::H264 => "h264",
        };
        for (flag, value) in [
            ("--codec", codec.to_owned()),
            ("--size", format!("{}x{}", p.width, p.height)),
            ("--fps", p.framerate.to_string()),
            ("--gop-size", p.gop_size.to_string()),
            ("--in-flight", p.in_flight_frames.to_string()),
            ("--preset", p.preset.to_string()),
        ] {
            overrides.push(flag.into());
            overrides.push(value.into());
        }
    }
    args.splice(dashdash..dashdash, overrides);

    info!(gpu = req.gpu, ?args, "restarting veil on another gpu");
    let err = std::process::Command::new(std::env::current_exe()?)
        .args(args)
        .env("VEIL_GPU", &req.gpu)
        .exec();
    Err(err).context("re-executing veil")
}

fn usage() -> ! {
    eprintln!(
        "Usage:
  veil [OPTIONS] -- <cmd> [args...]

OPTIONS:
  --out <path>              Output file for encoded stream (optional)
  --size <WxH>              Resolution (default: 1280x720)
  --fps <n>                 Framerate (default: 60)
  --codec <auto|h264|h265|av1>
                            Video codec (default: auto, the best codec every
                            viewer decodes; av1 is experimental)
  --gop-size <n>            IDR/keyframe interval in frames (default: 30; 0 = encoder default of 60)
  --in-flight <n>           Encoder output buffers in flight (default: 2; 1 = lowest latency)
  --preset <0-9>            Quality preset; 0-6 = VBR (higher is more bitrate), 7-9 = constant QP (default: 0)
  --h264-profile <p>        H.264 profile: baseline, main, high (default: baseline)
  --h264-level <l>          H.264 level: 4.1, 5.1, 5.2 etc (default: 5.1)
  --xwayland                Enable XWayland (default)
  --no-xwayland             Disable XWayland
  --force-1x                Force 1x UI scaling
  --seconds <n>             Max duration in seconds (for testing)
  --transport               Enable QUIC/WebTransport streaming server
  --transport-addr <addr>   WebTransport bind address (default: 0.0.0.0:7801; cert-hash HTTP on port+1)

Examples:
  # File output
  cargo run -- --seconds 5 --out out.h264 -- vkcube

  # WebTransport streaming
  cargo run -- --transport -- vkcube
  cargo run -- --transport --transport-addr 0.0.0.0:7801 -- steam

  # Streaming + file recording
  cargo run -- --transport --out recording.h264 -- glxgears
"
    );
    std::process::exit(2);
}

fn parse_size(s: &str) -> anyhow::Result<(u32, u32)> {
    let mut it = s.split('x');
    let w = it.next().ok_or_else(|| anyhow::anyhow!("missing width"))?
        .parse::<u32>().context("parsing width")?;
    let h = it.next().ok_or_else(|| anyhow::anyhow!("missing height"))?
        .parse::<u32>().context("parsing height")?;
    if it.next().is_some() {
        bail!("invalid --size {s:?}, expected WxH");
    }
    Ok((w, h))
}

fn parse_codec(s: &str) -> anyhow::Result<VideoCodec> {
    match s {
        "h264" | "H264" => Ok(VideoCodec::H264),
        "h265" | "hevc" | "H265" | "HEVC" => Ok(VideoCodec::H265),
        "av1" | "AV1" => Ok(VideoCodec::Av1),
        other => bail!("unknown codec {other:?} (expected h264, h265 or av1)"),
    }
}

fn main() -> anyhow::Result<()> {
    let mut out_path: Option<PathBuf> = None;
    let mut width: u32 = 1280;
    let mut height: u32 = 720;
    let mut fps: u32 = 60;
    let mut codec = VideoCodec::H264;
    // Without --codec, pick the codec from what viewers can decode.
    let mut auto_codec = true;
    let mut gop_size: u32 = 30;
    let mut in_flight_frames: u32 = 0;
    let mut preset: u32 = 0;
    let mut xwayland = true;
    let mut force_1x_scale = false;
    let mut seconds: Option<u64> = None;
    let mut h264_profile_idc: u8 = 66;  // Baseline
    let mut h264_level_idc: u8 = 51;   // 5.1
    let mut enable_transport = false;
    let mut transport_addr: SocketAddr = "0.0.0.0:7801".parse().unwrap();

    let mut argv = std::env::args_os().skip(1).collect::<Vec<_>>();
    let cmd: Vec<OsString>;

    let dashdash_pos = argv.iter().position(|a| a == "--");
    if let Some(pos) = dashdash_pos {
        cmd = argv.drain(pos + 1..).collect();
        argv.truncate(pos);
    } else {
        cmd = argv.clone();
        argv.clear();
    }

    let mut i = 0;
    while i < argv.len() {
        let a = argv[i].to_string_lossy().to_string();
        match a.as_str() {
            "--out" => {
                i += 1;
                let v = argv.get(i).cloned().unwrap_or_else(|| usage());
                out_path = Some(PathBuf::from(v));
            }
            "--size" => {
                i += 1;
                let v = argv.get(i).cloned().unwrap_or_else(|| usage());
                let (w, h) = parse_size(&v.to_string_lossy())?;
                width = w;
                height = h;
            }
            "--fps" => {
                i += 1;
                let v = argv.get(i).cloned().unwrap_or_else(|| usage());
                fps = v.to_string_lossy().parse::<u32>().context("parsing --fps")?;
            }
            "--codec" => {
                i += 1;
                let v = argv.get(i).cloned().unwrap_or_else(|| usage());
                let v = v.to_string_lossy();
                auto_codec = v.eq_ignore_ascii_case("auto");
                codec = if auto_codec { VideoCodec::H264 } else { parse_codec(&v)? };
            }
            "--gop-size" => {
                i += 1;
                let v = argv.get(i).cloned().unwrap_or_else(|| usage());
                gop_size = v.to_string_lossy().parse::<u32>().context("parsing --gop-size")?;
            }
            "--in-flight" => {
                i += 1;
                let v = argv.get(i).cloned().unwrap_or_else(|| usage());
                in_flight_frames = v.to_string_lossy().parse::<u32>().context("parsing --in-flight")?;
            }
            "--preset" => {
                i += 1;
                let v = argv.get(i).cloned().unwrap_or_else(|| usage());
                preset = v.to_string_lossy().parse::<u32>().context("parsing --preset")?;
                if preset > 9 {
                    bail!("--preset must be 0-9");
                }
            }
            "--xwayland" => xwayland = true,
            "--no-xwayland" => xwayland = false,
            "--force-1x" => force_1x_scale = true,
            "--seconds" => {
                i += 1;
                let v = argv.get(i).cloned().unwrap_or_else(|| usage());
                seconds = Some(
                    v.to_string_lossy().parse::<u64>().context("parsing --seconds")?,
                );
            }
            "--h264-profile" => {
                i += 1;
                let v = argv.get(i).cloned().unwrap_or_else(|| usage());
                h264_profile_idc = match v.to_string_lossy().as_ref() {
                    "baseline" => 66,
                    "main"     => 77,
                    "high"     => 100,
                    other => { eprintln!("Unknown --h264-profile: {other}"); usage() }
                };
            }
            "--h264-level" => {
                i += 1;
                let v = argv.get(i).cloned().unwrap_or_else(|| usage());
                let s = v.to_string_lossy();
                // Accept "5.1" or "51" formats
                let normalized = s.replace('.', "");
                h264_level_idc = normalized.parse::<u8>()
                    .context("parsing --h264-level")?;
            }
            "--transport" => enable_transport = true,
            "--transport-addr" => {
                i += 1;
                let v = argv.get(i).cloned().unwrap_or_else(|| usage());
                transport_addr = v
                    .to_string_lossy()
                    .parse::<SocketAddr>()
                    .context("parsing --transport-addr")?;
            }
            "-h" | "--help" => usage(),
            _ => {
                eprintln!("Unknown argument: {a}");
                usage();
            }
        }
        i += 1;
    }

    if cmd.is_empty() {
        usage();
    }

    run(RunOptions {
        command: cmd,
        output_path: out_path,
        width,
        height,
        framerate: fps,
        codec,
        auto_codec: auto_codec && enable_transport,
        preset,
        profile: VideoProfile::Hd,
        gop_size,
        in_flight_frames,
        h264_profile_idc,
        h264_level_idc,
        xwayland,
        force_1x_scale,
        max_duration: seconds.map(std::time::Duration::from_secs),
        enable_transport,
        transport_addr,
    })
}
