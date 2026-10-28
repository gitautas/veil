use wayland_server::{
    Resource as _,
    protocol::{wl_subcompositor, wl_subsurface},
};

use crate::session::compositor::{
    Compositor,
    surface::{SurfaceKey, SurfaceRole},
};

impl wayland_server::GlobalDispatch<wl_subcompositor::WlSubcompositor, ()> for Compositor {
    fn bind(
        _state: &mut Self,
        _handle: &wayland_server::DisplayHandle,
        _client: &wayland_server::Client,
        resource: wayland_server::New<wl_subcompositor::WlSubcompositor>,
        _global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }
}

impl wayland_server::Dispatch<wl_subcompositor::WlSubcompositor, ()> for Compositor {
    fn request(
        state: &mut Self,
        _client: &wayland_server::Client,
        resource: &wl_subcompositor::WlSubcompositor,
        request: wl_subcompositor::Request,
        _data: &(),
        _dhandle: &wayland_server::DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        match request {
            wl_subcompositor::Request::GetSubsurface { id, surface, parent } => {
                let child = *surface.data::<SurfaceKey>().expect("surface has no key");
                let parent = *parent.data::<SurfaceKey>().expect("surface has no key");

                if child == parent || state.is_ancestor(child, parent) {
                    resource.post_error(
                        wl_subcompositor::Error::BadParent,
                        "A surface can't be its own ancestor.",
                    );
                    return;
                }
                let Some(surf) = state.surfaces.get_mut(child) else { return };
                if surf.role.current.is_some() || surf.role.pending.is_some() {
                    resource.post_error(
                        wl_subcompositor::Error::BadSurface,
                        "The surface already has a role.",
                    );
                    return;
                }
                // The role takes effect immediately.
                surf.role.current = Some(SurfaceRole::Subsurface { parent });
                surf.subsurface_position = glam::IVec2::ZERO;

                // New subsurfaces go on top of the parent's stack.
                if let Some(parent) = state.surfaces.get_mut(parent) {
                    parent.stack.push(child);
                    if let Some(pending) = parent.pending_stack.as_mut() {
                        pending.push(child);
                    }
                }

                data_init.init(id, child);
            }
            wl_subcompositor::Request::Destroy => (),
            _ => unreachable!(),
        }
    }
}

impl wayland_server::Dispatch<wl_subsurface::WlSubsurface, SurfaceKey> for Compositor {
    fn request(
        state: &mut Self,
        _client: &wayland_server::Client,
        resource: &wl_subsurface::WlSubsurface,
        request: wl_subsurface::Request,
        data: &SurfaceKey,
        _dhandle: &wayland_server::DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let child = *data;
        let parent = match state.surfaces.get(child).and_then(|s| s.role.current.clone()) {
            Some(SurfaceRole::Subsurface { parent }) => parent,
            // The surface (or its role) is gone; nothing to do.
            _ => return,
        };

        let above = matches!(request, wl_subsurface::Request::PlaceAbove { .. });
        match request {
            wl_subsurface::Request::SetPosition { x, y } => {
                if let Some(surf) = state.surfaces.get_mut(child) {
                    surf.pending_subsurface_position = Some(glam::IVec2::new(x, y));
                }
            }
            wl_subsurface::Request::PlaceAbove { sibling } | wl_subsurface::Request::PlaceBelow { sibling } => {
                let sibling = *sibling.data::<SurfaceKey>().expect("surface has no key");
                let Some(parent_surf) = state.surfaces.get_mut(parent) else { return };

                let mut stack = parent_surf.pending_stack.take().unwrap_or_else(|| parent_surf.stack.clone());
                stack.retain(|k| *k != child);
                // The sibling must be the parent or another of its subsurfaces.
                let Some(pos) = stack.iter().position(|k| *k == sibling).filter(|_| sibling != child) else {
                    parent_surf.pending_stack = Some(stack);
                    resource.post_error(
                        wl_subsurface::Error::BadSurface,
                        "The sibling isn't the parent or one of its subsurfaces.",
                    );
                    return;
                };
                stack.insert(if above { pos + 1 } else { pos }, child);
                parent_surf.pending_stack = Some(stack);
            }
            // All subsurfaces are treated as desynchronized: their commits
            // apply right away rather than waiting for the parent's.
            wl_subsurface::Request::SetSync | wl_subsurface::Request::SetDesync => (),
            wl_subsurface::Request::Destroy => state.detach_subsurface(child),
            _ => unreachable!(),
        }
    }

    fn destroyed(
        state: &mut Self,
        _client: wayland_server::backend::ClientId,
        _resource: &wl_subsurface::WlSubsurface,
        data: &SurfaceKey,
    ) {
        state.detach_subsurface(*data);
    }
}

impl Compositor {
    /// Whether `ancestor` is `id`'s parent, or its parent's parent, and so on.
    pub fn is_ancestor(&self, ancestor: SurfaceKey, id: SurfaceKey) -> bool {
        let mut current = id;
        // Bounded, in case of a (should-be-impossible) cycle.
        for _ in 0..64 {
            match self.surfaces.get(current).and_then(|s| s.role.current.clone()) {
                Some(SurfaceRole::Subsurface { parent }) if parent == ancestor => return true,
                Some(SurfaceRole::Subsurface { parent }) => current = parent,
                _ => return false,
            }
        }
        false
    }
}
