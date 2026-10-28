//! Binary wire protocol for Veil WebTransport media streaming.
//!
//! All multi-byte integers are big-endian.
//!
//! # Server → Client (QUIC unidirectional streams)
//!
//! ```text
//! MSG_HELLO         [0x00] codec:u8  width:u16  height:u16  fps:u16  profile_idc:u8  level_idc:u8  gop_size:u16  in_flight:u8
//! MSG_VIDEO         [0x01] frame_type:u8  seq:u32  pts_ms:u32  <data>
//! MSG_AUDIO         [0x02] seq:u32  pts_ms:u32  <data>
//! MSG_CTRL_DISPLAY  [0x10] width:u16  height:u16  fps:u16
//! MSG_CTRL_CURSOR   [0x11] hotspot_x:u16  hotspot_y:u16  kind:u8  <kind-payload>
//!                     kind 0x00 = hidden (no payload)
//!                     kind 0x01 = PNG image: len:u32  <bytes>
//!                     kind 0x02 = named icon: len:u8  <utf8>
//! MSG_CTRL_POINTER_LOCK    [0x12] x:f32  y:f32
//! MSG_CTRL_POINTER_RELEASE [0x13]
//! MSG_WINDOW_LIST   [0x15] count:u8
//!                     [id:u32  active:u8  title_len:u8  <title:utf8>  app_id_len:u8  <app_id:utf8>]*
//! ```
//!
//! # Client → Server (QUIC datagrams)
//!
//! ```text
//! INPUT_KEY_DOWN    [0x20] keycode:u32  char_codepoint:u32
//! INPUT_KEY_UP      [0x21] keycode:u32
//! INPUT_MOUSE_MOVE  [0x22] x:f32  y:f32  dx:f32  dy:f32
//! INPUT_MOUSE_DOWN  [0x23] button:u32  x:f32  y:f32
//! INPUT_MOUSE_UP    [0x24] button:u32  x:f32  y:f32
//! INPUT_SCROLL      [0x25] dx:f32  dy:f32
//! INPUT_GAMEPAD_BTN [0x26] id:u8  button:u8  state:u8  value:f32
//! INPUT_GAMEPAD_AXIS[0x27] id:u8  axis:u8  value:f32
//! INPUT_REQ_KEYFRAME   [0x28]
//! INPUT_POINTER_ENTER  [0x29]
//! INPUT_POINTER_LEAVE  [0x2A]
//! INPUT_PING           [0x2B] timestamp_ms:u64
//! INPUT_FOCUS_WINDOW   [0x2E] window_id:u32
//! ```

use bytes::{BufMut, Bytes, BytesMut};

// ─── Message type constants ─────────────────────────────────────────────────

/// Session hello — first message sent to every new client.
pub const MSG_HELLO: u8 = 0x00;
/// Video frame stream.
pub const MSG_VIDEO: u8 = 0x01;
/// Audio frame (stream or datagram depending on size).
pub const MSG_AUDIO: u8 = 0x02;
/// Display parameters changed.
pub const MSG_CTRL_DISPLAY: u8 = 0x10;
/// Cursor update.
pub const MSG_CTRL_CURSOR: u8 = 0x11;
/// Pointer locked at (x, y).
pub const MSG_CTRL_POINTER_LOCK: u8 = 0x12;
/// Pointer lock released.
pub const MSG_CTRL_POINTER_RELEASE: u8 = 0x13;
/// Pong (echo of client's INPUT_PING timestamp).
pub const MSG_CTRL_PONG: u8 = 0x14;
/// Current list of mapped toplevel windows.
pub const MSG_WINDOW_LIST: u8 = 0x15;
/// What the server is doing (GPU, codec, rate control…), JSON-encoded
/// [`StreamInfo`](crate::session::control::StreamInfo).
pub const MSG_SERVER_INFO: u8 = 0x16;
/// Encoder statistics for the last second, JSON-encoded
/// [`EncoderStats`](crate::session::control::EncoderStats).
pub const MSG_SERVER_STATS: u8 = 0x17;
/// The server's virtual gamepads, a JSON list of
/// [`GamepadStatus`](crate::session::control::GamepadStatus).
pub const MSG_GAMEPAD_STATUS: u8 = 0x18;
/// The compositor's windows and why they are or aren't visible, JSON
/// [`CompositorState`](crate::session::control::CompositorState).
pub const MSG_COMPOSITOR_STATE: u8 = 0x19;
/// Connected viewers: JSON `{"you": id, "viewers": [ViewerInfo]}`.
pub const MSG_VIEWERS: u8 = 0x1A;
/// Rumble for one of the recipient's pads:
/// `[index:u8][strong:u16][weak:u16][duration_ms:u16]` (big-endian).
pub const MSG_GAMEPAD_RUMBLE: u8 = 0x1B;
/// This viewer's network and the stream's adaptation, JSON
/// [`NetworkStatus`](crate::transport::session::NetworkStatus), every second.
pub const MSG_NETWORK: u8 = 0x1C;

// ─── Frame type flags ───────────────────────────────────────────────────────

pub const FRAME_KEY: u8 = 0x00;
pub const FRAME_DELTA: u8 = 0x01;

// ─── Codec identifiers ──────────────────────────────────────────────────────

pub const CODEC_H264: u8 = 0x01;
pub const CODEC_H265: u8 = 0x02;
pub const CODEC_AV1: u8 = 0x03;

// ─── Input datagram type constants ─────────────────────────────────────────

pub const INPUT_KEY_DOWN: u8 = 0x20;
pub const INPUT_KEY_UP: u8 = 0x21;
pub const INPUT_MOUSE_MOVE: u8 = 0x22;
pub const INPUT_MOUSE_DOWN: u8 = 0x23;
pub const INPUT_MOUSE_UP: u8 = 0x24;
pub const INPUT_SCROLL: u8 = 0x25;
pub const INPUT_GAMEPAD_BTN: u8 = 0x26;
pub const INPUT_GAMEPAD_AXIS: u8 = 0x27;
pub const INPUT_REQ_KEYFRAME: u8 = 0x28;
pub const INPUT_POINTER_ENTER: u8 = 0x29;
pub const INPUT_POINTER_LEAVE: u8 = 0x2A;
pub const INPUT_PING: u8 = 0x2B;
pub const INPUT_GAMEPAD_AVAILABLE: u8 = 0x2C;
pub const INPUT_GAMEPAD_UNAVAILABLE: u8 = 0x2D;
pub const INPUT_FOCUS_WINDOW: u8 = 0x2E;
/// Change encoding settings: JSON
/// [`StreamParamsRequest`](crate::session::control::StreamParamsRequest).
pub const INPUT_SET_PARAMS: u8 = 0x2F;
/// Restart on another GPU: UTF-8 GPU name, as listed in the server info.
pub const INPUT_SWITCH_GPU: u8 = 0x30;
/// A full gamepad snapshot, sent on every change and periodically:
/// `[index:u8][seq:u32][buttons:u32][lt:u8][rt:u8][lx,ly,rx,ry:i16]`
/// (big-endian). Lost or reordered datagrams can't leave inputs stuck, since
/// the next snapshot carries the whole state.
pub const INPUT_GAMEPAD_STATE: u8 = 0x31;
/// What the browser can decode, JSON
/// [`DecodeCaps`](crate::transport::adapt::DecodeCaps); sent on connect, on
/// change and every few seconds. Handled by the transport.
pub const INPUT_DECODE_CAPS: u8 = 0x32;

// ─── Encoding helpers ───────────────────────────────────────────────────────

/// Session hello: codec + initial stream parameters.
pub fn encode_hello(codec: u8, width: u32, height: u32, fps: u32,
                    profile_idc: u8, level_idc: u8, gop_size: u32, in_flight: u8) -> Bytes {
    let mut b = BytesMut::with_capacity(13);
    b.put_u8(MSG_HELLO);
    b.put_u8(codec);
    b.put_u16(width as u16);
    b.put_u16(height as u16);
    b.put_u16(fps as u16);
    b.put_u8(profile_idc);
    b.put_u8(level_idc);
    b.put_u16(gop_size as u16);
    b.put_u8(in_flight);
    b.freeze()
}

/// Video frame payload.
pub fn encode_video(is_keyframe: bool, seq: u64, pts_ms: u32, data: &[u8]) -> Bytes {
    let mut b = BytesMut::with_capacity(10 + data.len());
    b.put_u8(MSG_VIDEO);
    b.put_u8(if is_keyframe { FRAME_KEY } else { FRAME_DELTA });
    b.put_u32(seq as u32);
    b.put_u32(pts_ms);
    b.put_slice(data);
    b.freeze()
}

/// Audio frame payload.
pub fn encode_audio(seq: u64, pts_ms: u32, data: &[u8]) -> Bytes {
    let mut b = BytesMut::with_capacity(9 + data.len());
    b.put_u8(MSG_AUDIO);
    b.put_u32(seq as u32);
    b.put_u32(pts_ms);
    b.put_slice(data);
    b.freeze()
}

/// Display-params control message.
pub fn encode_ctrl_display(width: u32, height: u32, fps: u32) -> Bytes {
    let mut b = BytesMut::with_capacity(7);
    b.put_u8(MSG_CTRL_DISPLAY);
    b.put_u16(width as u16);
    b.put_u16(height as u16);
    b.put_u16(fps as u16);
    b.freeze()
}

/// Cursor update control message.
pub fn encode_ctrl_cursor(
    image: Option<&[u8]>,
    icon: Option<&str>,
    hotspot_x: u32,
    hotspot_y: u32,
) -> Bytes {
    let mut b = BytesMut::new();
    b.put_u8(MSG_CTRL_CURSOR);
    b.put_u16(hotspot_x as u16);
    b.put_u16(hotspot_y as u16);
    match (image, icon) {
        (Some(img), _) => {
            b.put_u8(0x01);
            b.put_u32(img.len() as u32);
            b.put_slice(img);
        }
        (None, Some(name)) => {
            let nb = name.as_bytes();
            b.put_u8(0x02);
            b.put_u8(nb.len().min(255) as u8);
            b.put_slice(&nb[..nb.len().min(255)]);
        }
        _ => b.put_u8(0x00),
    }
    b.freeze()
}

/// Pong response (echoes the client's ping timestamp).
pub fn encode_ctrl_pong(timestamp_ms: u64) -> Bytes {
    let mut b = BytesMut::with_capacity(9);
    b.put_u8(MSG_CTRL_PONG);
    b.put_u64(timestamp_ms);
    b.freeze()
}

/// Pointer-lock notification.
pub fn encode_ctrl_pointer_lock(x: f64, y: f64) -> Bytes {
    let mut b = BytesMut::with_capacity(9);
    b.put_u8(MSG_CTRL_POINTER_LOCK);
    b.put_f32(x as f32);
    b.put_f32(y as f32);
    b.freeze()
}

/// Pointer-release notification.
pub fn encode_ctrl_pointer_release() -> Bytes {
    Bytes::from_static(&[MSG_CTRL_POINTER_RELEASE])
}

/// Rumble for one of the recipient's pads.
pub fn encode_gamepad_rumble(index: u8, strong: u16, weak: u16, duration_ms: u16) -> Bytes {
    let mut b = BytesMut::with_capacity(8);
    b.put_u8(MSG_GAMEPAD_RUMBLE);
    b.put_u8(index);
    b.put_u16(strong);
    b.put_u16(weak);
    b.put_u16(duration_ms);
    b.freeze()
}

/// A message type byte followed by a JSON body.
pub fn encode_json(msg_type: u8, value: &impl serde::Serialize) -> Bytes {
    let mut b = BytesMut::new();
    b.put_u8(msg_type);
    // Serializing our own plain structs can't fail.
    b.extend_from_slice(&serde_json::to_vec(value).expect("serializing to JSON"));
    b.freeze()
}

/// Window list — sent whenever the compositor surface stack changes.
pub fn encode_window_list(windows: &[crate::session::control::WindowInfo]) -> Bytes {
    let mut b = BytesMut::new();
    b.put_u8(MSG_WINDOW_LIST);
    let count = windows.len().min(255) as u8;
    b.put_u8(count);
    for win in &windows[..count as usize] {
        b.put_u32(win.id);
        b.put_u8(u8::from(win.active));
        let title = win.title.as_bytes();
        let tlen = title.len().min(255);
        b.put_u8(tlen as u8);
        b.put_slice(&title[..tlen]);
        let app = win.app_id.as_bytes();
        let alen = app.len().min(255);
        b.put_u8(alen as u8);
        b.put_slice(&app[..alen]);
    }
    b.freeze()
}

// ─── Input datagram decoder ─────────────────────────────────────────────────

use crate::session::compositor::{ButtonState, KeyState};
use crate::session::GamepadLayout;
use crate::session::control::ControlMessage;

/// Parse a client input datagram into a `ControlMessage` for the reactor.
///
/// Returns `None` if the datagram is too short or has an unrecognized type.
pub fn parse_input_datagram(data: &[u8]) -> Option<ControlMessage> {
    if data.is_empty() {
        return None;
    }
    let tag = data[0];
    let p = &data[1..]; // payload

    match tag {
        INPUT_KEY_DOWN => {
            if p.len() < 8 {
                return None;
            }
            let keycode = u32::from_be_bytes(p[0..4].try_into().ok()?);
            let char_cp = u32::from_be_bytes(p[4..8].try_into().ok()?);
            Some(ControlMessage::KeyboardInput {
                key_code: keycode,
                state: KeyState::Pressed,
                char: if char_cp == 0 { None } else { char::from_u32(char_cp) },
            })
        }
        INPUT_KEY_UP => {
            if p.len() < 4 {
                return None;
            }
            let keycode = u32::from_be_bytes(p[0..4].try_into().ok()?);
            Some(ControlMessage::KeyboardInput {
                key_code: keycode,
                state: KeyState::Released,
                char: None,
            })
        }
        INPUT_MOUSE_MOVE => {
            if p.len() < 16 {
                return None;
            }
            let x = f32::from_be_bytes(p[0..4].try_into().ok()?) as f64;
            let y = f32::from_be_bytes(p[4..8].try_into().ok()?) as f64;
            let dx = f32::from_be_bytes(p[8..12].try_into().ok()?) as f64;
            let dy = f32::from_be_bytes(p[12..16].try_into().ok()?) as f64;
            if dx != 0.0 || dy != 0.0 {
                Some(ControlMessage::RelativePointerMotion(dx, dy))
            } else {
                Some(ControlMessage::PointerMotion(x, y))
            }
        }
        INPUT_MOUSE_DOWN => {
            if p.len() < 12 {
                return None;
            }
            let button = u32::from_be_bytes(p[0..4].try_into().ok()?);
            let x = f32::from_be_bytes(p[4..8].try_into().ok()?) as f64;
            let y = f32::from_be_bytes(p[8..12].try_into().ok()?) as f64;
            Some(ControlMessage::PointerInput { x, y, button_code: button, state: ButtonState::Pressed })
        }
        INPUT_MOUSE_UP => {
            if p.len() < 12 {
                return None;
            }
            let button = u32::from_be_bytes(p[0..4].try_into().ok()?);
            let x = f32::from_be_bytes(p[4..8].try_into().ok()?) as f64;
            let y = f32::from_be_bytes(p[8..12].try_into().ok()?) as f64;
            Some(ControlMessage::PointerInput { x, y, button_code: button, state: ButtonState::Released })
        }
        INPUT_SCROLL => {
            if p.len() < 8 {
                return None;
            }
            let dx = f32::from_be_bytes(p[0..4].try_into().ok()?) as f64;
            let dy = f32::from_be_bytes(p[4..8].try_into().ok()?) as f64;
            Some(ControlMessage::PointerAxis(dx, dy))
        }
        INPUT_GAMEPAD_BTN => {
            if p.len() < 7 {
                return None;
            }
            let id = p[0] as u64;
            let btn = p[1] as u32;
            let pressed = p[2] != 0;
            let value = f32::from_be_bytes(p[3..7].try_into().ok()?) as f64;
            // The triggers (standard buttons 6 and 7) are analog axes on a
            // real pad, not buttons.
            if btn == 6 || btn == 7 {
                return Some(ControlMessage::GamepadTrigger { id, trigger_code: btn - 6, value });
            }
            Some(ControlMessage::GamepadInput {
                id,
                button_code: btn,
                state: if pressed { ButtonState::Pressed } else { ButtonState::Released },
            })
        }
        // An optional 20th byte (after the index) is the pad type.
        INPUT_GAMEPAD_STATE => {
            if p.len() < 19 {
                return None;
            }
            let be16 = |i: usize| i16::from_be_bytes([p[i], p[i + 1]]);
            Some(ControlMessage::GamepadState {
                id: p[0] as u64,
                seq: u32::from_be_bytes(p[1..5].try_into().ok()?),
                state: crate::session::control::GamepadState {
                    buttons: u32::from_be_bytes(p[5..9].try_into().ok()?),
                    left_trigger: p[9],
                    right_trigger: p[10],
                    axes: [be16(11), be16(13), be16(15), be16(17)],
                },
                layout: p.get(19).copied().map(GamepadLayout::from_wire),
            })
        }
        INPUT_GAMEPAD_AXIS => {
            if p.len() < 6 {
                return None;
            }
            let id = p[0] as u64;
            let axis = p[1] as u32;
            let value = f32::from_be_bytes(p[2..6].try_into().ok()?) as f64;
            Some(ControlMessage::GamepadAxis { id, axis_code: axis, value })
        }
        INPUT_REQ_KEYFRAME => Some(ControlMessage::RefreshVideo),
        INPUT_POINTER_ENTER => Some(ControlMessage::PointerEntered),
        INPUT_POINTER_LEAVE => Some(ControlMessage::PointerLeft),
        INPUT_PING => {
            if p.len() < 8 { return None; }
            let ts = u64::from_be_bytes(p[0..8].try_into().ok()?);
            Some(ControlMessage::Ping(ts))
        }
        INPUT_GAMEPAD_AVAILABLE => {
            if p.is_empty() { return None; }
            Some(ControlMessage::GamepadAvailable(
                p[0] as u64,
                p.get(1).copied().map(GamepadLayout::from_wire),
            ))
        }
        INPUT_GAMEPAD_UNAVAILABLE => {
            if p.is_empty() { return None; }
            Some(ControlMessage::GamepadUnavailable(p[0] as u64))
        }
        INPUT_FOCUS_WINDOW => {
            if p.len() < 4 { return None; }
            let wid = u32::from_be_bytes(p[0..4].try_into().ok()?);
            Some(ControlMessage::FocusWindow(wid))
        }
        INPUT_SET_PARAMS => serde_json::from_slice(p)
            .ok()
            .map(ControlMessage::SetStreamParams),
        INPUT_SWITCH_GPU => std::str::from_utf8(p)
            .ok()
            .map(|gpu| ControlMessage::SwitchGpu(gpu.to_owned())),
        _ => None,
    }
}
