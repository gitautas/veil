use std::{
    collections::BTreeMap,
    env,
    ffi::{OsStr, OsString},
    fs::File,
    io::{BufRead, BufReader},
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
    time,
};

use anyhow::{Context as _, bail};

use tracing::{debug, info, trace, trace_span, warn};

use super::{
    AppConfig, GamepadLayout, SessionEvent, SessionHandle, audio,
    compositor::{self, Compositor, xwayland},
    control::{
        AudioStreamParams, CompositorState, ControlMessage, DisplayParams, RESTART, RestartRequest,
        StreamInfo, StreamParamsRequest, VideoStreamParams,
    },
    input, video,
};
use crate::{
    codec::VideoCodec,
    pixel_scale::PixelScale,
    runtime::{ProcessHandle, Runtime},
    vulkan::{Vendor, VkContext},
    waking_sender::WakingSender,
};

const READY_TIMEOUT: std::time::Duration = time::Duration::from_secs(120);

const DISPLAY: mio::Token = mio::Token(0);
const ACCEPT: mio::Token = mio::Token(1);
const CHILD: mio::Token = mio::Token(2);
const WAKER: mio::Token = mio::Token(3);
#[allow(dead_code)]
const TIMER: mio::Token = mio::Token(4);

const XDISPLAY: mio::Token = mio::Token(10);
const XWAYLAND: mio::Token = mio::Token(11);
const XWAYLAND_READY: mio::Token = mio::Token(12);

pub struct Reactor {
    poll: mio::Poll,
    waker: Arc<mio::Waker>,

    compositor: Compositor,
    session_handle: SessionHandle,

    listening_socket: wayland_server::ListeningSocket,
    wayland_display: wayland_server::Display<Compositor>,

    app_config: AppConfig,
    child: ProcessHandle,
    /// How the launched process exited, once it has.
    child_exit: Option<String>,
    wayland_socket: String,
    child_debug_log: Option<File>,

    display_params: DisplayParams,
    new_display_params: Option<DisplayParams>,

    audio_pipeline: audio::EncodePipeline,
    video_pipeline: Option<video::EncodePipeline>,
    new_video_stream_params: Option<VideoStreamParams>,
    /// The parameters `video_pipeline` was created with.
    video_params: Option<VideoStreamParams>,
    stream_info: Option<Arc<StreamInfo>>,
    last_stats_at: time::Instant,

    input_manager: input::InputDeviceManager,
    gamepads: BTreeMap<u64, input::GamepadHandle>,

    xwayland: Option<xwayland::XWayland>,
    xwayland_debug_log: Option<File>,

    streaming_active: bool,

    ready_once: Option<mpsc::Sender<WakingSender<ControlMessage>>>,
    shutting_down: bool,
    // Set to true once we've called child.wait() so we don't double-wait if
    // both the pidfd and the pipe fire close events (launcher-pattern apps like
    // Steam fork a grandchild and exit with 0 before Wayland surfaces appear).
    child_reaped: bool,

    vk: Arc<VkContext>,
}

impl Reactor {
    pub fn run(
        vk: Arc<VkContext>,
        app_config: AppConfig,
        display_params: DisplayParams,
        permanent_gamepads: Vec<(u64, GamepadLayout)>,
        bug_report_dir: Option<PathBuf>,
        ready_send: mpsc::Sender<WakingSender<ControlMessage>>,
        events_tx: tokio::sync::broadcast::Sender<crate::session::control::SessionEvent>,
    ) -> anyhow::Result<()> {
        // Initialize the epoch for timestamping
        let _ = super::EPOCH.set(time::Instant::now());

        let mut display = wayland_server::Display::new().context("failed to create display")?;

        let ui_scale = if app_config.force_1x_scale {
            PixelScale::ONE
        } else {
            display_params.ui_scale
        };

        trace!(
            %ui_scale,
            width = display_params.width,
            height = display_params.height,
            "configuring virtual display"
        );

        // Create wayland globals.
        let dh = display.handle();
        compositor::create_globals(&dh);

        let mut runtime =
            Runtime::new(app_config.command.clone()).context("initializing container")?;

        for (k, v) in &app_config.env {
            runtime.set_env(k, v);
        }

        let poll = mio::Poll::new()?;
        let waker = Arc::new(mio::Waker::new(poll.registry(), WAKER)?);
        let handle = SessionHandle::new(waker.clone(), events_tx);

        let display_fd = display.backend().poll_fd().as_raw_fd();
        poll.registry().register(
            &mut mio::unix::SourceFd(&display_fd),
            DISPLAY,
            mio::Interest::READABLE,
        )?;

        // Bind the Wayland listening socket in the *host's* XDG_RUNTIME_DIR so
        // it remains reachable even if Steam (or any other launcher) resets
        // XDG_RUNTIME_DIR to /run/user/<uid>.  A path under our private temp
        // dir would disappear from the client's view the moment it resets the
        // env var.
        let host_xdg_runtime_dir = env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| runtime.extern_run_path().to_path_buf());
        let socket_name = gen_socket_name();
        let socket_path = host_xdg_runtime_dir.join(&socket_name);
        let listening_socket = wayland_server::ListeningSocket::bind_absolute(socket_path.clone())?;
        info!(?socket_path, "bound wayland socket");

        let listener_fd = listening_socket.as_raw_fd();
        poll.registry().register(
            &mut mio::unix::SourceFd(&listener_fd),
            ACCEPT,
            mio::Interest::READABLE,
        )?;

        // Set up the pulse audio server.
        let audio_pipeline = audio::EncodePipeline::new(handle.clone(), runtime.extern_run_path())?;

        // Set up compositor state.
        let compositor = compositor::Compositor::new(
            vk.clone(),
            handle.clone(),
            DisplayParams {
                ui_scale, // Overridden by force_1x_scale.
                ..display_params
            },
        )?;

        // Set up input emulation (this is just for gamepads).
        let mut input_manager = input::InputDeviceManager::new()?;
        let mut gamepads = BTreeMap::new();

        for (pad_id, layout) in permanent_gamepads {
            let dev = input_manager.plug_gamepad(pad_id, layout, true)?;
            gamepads.insert(pad_id, dev);
        }

        // Spawn Xwayland, if we're using it.
        let (xwayland, xwayland_recv, xwayland_debug_log) = if app_config.xwayland {
            let mut xwayland_debug_log = if let Some(bug_report_dir) = bug_report_dir.as_ref() {
                let path = bug_report_dir.join("xwayland.log");
                Some(std::fs::File::create(path).context("failed to create xwayland logfile")?)
            } else {
                None
            };

            let (output_send, mut output_recv) = mio::unix::pipe::new()?;
            let mut xwayland = match xwayland::XWayland::spawn(
                &mut display.handle(),
                runtime.extern_run_path(),
                output_send,
            ) {
                Ok(xw) => xw,
                Err(e) => {
                    // Make sure we save any errors.
                    dump_child_output(
                        &mut BufReader::new(&mut output_recv),
                        &mut xwayland_debug_log,
                    );

                    return Err(e).context("spawning Xwayland");
                }
            };

            // Xwayland writes to this pipe when it's ready.
            poll.registry().register(
                &mut xwayland.displayfd_recv,
                XWAYLAND_READY,
                mio::Interest::READABLE,
            )?;

            // Stderr/stdout of the xwayland process.
            poll.registry()
                .register(&mut output_recv, XWAYLAND, mio::Interest::READABLE)?;

            (Some(xwayland), Some(output_recv), xwayland_debug_log)
        } else {
            (None, None, None)
        };

        // Spawn the client with a pipe as stdout/stderr.
        let (pipe_send, mut pipe_recv) = mio::unix::pipe::new()?;
        runtime.set_stdout(&pipe_send)?;
        runtime.set_stderr(&pipe_send)?;
        drop(pipe_send);

        // Set the wayland socket and X11 sockets.
        //
        // Because the socket lives in the host's XDG_RUNTIME_DIR we can now
        // pass the *relative* socket name.  libwayland-client will look it up
        // as $XDG_RUNTIME_DIR/<name>, and since Steam resets XDG_RUNTIME_DIR
        // to /run/user/<uid> (the same dir our socket is in), the lookup works.
        runtime.set_env("WAYLAND_DISPLAY", &socket_name);

        // Hint toolkits (SDL, GTK, Qt) that this is a Wayland session.
        runtime.set_env("XDG_SESSION_TYPE", "wayland");

        // Make the app render on the GPU we composite and encode on. Importing
        // its buffers from another vendor's GPU crashes the client. Mesa's
        // device-select layer applies to every Vulkan driver; the trailing `!`
        // hides the other devices, so apps that prefer some other GPU (e.g.
        // DXVK picking a discrete card) can't.
        let dev = &vk.device_info;
        let pci_id = format!("{:04x}:{:04x}", dev.vendor_id, dev.device_id);
        runtime.set_env("MESA_VK_DEVICE_SELECT", format!("{pci_id}!"));
        // OpenGL: DRI_PRIME for Mesa, PRIME render offload for NVIDIA's GLX/EGL.
        runtime.set_env("DRI_PRIME", &pci_id);
        if dev.device_vendor == Vendor::Nvidia {
            runtime.set_env("__NV_PRIME_RENDER_OFFLOAD", "1");
            runtime.set_env("__GLX_VENDOR_LIBRARY_NAME", "nvidia");
            runtime.set_env("__VK_LAYER_NV_optimus", "NVIDIA_only");
        }

        // PulseAudio: set an explicit server path so apps find our built-in
        // PulseAudio stub regardless of what XDG_RUNTIME_DIR they end up with.
        let pulse_socket = runtime.extern_run_path().join("pulse/native");
        runtime.set_env("PULSE_SERVER", format!("unix:{}", pulse_socket.display()));
        if let Some(xwayland) = &xwayland {
            xwayland.prepare_socket(&mut runtime);
        }

        // Shadow pipewire, just in case.
        runtime.set_env("PIPEWIRE_REMOTE", "(null)");

        // Propagate essential host env needed by desktop apps (e.g., Steam).
        for key in [
            "PATH",
            "HOME",
            "USER",
            "LANG",
            "LC_ALL",
            "DBUS_SESSION_BUS_ADDRESS",
            "XAUTHORITY",
        ] {
            if let Ok(val) = env::var(key) {
                runtime.set_env(key, val);
            }
        }

        let child = match runtime.spawn() {
            Ok(ch) => ch,
            Err(e) => {
                // Make sure we pump the child stdio and catch any container-related
                // error.
                let mut debug_log = bug_report_dir
                    .as_ref()
                    .and_then(|dir| std::fs::File::create(dir.join("child.log")).ok());
                let mut child_output = BufReader::new(&mut pipe_recv);
                dump_child_output(&mut child_output, &mut debug_log);
                return Err(e).context("starting application container");
            }
        };

        poll.registry().register(
            &mut mio::unix::SourceFd(&child.pidfd().as_raw_fd()),
            CHILD,
            mio::Interest::READABLE,
        )?;

        poll.registry()
            .register(&mut pipe_recv, CHILD, mio::Interest::READABLE)?;

        // Use `glxinfo` and `eglinfo` to generate more debugging help.
        if let Some(bug_report_dir) = bug_report_dir.as_ref() {
            let p = bug_report_dir.to_owned();
            let wayland_socket = socket_path.clone().into_os_string();
            let x11_socket = xwayland
                .as_ref()
                .map(|x| x.display_socket.inner_path().clone());

            std::thread::spawn(move || {
                save_glxinfo_eglinfo(
                    &p,
                    &wayland_socket,
                    x11_socket.as_ref().map(|p| p.as_os_str()),
                );
            });
        }

        // If bug report mode is enabled, save the stdout/stderr of the child to
        // a logfile.
        let child_debug_log = if let Some(bug_report_dir) = bug_report_dir.as_ref() {
            let path = bug_report_dir.join(format!("child-{}.log", child.pid().as_raw_nonzero()));
            Some(std::fs::File::create(path).context("failed to create child logfile")?)
        } else {
            None
        };

        // Framerate timer (simulates vblank).
        //
        // NOTE: We avoid `mio-timerfd` here because it pulls in a different `mio`
        // version than the rest of the project, which breaks `mio::event::Source`.
        // The reactor already uses timeouts in `poll()`, so we can drive frames
        // using `poll` timeouts instead of registering an fd.

        let mut reactor = Self {
            poll,
            waker,

            wayland_display: display,
            compositor,

            session_handle: handle,
            listening_socket,

            app_config,
            child,
            child_exit: None,
            wayland_socket: socket_name.to_string_lossy().into_owned(),
            child_debug_log,

            display_params,
            new_display_params: None,

            audio_pipeline,
            video_pipeline: None,
            new_video_stream_params: None,
            video_params: None,
            stream_info: None,
            last_stats_at: time::Instant::now(),

            input_manager,
            gamepads,

            xwayland,
            xwayland_debug_log,

            streaming_active: false,

            ready_once: Some(ready_send),
            shutting_down: false,
            child_reaped: false,

            vk,
        };

        reactor.main_loop(pipe_recv, xwayland_recv)
    }

    fn main_loop(
        &mut self,
        mut child_pipe: mio::unix::pipe::Receiver,
        mut xwayland_pipe: Option<mio::unix::pipe::Receiver>,
    ) -> Result<(), anyhow::Error> {
        let mut events = mio::Events::with_capacity(64);

        let (control_send, control_recv) = crossbeam_channel::unbounded();
        let control_send = WakingSender::new(self.waker.clone(), control_send);

        let start = time::Instant::now();
        let mut child_output = BufReader::new(&mut child_pipe);
        let mut xwayland_output = xwayland_pipe.as_mut().map(BufReader::new);

        info!("reactor: main_loop entered");

        let mut loop_iteration = 0u64;
        let mut next_frame_at = time::Instant::now();
        loop {
            loop_iteration += 1;
            if loop_iteration % 60 == 0 {
                trace!(
                    "reactor: loop iteration {}, streaming_active={}, video_pipeline={}, surfaces_ready={}",
                    loop_iteration,
                    self.streaming_active,
                    self.video_pipeline.is_some(),
                    self.compositor.surfaces_ready()
                );
            }
            // Drive frames via `poll` timeout instead of a registered timerfd.
            //
            // When streaming is active, we sleep until the next frame is due.
            // Client events wake us early, so frames are only produced once
            // the deadline passes (see below). When inactive, we wake
            // periodically to handle control messages and housekeeping.
            let poll_timeout = if self.active() {
                next_frame_at.saturating_duration_since(time::Instant::now())
            } else {
                time::Duration::from_secs(1)
            };

            trace_span!("poll").in_scope(|| self.poll.poll(&mut events, Some(poll_timeout)))?;

            for event in events.iter() {
                match event.token() {
                    ACCEPT => {
                        if let Some(client_stream) = self.listening_socket.accept()? {
                            let _client = self.wayland_display.handle().insert_client(
                                client_stream,
                                Arc::new(compositor::ClientState::default()),
                            )?;

                            debug!("client app connected");
                        }
                    }
                    CHILD if event.is_read_closed() => {
                        if self.child_reaped {
                            // Already reaped — the pipe fired again after we
                            // continued past a launcher-style status-0 exit.
                            // Nothing to do; wait for READY_TIMEOUT or Stop.
                        } else {
                            self.child_reaped = true;
                            let status = self.child.wait()?;
                            self.child_exit = Some(status.to_string());

                            if self.ready_once.is_some() {
                                if status.success() {
                                    // Launcher-style apps (e.g. Steam) fork the
                                    // real process and exit with status 0 before
                                    // any Wayland surface is committed.  Continue
                                    // running — the grandchild will connect to our
                                    // Wayland socket shortly.  READY_TIMEOUT still
                                    // applies if nothing ever connects.
                                    warn!("child exited with status 0 before surfaces were ready; \
                                           continuing (launcher pattern — e.g. Steam)");
                                } else {
                                    self.session_handle.kick_clients();
                                    bail!("client exited without doing anything (status: {status})");
                                }
                            } else {
                                self.session_handle.kick_clients();
                                info!(?status, "client exited after session");
                                return Ok(());
                            }
                        }
                    }
                    CHILD if event.is_readable() => {
                        dump_child_output(&mut child_output, &mut self.child_debug_log)
                    }
                    WAKER => loop {
                        match control_recv.try_recv() {
                            Ok(ControlMessage::SwitchGpu(ref gpu)) if !self.request_restart(gpu) => {}
                            Ok(ControlMessage::Stop | ControlMessage::SwitchGpu(_)) => {
                                info!(
                                    streaming_active = self.streaming_active,
                                    has_video_pipeline = self.video_pipeline.is_some(),
                                    "reactor: received ControlMessage::Stop"
                                );

                                self.session_handle.kick_clients();
                                self.shutting_down = true;

                                info!(
                                    "reactor: beginning bounded shutdown (TERM -> drain -> KILL)"
                                );

                                // Clean shutdown strategy:
                                // 1) Ask children to terminate (TERM)
                                // 2) Drain their output + wait a bounded time
                                // 3) Escalate to KILL if still alive
                                //
                                // NOTE: In our container-style runtime, TERM may be ignored (PID 1 behavior),
                                // but we still try it first to allow graceful teardown.
                                let stop_started = time::Instant::now();
                                let grace = time::Duration::from_millis(750);

                                // Best-effort: ask the main client to terminate.
                                match self.child.signal(rustix::process::Signal::TERM) {
                                    Ok(()) => debug!("reactor: sent TERM to client"),
                                    Err(e) => {
                                        warn!(error = ?e, "reactor: failed to send TERM to client")
                                    }
                                }

                                // Best-effort: ask Xwayland to terminate too (if present).
                                if let Some(xw) = self.xwayland.as_mut() {
                                    match xw.child.signal(rustix::process::Signal::TERM) {
                                        Ok(()) => debug!("reactor: sent TERM to Xwayland"),
                                        Err(e) => {
                                            warn!(error = ?e, "reactor: failed to send TERM to Xwayland")
                                        }
                                    }
                                }

                                // Drain any remaining child output while we wait briefly for exit.
                                // We intentionally do not block indefinitely here: the runner expects Stop
                                // to return promptly (bounded clean shutdown).
                                while stop_started.elapsed() < grace {
                                    // Drain pipes opportunistically.
                                    dump_child_output(&mut child_output, &mut self.child_debug_log);
                                    if let Some(ref mut xo) = xwayland_output {
                                        dump_child_output(xo, &mut self.xwayland_debug_log);
                                    }

                                    // Give the event loop a chance to observe EOF/read-closed events.
                                    // (We bound this to a short timeout.)
                                    let _ = self
                                        .poll
                                        .poll(&mut events, Some(time::Duration::from_millis(25)));
                                }

                                // Escalate: KILL anything still around.
                                match self.child.signal(rustix::process::Signal::KILL) {
                                    Ok(()) => debug!("reactor: sent KILL to client (escalation)"),
                                    Err(e) => {
                                        warn!(error = ?e, "reactor: failed to send KILL to client")
                                    }
                                }
                                if let Some(xw) = self.xwayland.as_mut() {
                                    match xw.child.signal(rustix::process::Signal::KILL) {
                                        Ok(()) => {
                                            debug!("reactor: sent KILL to Xwayland (escalation)")
                                        }
                                        Err(e) => {
                                            warn!(error = ?e, "reactor: failed to send KILL to Xwayland")
                                        }
                                    }
                                }

                                // Final short drain so logs/files get flushed.
                                let drain_deadline = time::Instant::now();
                                while drain_deadline.elapsed() < time::Duration::from_millis(150) {
                                    dump_child_output(&mut child_output, &mut self.child_debug_log);
                                    if let Some(ref mut xo) = xwayland_output {
                                        dump_child_output(xo, &mut self.xwayland_debug_log);
                                    }
                                    let _ = self
                                        .poll
                                        .poll(&mut events, Some(time::Duration::from_millis(25)));
                                }

                                info!("reactor: Stop handled; exiting reactor main loop");
                                // Return: bounded clean shutdown completed (best-effort).
                                return Ok(());
                            }
                            Ok(msg) => {
                                trace!("reactor: received control message (non-Stop)");
                                self.handle_control_message(msg)?
                            }
                            Err(crossbeam_channel::TryRecvError::Empty) => break,
                            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                                panic!("control channel disconnected")
                            }
                        }
                    },
                    DISPLAY => {
                        trace!("dispatching display");
                        self.wayland_display
                            .dispatch_clients(&mut self.compositor)
                            .context("failed to dispatch the wayland display")?;
                    }
                    XDISPLAY => {
                        trace!("dispatching xwm");
                        self.compositor
                            .dispatch_xwm()
                            .context("failed to dispatch the xwm")?;
                    }
                    XWAYLAND_READY => {
                        let xwayland = self.xwayland.as_mut().unwrap();
                        if let Some(socket) = xwayland.poll_ready()? {
                            self.poll
                                .registry()
                                .deregister(&mut xwayland.displayfd_recv)?;

                            // Setup the XWM connection to the Xwayland server.
                            let fd = self.compositor.insert_xwayland(socket)?;
                            self.poll.registry().register(
                                &mut mio::unix::SourceFd(&fd.as_raw_fd()),
                                XDISPLAY,
                                mio::Interest::READABLE,
                            )?;
                        }
                    }
                    XWAYLAND if event.is_read_closed() => {
                        self.xwayland.as_mut().unwrap().child.wait()?;
                    }
                    XWAYLAND if event.is_readable() => {
                        dump_child_output(
                            xwayland_output.as_mut().unwrap(),
                            &mut self.xwayland_debug_log,
                        );
                    }

                    _ => unreachable!(),
                }
            }

            if !self.shutting_down {
                self.idle()?;
            }

            // Check that we haven't timed out waiting for the client to start up.
            if self.ready_once.is_some() && self.compositor.surfaces_ready() {
                info!("reactor: surfaces are ready, sending control channel to runner");
                self.ready_once.take().unwrap().send(control_send.clone())?;
            } else if self.ready_once.is_some() && start.elapsed() > READY_TIMEOUT {
                self.child.signal(rustix::process::Signal::KILL)?;
                bail!("timed out waiting for client");
            }

            // If we're actively streaming, produce a frame once it's due. The
            // loop also wakes for every client commit and input event, which
            // would otherwise encode far above the display framerate.
            let now = time::Instant::now();
            let is_active = self.active();
            if is_active && now >= next_frame_at {
                let interval =
                    time::Duration::from_secs_f64(1.0 / self.display_params.framerate as f64);
                next_frame_at += interval;
                // If we fell more than a frame behind, don't try to catch up
                // with a burst of frames.
                if next_frame_at < now {
                    next_frame_at = now + interval;
                }

                // Check if we need to resize the virtual display.
                if let Some(new_params) = self.new_display_params.take() {
                    self.update_display_params(new_params)?;
                }

                self.frame()?;
            }
        }
    }

    fn idle(&mut self) -> anyhow::Result<()> {
        // Perform compositor upkeep.
        self.compositor.idle(self.active())?;

        // Send pending controller SYN_REPORT events, and forward any rumble
        // games asked for.
        let mut device_appeared = false;
        for (id, dev) in self.gamepads.iter_mut() {
            dev.frame();
            device_appeared |= dev.find_device();
            for rumble in dev.poll_rumble() {
                self.session_handle.dispatch(SessionEvent::GamepadRumble {
                    id: *id,
                    strong: rumble.strong,
                    weak: rumble.weak,
                    duration_ms: rumble.duration_ms,
                });
            }
        }

        if device_appeared {
            self.publish_gamepads();
        }

        // Flush events to the app.
        self.wayland_display.flush_clients()?;

        Ok(())
    }

    fn active(&self) -> bool {
        self.streaming_active
    }

    fn update_display_params(&mut self, params: DisplayParams) -> anyhow::Result<()> {
        let old = self.display_params;

        let old_ui_scale = self.display_params.ui_scale;
        let new_ui_scale = if self.app_config.force_1x_scale {
            PixelScale::ONE
        } else {
            params.ui_scale
        };

        let size_changed = old.width != params.width || old.height != params.height;
        let scale_changed = old_ui_scale != new_ui_scale;
        let framerate_changed = old.framerate != params.framerate;

        if size_changed || scale_changed || framerate_changed {
            debug!(
                old_width = old.width,
                new_width = params.width,
                old_height = old.height,
                new_height = params.height,
                old_framerate = old.framerate,
                new_framerate = params.framerate,
                old_ui_scale = %old_ui_scale,
                new_ui_scale = %new_ui_scale,
                "resizing output",
            );

            // Previously the project supported remote "attachments" that might need
            // to reattach on size/framerate changes. In the trimmed local runner,
            // we either keep streaming (scale-only changes) or stop streaming
            // when the stream becomes incompatible (size/framerate changes).
            let force_restart_stream = size_changed || framerate_changed;

            self.compositor.update_display_params(
                DisplayParams {
                    ui_scale: new_ui_scale,
                    ..params
                },
                // If we're forcing a restart, treat the output as inactive to help
                // clients behave well during mode transitions.
                !force_restart_stream,
            )?;

            self.session_handle
                .dispatch(SessionEvent::DisplayParamsChanged {
                    params,
                    reattach: force_restart_stream,
                });

            if force_restart_stream {
                // Stop streaming and reset pipelines. The runner can restart the
                // stream with new parameters if desired.
                self.streaming_active = false;
                self.audio_pipeline.stop_stream();

                self.video_pipeline = None;
                self.new_video_stream_params = None;
            }
        } else if params.ui_scale != old.ui_scale {
            // Synthesize a param change if we are forcing 1x scale.
            self.session_handle
                .dispatch(SessionEvent::DisplayParamsChanged {
                    params,
                    reattach: false,
                });
        }

        self.display_params = DisplayParams {
            ui_scale: new_ui_scale,
            ..params
        };

        Ok(())
    }

    fn frame(&mut self) -> anyhow::Result<()> {
        if !self.streaming_active {
            trace!("frame: streaming_active=false, skipping");
            return Ok(());
        }

        let surfaces_ready = self.compositor.surfaces_ready();
        if !surfaces_ready {
            static WARN_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let count = WARN_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if count % 60 == 0 {
                warn!("frame: surfaces not ready (warned {} times)", count + 1);
            }
            return Ok(());
        }

        if let Some(mut params) = self.new_video_stream_params.take() {
            // E.g. after switching to a GPU without AV1 encode.
            if !supports_codec(&self.vk.device_info, params.codec) {
                warn!(codec = ?params.codec, "codec not supported on this gpu; using h264");
                params.codec = VideoCodec::H264;
            }
            info!(
                "frame: creating video_pipeline with codec={:?} size={}x{} fps={}",
                params.codec, params.width, params.height, params.framerate
            );
            self.video_pipeline = Some(video::EncodePipeline::new(
                self.vk.clone(),
                self.session_handle.clone(),
                self.display_params,
                params,
            )?);
            info!("frame: video_pipeline created successfully");
            self.video_params = Some(params);
            self.publish_stream_info();

            // Audio runs alongside video; (re)start it whenever the stream does.
            if let Err(e) = self.audio_pipeline.restart_stream(AudioStreamParams {
                sample_rate: 48000,
                channels: 2,
            }) {
                warn!("failed to start audio: {e:#}");
            }
        }

        let Some(video_pipeline) = &mut self.video_pipeline else {
            static WARN_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let count = WARN_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if count % 60 == 0 {
                warn!("frame: video_pipeline is None (warned {} times)", count + 1);
            }
            return Ok(());
        };

        let stats_due = self.last_stats_at.elapsed() >= time::Duration::from_secs(1);
        if stats_due {
            self.last_stats_at = time::Instant::now();
            self.session_handle
                .dispatch(SessionEvent::EncoderStats(video_pipeline.take_stats()));
        }

        trace!("frame: compositing with video_pipeline");
        // Composite visible surfaces.
        self.compositor.composite_frame(video_pipeline)?;
        trace!("frame: composite_frame completed");

        // Render the cursor, if needed.
        self.compositor.render_cursor()?;

        // Gamepad event counts change continuously, so refresh them with the
        // encoder stats.
        if stats_due && !self.gamepads.is_empty() {
            self.publish_gamepads();
        }
        if stats_due {
            self.publish_compositor_state();
        }

        Ok(())
    }

    fn handle_control_message(&mut self, msg: ControlMessage) -> anyhow::Result<()> {
        if self.shutting_down {
            // We're about to shut down, so ignore all messages.
            return Ok(());
        }

        match msg {
            ControlMessage::StartStream { video_params, ready } => {
                self.streaming_active = true;
                self.new_video_stream_params = Some(video_params.clone());
                self.compositor.update_focus_and_visibility(true)?;

                info!(
                    "reactor: StartStream processed, streaming_active={}, codec={:?}, size={}x{}, fps={}",
                    self.streaming_active,
                    video_params.codec,
                    video_params.width,
                    video_params.height,
                    video_params.framerate
                );

                let _ = ready.send(());
                self.compositor.emit_window_list();
            }
            ControlMessage::StopStream => {
                self.streaming_active = false;
                self.video_pipeline = None;
                self.new_video_stream_params = None;
                self.audio_pipeline.stop_stream();
                self.compositor.update_focus_and_visibility(false)?;
            }
            ControlMessage::RefreshVideo => {
                if let Some(video) = &mut self.video_pipeline {
                    video.request_refresh();
                } else {
                    debug!("RefreshVideo received but video_pipeline not yet created; \
                            initial IDR from pipeline init will serve as the keyframe");
                }
            }
            ControlMessage::UpdateDisplayParams(params) => {
                // Updates once per render.
                self.new_display_params = Some(params);
            }
            ControlMessage::Ping(ts) => {
                self.session_handle.dispatch(SessionEvent::Pong(ts));
            }
            ControlMessage::KeyboardInput { .. }
            | ControlMessage::PointerInput { .. }
            | ControlMessage::PointerMotion { .. }
            | ControlMessage::RelativePointerMotion { .. }
            | ControlMessage::PointerAxis(_, _)
            | ControlMessage::PointerAxisDiscrete(_, _)
            | ControlMessage::PointerEntered
            | ControlMessage::PointerLeft => self.compositor.handle_input_event(msg),
            ControlMessage::FocusWindow(id) => {
                self.compositor.focus_window(id)?;
                self.publish_compositor_state();
            }
            ControlMessage::GamepadAvailable(id, layout) => {
                self.ensure_gamepad(id, layout);
                self.publish_gamepads();
            }
            ControlMessage::GamepadUnavailable(id) => {
                use std::collections::btree_map::Entry;
                match self.gamepads.entry(id) {
                    Entry::Occupied(v) if !v.get().permanent => {
                        v.remove();
                    }
                    _ => (),
                }
                self.publish_gamepads();
            }
            ControlMessage::GamepadAxis {
                id,
                axis_code,
                value,
            } => {
                self.ensure_gamepad(id, None);
                if let Some(gamepad) = self.gamepads.get_mut(&id) {
                    gamepad.axis(axis_code, value);
                }
            }
            ControlMessage::GamepadTrigger {
                id,
                trigger_code,
                value,
            } => {
                self.ensure_gamepad(id, None);
                if let Some(gamepad) = self.gamepads.get_mut(&id) {
                    gamepad.trigger(trigger_code, value);
                }
            }
            ControlMessage::GamepadInput {
                id,
                button_code,
                state,
            } => {
                self.ensure_gamepad(id, None);
                if let Some(gamepad) = self.gamepads.get_mut(&id) {
                    gamepad.input(button_code, state);
                }
            }
            ControlMessage::GamepadState { id, seq, state, layout } => {
                self.ensure_gamepad(id, layout);
                if let Some(gamepad) = self.gamepads.get_mut(&id) {
                    gamepad.apply_state(seq, state);
                }
            }
            ControlMessage::RefreshWindowList => {
                self.compositor.emit_window_list();
            }
            ControlMessage::RefreshStreamInfo => {
                if let Some(info) = &self.stream_info {
                    self.session_handle
                        .dispatch(SessionEvent::StreamInfo(info.clone()));
                }
                self.publish_gamepads();
                self.publish_compositor_state();
            }
            ControlMessage::SetStreamParams(req) => self.set_stream_params(req)?,
            // Stop and SwitchGpu are handled on the poll thread above.
            ControlMessage::Stop | ControlMessage::SwitchGpu(_) => unreachable!(),
        }

        Ok(())
    }
}

fn supports_codec(dev: &crate::vulkan::VkDeviceInfo, codec: VideoCodec) -> bool {
    match codec {
        VideoCodec::H264 => dev.supports_h264,
        VideoCodec::H265 => dev.supports_h265,
        VideoCodec::Av1 => dev.supports_av1,
    }
}

impl Reactor {
    /// Describe the current pipeline to clients.
    fn publish_stream_info(&mut self) {
        let (Some(params), Some(pipeline)) = (self.video_params, &self.video_pipeline) else {
            return;
        };
        let encoder = pipeline.describe();
        let dev = &self.vk.device_info;

        let codec_profile = match params.codec {
            VideoCodec::H264 => format!(
                "{} L{}.{}",
                match params.h264_profile_idc {
                    66 => "Baseline",
                    77 => "Main",
                    100 => "High",
                    _ => "Unknown",
                },
                params.h264_level_idc / 10,
                params.h264_level_idc % 10,
            ),
            VideoCodec::H265 => "Main".to_owned(),
            VideoCodec::Av1 => "Main".to_owned(),
        };

        let info = Arc::new(StreamInfo {
            gpu: dev.device_name.to_string_lossy().into_owned(),
            gpu_driver: dev.driver_version.to_string(),
            gpus: self.vk.eligible_gpus.clone(),
            codec: params.codec,
            codecs: [VideoCodec::H264, VideoCodec::H265, VideoCodec::Av1]
                .into_iter()
                .filter(|&codec| supports_codec(dev, codec))
                .collect(),
            codec_profile,
            width: params.width,
            height: params.height,
            framerate: params.framerate,
            gop_size: params.gop_size,
            in_flight_frames: params.in_flight_frames,
            preset: params.preset,
            rate_control: encoder.rate_control,
            temporal_layers: encoder.temporal_layers,
            write_path: if encoder.input_storage { "direct" } else { "copy" },
        });
        info!(?info, "stream info");
        self.stream_info = Some(info.clone());
        self.session_handle.dispatch(SessionEvent::StreamInfo(info));
    }

    /// Apply a client's settings change by rebuilding the encode pipeline.
    fn set_stream_params(&mut self, req: StreamParamsRequest) -> anyhow::Result<()> {
        let Some(current) = self.video_params else {
            warn!("SetStreamParams before the stream started; ignoring");
            return Ok(());
        };

        let dev = &self.vk.device_info;
        let mut params = current;
        if let Some(codec) = req.codec {
            if !supports_codec(dev, codec) {
                warn!(?codec, "codec not supported on this gpu; ignoring");
            } else {
                params.codec = codec;
            }
        }
        if let Some(preset) = req.preset {
            params.preset = preset.min(9);
        }
        if let Some(fps) = req.framerate {
            params.framerate = fps.clamp(1, 240);
        }
        if let Some(gop) = req.gop_size {
            params.gop_size = gop.min(600);
        }
        if let Some(n) = req.in_flight_frames {
            params.in_flight_frames = n.clamp(1, 8);
        }
        // Encoders want even dimensions for 4:2:0.
        if let Some(w) = req.width {
            params.width = (w.clamp(320, 4096)) & !1;
        }
        if let Some(h) = req.height {
            params.height = (h.clamp(240, 4096)) & !1;
        }

        if params == current {
            // Nothing changed; make sure the client is in sync anyway.
            return self.handle_control_message(ControlMessage::RefreshStreamInfo);
        }

        info!(?current, ?params, "changing stream params");

        let display = self.display_params;
        if params.width != display.width
            || params.height != display.height
            || params.framerate != display.framerate
        {
            // This stops the stream; it's restarted below.
            self.update_display_params(DisplayParams {
                width: params.width,
                height: params.height,
                framerate: params.framerate,
                ..display
            })?;
        }

        // Drop the old encoder before creating the new one, so they don't
        // compete for GPU memory and encode sessions.
        self.video_pipeline = None;
        self.streaming_active = true;
        self.new_video_stream_params = Some(params);
        self.compositor.update_focus_and_visibility(true)?;
        Ok(())
    }

    /// Record a request to restart on another GPU. Returns false (and logs) if
    /// `gpu` isn't one of the eligible GPUs.
    fn request_restart(&mut self, gpu: &str) -> bool {
        if !self.vk.eligible_gpus.iter().any(|name| name == gpu) {
            warn!(gpu, "SwitchGpu to unknown gpu; ignoring");
            return false;
        }
        info!(gpu, "restarting on another gpu");
        *RESTART.lock().unwrap() = Some(RestartRequest {
            gpu: gpu.to_owned(),
            video_params: self.video_params,
        });
        true
    }

    /// Plug a virtual gamepad on first input if it hasn't been registered yet.
    /// Make sure pad `id` exists, as `layout` if given. A pad of another
    /// layout is replaced (unplugged and plugged again).
    fn ensure_gamepad(&mut self, id: u64, layout: Option<GamepadLayout>) {
        let existing = self.gamepads.get(&id).map(|pad| (pad.layout, pad.permanent));
        let layout = match (existing, layout) {
            (Some(_), None) => return,
            (Some((current, _)), Some(wanted)) if current == wanted => return,
            (Some((_, true)), Some(_)) => return, // Permanent pads stay as they are.
            (Some((current, _)), Some(wanted)) => {
                info!(id, ?current, ?wanted, "replacing virtual gamepad");
                self.gamepads.remove(&id);
                wanted
            }
            (None, layout) => layout.unwrap_or_default(),
        };
        match self.input_manager.plug_gamepad(id, layout, false) {
            Ok(dev) => { self.gamepads.insert(id, dev); }
            Err(e)  => { warn!("failed to plug virtual gamepad {id}: {e:#}"); }
        }
        self.publish_gamepads();
    }

    /// Tell clients what the compositor is showing (and what it isn't).
    fn publish_compositor_state(&mut self) {
        let snap = self.compositor.inspect();
        let state = CompositorState {
            width: self.display_params.width,
            height: self.display_params.height,
            framerate: self.display_params.framerate,
            wayland_socket: self.wayland_socket.clone(),
            x11_display: self.xwayland.as_ref().map(|x| x.display_socket.display()),
            app_command: self
                .app_config
                .command
                .iter()
                .map(|a| a.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" "),
            app_pid: Some(self.child.pid().as_raw_nonzero().get() as u32),
            app_exit: self.child_exit.clone(),
            clients: snap.clients,
            surfaces: snap.surfaces,
            other_surfaces: snap.other_surfaces,
            x11_windows: snap.x11_windows,
        };
        self.session_handle
            .dispatch(SessionEvent::CompositorState(Arc::new(state)));
    }

    /// Tell clients about the virtual gamepads.
    fn publish_gamepads(&self) {
        let status = self
            .gamepads
            .iter()
            .map(|(id, pad)| pad.status(*id))
            .collect();
        self.session_handle.dispatch(SessionEvent::GamepadStatus(status));
    }
}

fn gen_socket_name() -> OsString {
    let id: u64 = rand::random();
    format!("veil-{}", id).into()
}

fn dump_child_output(pipe: &mut impl BufRead, debug_log: &mut Option<std::fs::File>) {
    let mut buf = String::new();

    loop {
        buf.clear();
        match pipe.read_line(&mut buf) {
            Ok(1..) => {
                if let Some(debug_log) = debug_log {
                    let _ = std::io::Write::write_all(debug_log, buf.as_bytes());
                }

                let buf = buf.trim();
                if !buf.is_empty() {
                    info!(target: "veil::session::child", "{}", buf);
                }
            }
            Ok(0) => break,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) => {
                debug!("child error: {:?}", e);
                break;
            }
        }
    }
}

fn save_glxinfo_eglinfo(
    bug_report_dir: impl AsRef<Path>,
    socket_name: &OsStr,
    x11_display: Option<&OsStr>,
) {
    use std::process::Command;

    if let Some(x11_display) = x11_display {
        match Command::new("glxinfo")
            .env_clear()
            .env("DISPLAY", x11_display)
            .output()
        {
            Ok(output) => {
                let _ = std::fs::write(bug_report_dir.as_ref().join("glxinfo.log"), output.stdout);
            }
            Err(e) => debug!("failed to run glxinfo: {:#}", e),
        }
    }

    match Command::new("eglinfo")
        .env_clear()
        .env("WAYLAND_DISPLAY", socket_name)
        .output()
    {
        Ok(output) => {
            let _ = std::fs::write(bug_report_dir.as_ref().join("eglinfo.log"), output.stdout);
        }
        Err(e) => debug!("failed to run eglinfo: {:#}", e),
    }
}
