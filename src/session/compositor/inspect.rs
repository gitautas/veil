//! A snapshot of compositor state for the client UI, mostly to answer "why
//! can't I see this window?".

use std::collections::{HashMap, HashSet};
use std::time;

use slotmap::Key as _;
use wayland_server::Resource as _;

use crate::session::compositor::{
    Compositor,
    buffers::BufferBacking,
    surface::{SurfaceKey, SurfaceRole},
};
use crate::session::control::{BufferState, SurfaceState, X11WindowState};

/// Commit counts at the previous snapshot, for a per-second rate.
#[derive(Default)]
pub struct InspectState {
    at: Option<time::Instant>,
    commits: HashMap<SurfaceKey, u64>,
}

/// What [`Compositor::inspect`] returns; the reactor adds app and socket info.
pub struct Snapshot {
    pub clients: usize,
    pub surfaces: Vec<SurfaceState>,
    pub other_surfaces: usize,
    pub x11_windows: Vec<X11WindowState>,
}

impl Compositor {
    pub fn inspect(&mut self) -> Snapshot {
        let now = time::Instant::now();
        let elapsed = self
            .inspect_state
            .at
            .map(|at| now.duration_since(at).as_secs_f64())
            .filter(|s| *s > 0.0);

        // Same rule as composite_frame: everything from the topmost
        // fullscreen surface upwards is drawn.
        let first_visible = self
            .surface_stack
            .iter()
            .rposition(|id| {
                self.surfaces[*id]
                    .configuration
                    .map_or(true, |conf| conf.fullscreen)
            })
            .unwrap_or_default();
        let covering = self.surface_stack.get(first_visible).copied();

        let xwm = self.xwm.as_ref();
        let mut clients = HashSet::new();
        let mut surfaces = Vec::new();
        let mut other_surfaces = 0;
        let mut new_commits = HashMap::new();

        // Stack top first, then everything not in the stack.
        let mut order: Vec<SurfaceKey> = self.surface_stack.iter().rev().copied().collect();
        order.extend(self.surfaces.keys().filter(|k| !self.surface_stack.contains(k)));

        for key in order {
            let surf = &self.surfaces[key];
            if let Some(client) = surf.wl_surface.client() {
                clients.insert(client.id());
            }
            new_commits.insert(key, surf.commits);

            let (kind, xwin) = match &surf.role.current {
                Some(SurfaceRole::XdgToplevel { .. }) => ("wayland", None),
                Some(SurfaceRole::XWayland { serial }) => {
                    ("x11", xwm.and_then(|xwm| xwm.xwindow_for_serial(*serial)))
                }
                Some(SurfaceRole::Cursor) => ("cursor", None),
                None => {
                    other_surfaces += 1;
                    continue;
                }
            };
            if kind == "cursor" {
                continue;
            }

            let stack_pos = self.surface_stack.iter().position(|k| *k == key);
            let mapped = stack_pos.is_some();
            let composited = stack_pos.is_some_and(|p| p >= first_visible);
            let conf = surf.configuration;

            let buffer = surf.content.as_ref().map(|c| {
                let b = &self.buffers[c.buffer];
                let (kind, format) = match &b.backing {
                    BufferBacking::Shm { format, .. } => ("shm", format),
                    BufferBacking::Dmabuf { format, .. } => ("dmabuf", format),
                };
                BufferState {
                    width: format.width,
                    height: format.height,
                    kind,
                    format: format!("{:?}", format.format),
                }
            });

            let commits_per_sec = match (elapsed, self.inspect_state.commits.get(&key)) {
                (Some(secs), Some(prev)) => (surf.commits.saturating_sub(*prev)) as f64 / secs,
                _ => 0.0,
            };

            let problem = if surf.pending_configure.is_some() {
                Some("Waiting for the app to acknowledge its configure (size/state)".to_owned())
            } else if conf.is_none() && surf.sent_configuration.is_none() {
                Some("Not configured yet".to_owned())
            } else if surf.content.is_none() {
                Some("No buffer committed — the app hasn't drawn anything yet".to_owned())
            } else if kind == "x11" && xwin.is_none() {
                Some("X11 surface with no matching X11 window".to_owned())
            } else if kind == "x11" && !mapped && xwin.is_some_and(|x| !x.mapped) {
                Some("X11 window not mapped yet".to_owned())
            } else if !mapped {
                Some("Not mapped".to_owned())
            } else if !composited {
                let by = covering
                    .map(|k| &self.surfaces[k])
                    .map(|s| s.title.clone().or(s.app_id.clone()).unwrap_or_else(|| "a window".into()))
                    .unwrap_or_else(|| "a window".into());
                Some(format!("Hidden behind fullscreen window “{by}”"))
            } else if commits_per_sec == 0.0 && elapsed.is_some() {
                Some("Visible, but not presenting new frames".to_owned())
            } else {
                match (conf, &buffer) {
                    (Some(c), Some(b)) if c.size.x != b.width || c.size.y != b.height => Some(format!(
                        "Buffer is {}×{} but was configured as {}×{}",
                        b.width, b.height, c.size.x, c.size.y
                    )),
                    _ => None,
                }
            };

            surfaces.push(SurfaceState {
                key: format!("{:?}", key.data()),
                window_id: self.surface_to_window_id.get(&key).copied(),
                kind,
                title: surf.title.clone().unwrap_or_default(),
                app_id: surf.app_id.clone().unwrap_or_default(),
                x11_window: xwin.map(|x| x.id),
                mapped,
                stack_index: stack_pos.map(|p| self.surface_stack.len() - 1 - p),
                composited,
                active: self.active_surface == Some(key),
                fullscreen: conf.is_some_and(|c| c.fullscreen),
                configured: conf.map(|c| [c.size.x, c.size.y]),
                buffer,
                awaiting_configure_ack: surf.pending_configure.is_some(),
                commits: surf.commits,
                commits_per_sec,
                problem,
            });
        }

        let x11_windows = xwm
            .map(|xwm| {
                xwm.xwindows
                    .values()
                    .map(|x| {
                        let has_surface = x
                            .serial
                            .is_some_and(|s| self.xwayland_surface_lookup.contains_key(&s));
                        let problem = if x.mapped && x.serial.is_none() {
                            Some("Mapped, but Xwayland never linked it to a surface".to_owned())
                        } else if x.mapped && !has_surface {
                            Some("Mapped, but its Wayland surface is missing".to_owned())
                        } else {
                            None
                        };
                        X11WindowState {
                            id: x.id,
                            title: x.title.clone().unwrap_or_default(),
                            class: x.app_id.clone().unwrap_or_default(),
                            geometry: [x.x, x.y, x.width, x.height],
                            override_redirect: x.override_redirect,
                            mapped: x.mapped,
                            fullscreen: xwm.is_fullscreen(x),
                            has_surface,
                            problem,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        self.inspect_state = InspectState { at: Some(now), commits: new_commits };

        Snapshot { clients: clients.len(), surfaces, other_surfaces, x11_windows }
    }
}
