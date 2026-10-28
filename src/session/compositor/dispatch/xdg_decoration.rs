//! xdg-decoration: windows are always "server-side" decorated, and we draw no
//! decorations at all. Every window is a borderless fullscreen stream, so this
//! just stops clients (e.g. GLFW apps like kitty) from drawing title bars.

use wayland_protocols::xdg::decoration::zv1::server::{
    zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
};
use wayland_server::Resource as _;

use crate::session::compositor::{Compositor, surface::SurfaceKey};

impl wayland_server::GlobalDispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, ()>
    for Compositor
{
    fn bind(
        _state: &mut Self,
        _handle: &wayland_server::DisplayHandle,
        _client: &wayland_server::Client,
        resource: wayland_server::New<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
        _global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }
}

impl wayland_server::Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, ()>
    for Compositor
{
    fn request(
        state: &mut Self,
        _client: &wayland_server::Client,
        _resource: &zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        request: zxdg_decoration_manager_v1::Request,
        _data: &(),
        _dhandle: &wayland_server::DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        match request {
            zxdg_decoration_manager_v1::Request::GetToplevelDecoration { id, toplevel } => {
                let surface = *toplevel.data::<SurfaceKey>().expect("toplevel has no surface");
                let decoration = data_init.init(id, surface);
                send_server_side(state, &decoration, surface);
            }
            zxdg_decoration_manager_v1::Request::Destroy => (),
            _ => unreachable!(),
        }
    }
}

/// Tell the client its decorations are server-side. The mode applies with the
/// next xdg_surface.configure, so if the window was already configured, make
/// sure another one goes out.
fn send_server_side(
    state: &mut Compositor,
    decoration: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
    surface: SurfaceKey,
) {
    decoration.configure(zxdg_toplevel_decoration_v1::Mode::ServerSide);
    if let Some(surf) = state.surfaces.get_mut(surface) {
        surf.sent_configuration = None;
    }
}

impl wayland_server::Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, SurfaceKey>
    for Compositor
{
    fn request(
        state: &mut Self,
        _client: &wayland_server::Client,
        resource: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        request: zxdg_toplevel_decoration_v1::Request,
        data: &SurfaceKey,
        _dhandle: &wayland_server::DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        match request {
            // Whatever the client prefers, it's server-side (i.e. none).
            zxdg_toplevel_decoration_v1::Request::SetMode { .. }
            | zxdg_toplevel_decoration_v1::Request::UnsetMode => {
                send_server_side(state, resource, *data);
            }
            zxdg_toplevel_decoration_v1::Request::Destroy => (),
            _ => unreachable!(),
        }
    }
}
