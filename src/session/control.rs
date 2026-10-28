use crate::{
    codec::VideoCodec,
    color::VideoProfile,
    pixel_scale::PixelScale,
    session::compositor::{self, ButtonState},
};

/// Info about one mapped toplevel window, sent to the browser for the switcher UI.
#[derive(Debug, Clone)]
pub struct WindowInfo {
    /// Stable per-session numeric ID used to request focus via FocusWindow.
    pub id: u32,
    pub title: String,
    pub app_id: String,
    /// True for the surface that currently has keyboard focus.
    pub active: bool,
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DisplayParams {
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub ui_scale: PixelScale,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct VideoStreamParams {
    pub width: u32,
    pub height: u32,
    pub codec: VideoCodec,
    pub preset: u32,
    pub profile: VideoProfile,
    pub framerate: u32,
    /// IDR/keyframe interval in frames. Smaller = faster seeking/recovery.
    /// Default: 60 (1 second at 60 fps). Use 0 to keep encoder default.
    pub gop_size: u32,
    /// Number of output frame buffers in flight. 1 = lowest latency, 2 = default.
    pub in_flight_frames: u32,
    /// H.264 profile IDC (66=Baseline, 77=Main, 100=High). Ignored for H.265.
    pub h264_profile_idc: u8,
    /// H.264 level × 10 as in the SPS (e.g. 41=4.1, 51=5.1, 52=5.2). Ignored for H.265.
    pub h264_level_idc: u8,
}

/// Audio is currently not part of the minimal “compositor + Vulkan video encode” runner.
/// Keep the type around for now to avoid rippling changes through the compositor/input
/// code paths; it can be removed once audio is fully excised.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct AudioStreamParams {
    pub sample_rate: u32,
    pub channels: u32,
}

#[allow(dead_code)]

pub enum ControlMessage {
    Stop,
    /// Start a local (in-process) stream of encoded video frames.
    StartStream {
        video_params: VideoStreamParams,
        ready: oneshot::Sender<()>,
    },
    /// Stop streaming frames (keeps the compositor alive while the app runs).
    StopStream,
    RefreshVideo,
    /// Cap the video bitrate at what the slowest viewer's network carries
    /// (bits/s), or lift the cap. Applied live, without a keyframe.
    SetBitrateLimit(Option<u64>),
    UpdateDisplayParams(DisplayParams),
    KeyboardInput {
        key_code: u32,
        state: compositor::KeyState,
        char: Option<char>,
    },
    PointerEntered,
    PointerLeft,
    PointerMotion(f64, f64),
    RelativePointerMotion(f64, f64),
    PointerInput {
        x: f64,
        y: f64,
        button_code: u32,
        state: ButtonState,
    },
    PointerAxis(f64, f64),
    PointerAxisDiscrete(f64, f64),
    /// A pad was plugged in; the layout is `None` from older clients.
    GamepadAvailable(u64, Option<super::GamepadLayout>),
    GamepadUnavailable(u64),
    /// Switch keyboard focus to the window with the given ID.
    FocusWindow(u32),
    GamepadAxis {
        id: u64,
        axis_code: u32,
        value: f64,
    },
    GamepadTrigger {
        id: u64,
        trigger_code: u32,
        value: f64,
    },
    GamepadInput {
        id: u64,
        button_code: u32,
        state: ButtonState,
    },
    /// A complete gamepad snapshot. `seq` increases per pad; older or
    /// duplicate snapshots (reordered or resent datagrams) are ignored.
    GamepadState {
        id: u64,
        seq: u32,
        state: GamepadState,
        /// Which pad to emulate; `None` from older clients. A change
        /// replaces the virtual device.
        layout: Option<super::GamepadLayout>,
    },
    /// Latency ping — the u64 is the client-supplied timestamp to echo back.
    Ping(u64),
    /// Ask the compositor to immediately re-broadcast the current window list.
    /// Sent by the transport when a new client connects.
    RefreshWindowList,
    /// Ask the reactor to re-broadcast the current [`StreamInfo`]. Sent by the
    /// transport when a new client connects.
    RefreshStreamInfo,
    /// Change encoding settings while streaming; unset fields keep their
    /// current value. Rebuilds the encode pipeline (and resizes the display
    /// if the size or framerate changed).
    SetStreamParams(StreamParamsRequest),
    /// Restart Veil (and the app) on another GPU, matched like `VEIL_GPU`.
    SwitchGpu(String),
}

/// Set by the reactor when a client asks to switch GPUs; `main` re-executes
/// Veil with it once the reactor has shut down.
pub static RESTART: std::sync::Mutex<Option<RestartRequest>> = std::sync::Mutex::new(None);

#[derive(Debug, Clone)]
pub struct RestartRequest {
    pub gpu: String,
    /// The settings to carry over, if streaming had started.
    pub video_params: Option<VideoStreamParams>,
}

/// A client's request to change encoding settings. See
/// [`ControlMessage::SetStreamParams`].
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct StreamParamsRequest {
    pub codec: Option<VideoCodec>,
    /// Pick the codec from what viewers decode (handled by the transport,
    /// which then sends `codec`).
    pub auto_codec: Option<bool>,
    pub preset: Option<u32>,
    pub framerate: Option<u32>,
    pub gop_size: Option<u32>,
    pub in_flight_frames: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// What the server is currently doing, for display in the client.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamInfo {
    pub gpu: String,
    pub gpu_driver: String,
    /// Every GPU Veil could encode on, by name; `gpu` is one of these.
    pub gpus: Vec<String>,
    pub codec: VideoCodec,
    /// The codecs this GPU can encode.
    pub codecs: Vec<VideoCodec>,
    pub codec_profile: String,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub gop_size: u32,
    pub in_flight_frames: u32,
    pub preset: u32,
    pub rate_control: String,
    pub temporal_layers: u32,
    /// How converted frames reach the encoder: "direct" (storage writes into
    /// the encode image) or "copy" (per-plane images copied in).
    pub write_path: &'static str,
}

/// One gamepad's full state, in the W3C "standard" layout.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GamepadState {
    /// Bit `i` set = standard-mapping button `i` pressed (0 = A … 16 = Guide).
    pub buttons: u32,
    /// 0-255, like a real Xbox 360 pad.
    pub left_trigger: u8,
    pub right_trigger: u8,
    /// Left X, left Y, right X, right Y; -32767..32767, +Y is down.
    pub axes: [i16; 4],
}

/// The server side of one client gamepad.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GamepadStatus {
    /// Server-wide pad ID (viewer and index combined).
    pub id: u64,
    /// The viewer the pad belongs to, and its index in that browser.
    pub viewer: u32,
    pub index: u8,
    pub name: String,
    /// The pad type the client asked for.
    pub layout: super::GamepadLayout,
    /// What the virtual device presents itself as.
    pub emulates: &'static str,
    /// The virtual device's evdev node, if it was created.
    pub device: Option<String>,
    /// Why it couldn't be created, if it wasn't.
    pub error: Option<String>,
    /// Why it isn't what the client asked for, if it isn't (e.g. a DualSense
    /// that fell back to an Xbox pad).
    pub notice: Option<String>,
    /// Input events received so far.
    pub events: u64,
}

/// The compositor's view of what's on screen, for diagnosing windows that
/// don't show up.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CompositorState {
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub wayland_socket: String,
    /// The X11 DISPLAY apps get, if Xwayland is running.
    pub x11_display: Option<String>,
    /// The app Veil launched.
    pub app_command: String,
    pub app_pid: Option<u32>,
    /// Set once the launched process has exited (launchers like Steam exit
    /// early and leave their children running).
    pub app_exit: Option<String>,
    /// Wayland clients connected to the compositor.
    pub clients: usize,
    /// Surfaces with a role, top of the stack first, then unmapped ones.
    pub surfaces: Vec<SurfaceState>,
    /// Surfaces without a role (not windows), just counted.
    pub other_surfaces: usize,
    pub x11_windows: Vec<X11WindowState>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SurfaceState {
    pub key: String,
    /// The ID the browser uses to raise it, if mapped.
    pub window_id: Option<u32>,
    /// "wayland" (xdg_toplevel), "x11" (Xwayland) or "cursor".
    pub kind: &'static str,
    pub title: String,
    pub app_id: String,
    pub x11_window: Option<u32>,
    /// In the stack (has a configuration and a buffer).
    pub mapped: bool,
    /// Position in the stack, 0 = top.
    pub stack_index: Option<usize>,
    /// Drawn into the video.
    pub composited: bool,
    pub active: bool,
    pub fullscreen: bool,
    pub configured: Option<[u32; 2]>,
    pub buffer: Option<BufferState>,
    pub awaiting_configure_ack: bool,
    pub commits: u64,
    pub commits_per_sec: f64,
    /// Why it isn't visible (or looks wrong), in plain words.
    pub problem: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BufferState {
    pub width: u32,
    pub height: u32,
    /// "shm" (CPU copy) or "dmabuf" (GPU).
    pub kind: &'static str,
    pub format: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct X11WindowState {
    pub id: u32,
    pub title: String,
    pub class: String,
    pub geometry: [u32; 4],
    pub override_redirect: bool,
    pub mapped: bool,
    pub fullscreen: bool,
    /// Linked to a Wayland surface (via the Xwayland serial).
    pub has_surface: bool,
    pub problem: Option<String>,
}

/// One connected client, without identifying details.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ViewerInfo {
    pub id: u32,
    pub connected_secs: u64,
    /// "lan" or "internet".
    pub network: &'static str,
    /// Sent input in the last few seconds.
    pub active: bool,
}

/// Encoder statistics over the last reporting window (about a second).
#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct EncoderStats {
    pub window_ms: u64,
    pub frames: u64,
    pub keyframes: u64,
    pub bytes: u64,
    /// Time from submitting a frame to the encoder to its bitstream being
    /// ready, in milliseconds.
    pub encode_ms_avg: f64,
    pub encode_ms_max: f64,
    /// The encoder's current target bitrate, and the preset's (the ceiling a
    /// network limit lowers it from), for VBR.
    pub target_bitrate: Option<u64>,
    pub preset_bitrate: Option<u64>,
    /// The network limit in effect, if any.
    pub bitrate_limit: Option<u64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum SessionEvent {
    DisplayParamsChanged {
        params: DisplayParams,
        reattach: bool,
    },
    VideoFrame {
        seq: u64,
        is_keyframe: bool,
        frame: bytes::Bytes,
    },
    AudioFrame {
        seq: u64,
        frame: bytes::Bytes,
    },
    CursorUpdate {
        image: Option<bytes::Bytes>,
        icon: Option<cursor_icon::CursorIcon>,
        hotspot_x: u32,
        hotspot_y: u32,
    },
    PointerLocked(f64, f64),
    PointerReleased,
    /// Echo of a client ping — carries the same timestamp the client sent.
    Pong(u64),
    /// Current list of mapped toplevel windows (sent to browser for the switcher UI).
    WindowList(Vec<WindowInfo>),
    /// The encode pipeline was (re)created, or a client asked for the info.
    StreamInfo(std::sync::Arc<StreamInfo>),
    EncoderStats(EncoderStats),
    /// The server's virtual gamepads, sent when they change, when a client
    /// connects, and about once a second while any exist.
    GamepadStatus(Vec<GamepadStatus>),
    /// A game asked for rumble on a virtual pad. Magnitudes are 0-65535; a
    /// zero duration with zero magnitudes means stop.
    GamepadRumble {
        id: u64,
        strong: u16,
        weak: u16,
        duration_ms: u16,
    },
    /// About once a second while streaming, and when a client connects.
    CompositorState(std::sync::Arc<CompositorState>),
    /// Connected clients; sent by the transport on connect/disconnect and
    /// every couple of seconds while anyone is connected.
    Viewers(std::sync::Arc<Vec<ViewerInfo>>),
    Shutdown,
}
