//! Who's connected, for the client UI.
//!
//! Deliberately doesn't expose addresses: every viewer gets the list, and the
//! page may be public.

use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::broadcast;

use crate::session::control::{SessionEvent, ViewerInfo};

/// Input within this long counts as "controlling".
const ACTIVE_INPUT: Duration = Duration::from_secs(5);

pub struct Viewers {
    inner: Mutex<Inner>,
    events_tx: broadcast::Sender<SessionEvent>,
}

#[derive(Default)]
struct Inner {
    next_id: u32,
    viewers: BTreeMap<u32, Viewer>,
}

struct Viewer {
    connected_at: Instant,
    lan: bool,
    last_input: Option<Instant>,
}

/// Keeps a viewer listed until dropped (i.e. until its session ends).
pub struct ViewerGuard {
    pub id: u32,
    viewers: Arc<Viewers>,
}

impl Drop for ViewerGuard {
    fn drop(&mut self) {
        self.viewers.inner.lock().unwrap().viewers.remove(&self.id);
        self.viewers.broadcast();
    }
}

impl Viewers {
    pub fn new(events_tx: broadcast::Sender<SessionEvent>) -> Arc<Self> {
        Arc::new(Self { inner: Mutex::default(), events_tx })
    }

    pub fn join(self: &Arc<Self>, peer: SocketAddr) -> ViewerGuard {
        let id = {
            let mut inner = self.inner.lock().unwrap();
            inner.next_id += 1;
            let id = inner.next_id;
            inner.viewers.insert(
                id,
                Viewer { connected_at: Instant::now(), lan: is_local(peer.ip()), last_input: None },
            );
            id
        };
        self.broadcast();
        ViewerGuard { id, viewers: self.clone() }
    }

    /// Note that a viewer sent input.
    pub fn input(&self, id: u32) {
        if let Some(v) = self.inner.lock().unwrap().viewers.get_mut(&id) {
            v.last_input = Some(Instant::now());
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.lock().unwrap().viewers.is_empty()
    }

    /// Send the current list to every session.
    pub fn broadcast(&self) {
        let now = Instant::now();
        let list = self
            .inner
            .lock()
            .unwrap()
            .viewers
            .iter()
            .map(|(id, v)| ViewerInfo {
                id: *id,
                connected_secs: now.duration_since(v.connected_at).as_secs(),
                network: if v.lan { "lan" } else { "internet" },
                active: v.last_input.is_some_and(|t| now.duration_since(t) < ACTIVE_INPUT),
            })
            .collect();
        // No receivers just means nobody's listening yet.
        let _ = self.events_tx.send(SessionEvent::Viewers(Arc::new(list)));
    }
}

/// Private, loopback and link-local addresses. NAT-reflected LAN clients show
/// up with the router's (private) address, so they count as local too.
fn is_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_local(IpAddr::V4(v4));
            }
            v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local()
        }
    }
}
