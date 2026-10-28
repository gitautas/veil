use std::{collections::BTreeMap, ffi::OsString, sync::OnceLock, time};

pub mod compositor;
pub mod control;

mod audio;
mod handle;
mod dualsense;
pub(crate) mod uhid;
mod input;
pub(crate) mod reactor;
mod video;

pub use control::SessionEvent;
pub use handle::SessionHandle;
pub use input::{GamepadLayout, split_pad_id};
pub use reactor::Reactor;

/// Program start time for timestamping (used for PTS generation in the encoder sink).
pub static EPOCH: OnceLock<time::Instant> = OnceLock::new();

/// Minimal app configuration expected by the compositor reactor.
///
/// This intentionally replaces the old config/server model. The runner constructs this
/// directly from CLI args and environment.
#[derive(Clone, Debug)]
pub struct AppConfig {
    /// Command to exec (argv-style; first element is executable).
    pub command: Vec<OsString>,
    /// Environment variables passed to the child process.
    pub env: BTreeMap<OsString, OsString>,
    /// If true, spawn Xwayland and enable X11 clients.
    pub xwayland: bool,
    /// If true, override UI scale to 1.0 regardless of requested display params.
    pub force_1x_scale: bool,
}
