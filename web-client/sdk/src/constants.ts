/**
 * Constants for the Veil SDK (QUIC/WebTransport transport).
 */

// ─── Server defaults ────────────────────────────────────────────────────────

/** Default WebTransport server URL. */
/** Default QUIC/WebTransport port; the cert-hash endpoint is on the next port. */
export const DEFAULT_QUIC_PORT = 7801;
export const DEFAULT_SERVER_URL = `https://localhost:${DEFAULT_QUIC_PORT}`;

/** Default codec. */
export const DEFAULT_VIDEO_CODEC = 'h264';

/** Default resolution width. */
export const DEFAULT_WIDTH = 1280;

/** Default resolution height. */
export const DEFAULT_HEIGHT = 720;

/** Default framerate. */
export const DEFAULT_FRAMERATE = 60;

// ─── Protocol constants (must match src/transport/proto.rs) ────────────────

/** Server → client message types. */
export const MSG_HELLO = 0x00;
export const MSG_VIDEO = 0x01;
export const MSG_AUDIO = 0x02;
export const MSG_CTRL_DISPLAY = 0x10;
export const MSG_CTRL_CURSOR = 0x11;
export const MSG_CTRL_POINTER_LOCK = 0x12;
export const MSG_CTRL_POINTER_RELEASE = 0x13;
export const MSG_CTRL_PONG = 0x14;
export const MSG_WINDOW_LIST = 0x15;
/** JSON: what the server is doing (GPU, codec, rate control…). See `ServerInfo`. */
export const MSG_SERVER_INFO = 0x16;
/** JSON: encoder statistics for the last second. See `ServerStats`. */
export const MSG_SERVER_STATS = 0x17;
/** JSON: the server's virtual gamepads. See `GamepadStatus`. */
export const MSG_GAMEPAD_STATUS = 0x18;
/** JSON: the compositor's windows and why they are or aren't visible. See `CompositorState`. */
export const MSG_COMPOSITOR_STATE = 0x19;
/** JSON: connected viewers. See `ViewerList`. */
export const MSG_VIEWERS = 0x1A;
/** Rumble for one of our pads: [index][strong u16][weak u16][duration_ms u16]. */
export const MSG_GAMEPAD_RUMBLE = 0x1B;
/** This viewer's network and the stream's adaptation (JSON), every second. */
export const MSG_NETWORK = 0x1C;
/** Do something on a passed-through HID device: [handle][kind][request u32][report number][data]. */
export const MSG_HID_REQUEST = 0x1D;
/** Whether a HID passthrough opened (JSON). */
export const MSG_HID_STATUS = 0x1E;

// Client → server streams (HID passthrough).
/** Pass a device through (JSON with its rebuilt report descriptor). */
export const HID_OPEN = 0x40;
/** Stop passing a device through: [handle]. */
export const HID_CLOSE = 0x41;
/** Answer a 0x1D request: [handle][kind][request u32][status][data]. */
export const HID_REPLY = 0x43;

/** Frame type flags. */
export const FRAME_KEY = 0x00;
export const FRAME_DELTA = 0x01;

/** Codec identifiers (from MSG_HELLO). */
export const CODEC_H264 = 0x01;
export const CODEC_H265 = 0x02;
/** Experimental. */
export const CODEC_AV1 = 0x03;

/** Client → server input datagram types. */
export const INPUT_KEY_DOWN = 0x20;
export const INPUT_KEY_UP = 0x21;
export const INPUT_MOUSE_MOVE = 0x22;
export const INPUT_MOUSE_DOWN = 0x23;
export const INPUT_MOUSE_UP = 0x24;
export const INPUT_SCROLL = 0x25;
export const INPUT_GAMEPAD_BTN = 0x26;
export const INPUT_GAMEPAD_AXIS = 0x27;
export const INPUT_REQ_KEYFRAME = 0x28;
export const INPUT_POINTER_ENTER = 0x29;
export const INPUT_POINTER_LEAVE = 0x2A;
export const INPUT_PING = 0x2B;
export const INPUT_GAMEPAD_AVAILABLE = 0x2C;
export const INPUT_GAMEPAD_UNAVAILABLE = 0x2D;
export const INPUT_FOCUS_WINDOW = 0x2E;
/** JSON `StreamParamsRequest`: change encoding settings. */
export const INPUT_SET_PARAMS = 0x2F;
/** UTF-8 GPU name: restart the server on that GPU. */
export const INPUT_SWITCH_GPU = 0x30;
/** Full gamepad snapshot: [index][seq u32][buttons u32][lt][rt][lx,ly,rx,ry i16]. */
export const INPUT_GAMEPAD_STATE = 0x31;
/** What this browser can decode (JSON), for the server's Auto codec. */
export const INPUT_DECODE_CAPS = 0x32;
/** An input report from a passed-through HID device (datagram): [handle][report]. */
export const INPUT_HID_REPORT = 0x42;

// ─── Client timing ──────────────────────────────────────────────────────────

/** Stats update interval (ms). */
export const STATS_UPDATE_INTERVAL = 1000;

/** Gamepad polling interval (ms). Browsers sample pads at up to ~250 Hz. */
export const GAMEPAD_POLL_INTERVAL = 4;

/** Resend each pad's full state at least this often (ms), so lost datagrams heal. */
export const GAMEPAD_RESEND_INTERVAL = 100;

/** Radial stick deadzone (0–1), rescaled; only meant to hide drift. */
export const GAMEPAD_DEADZONE = 0.05;

// ─── WebCodecs codec strings ────────────────────────────────────────────────

/** H.264 Baseline Profile Level 3.1 — works on all modern browsers. */
export const H264_CODEC_STRING = 'avc1.42E01F';

/**
 * Build a WebCodecs-compatible H.264 codec string from the SPS profile_idc and level_idc.
 * profile_idc: 66=Baseline, 77=Main, 100=High
 * level_idc: SPS level × 10 (e.g. 51 for 5.1)
 */
export function h264CodecString(profileIdc: number, levelIdc: number): string {
    let pp: string;
    let cc: string;
    if (profileIdc === 66) { pp = '42'; cc = 'E0'; }
    else if (profileIdc === 77) { pp = '4D'; cc = '40'; }
    else { pp = '64'; cc = '00'; } // High (100) and anything else
    const ll = levelIdc.toString(16).padStart(2, '0').toUpperCase();
    return `avc1.${pp}${cc}${ll}`;
}

/** H.265 Main Profile — Chrome only, partial support. */
export const H265_CODEC_STRING = 'hev1.1.6.L93.B0';

/** AV1 Main Profile, level 5.1, 8-bit. */
export const AV1_CODEC_STRING = 'av01.0.13M.08';

/** The default WebCodecs codec string for each codec. */
export function defaultCodecString(codec: 'h264' | 'h265' | 'av1'): string {
  if (codec === 'h265') return H265_CODEC_STRING;
  if (codec === 'av1') return AV1_CODEC_STRING;
  return H264_CODEC_STRING;
}

// ─── Input key-code lookup table ────────────────────────────────────────────

/**
 * Maps `KeyboardEvent.code` (physical key) → Linux evdev scancode.
 *
 * `e.code` is the physical key position, independent of layout or modifiers.
 * This is what games and Wayland compositors expect: e.g. WASD always maps to
 * scancodes 17/30/31/32 regardless of the user's keyboard layout.
 */
export const SCAN_CODE_MAP: Record<string, number> = {
  // Function & system
  Escape: 1,
  F1: 59, F2: 60, F3: 61, F4: 62, F5: 63, F6: 64,
  F7: 65, F8: 66, F9: 67, F10: 68, F11: 87, F12: 88,
  PrintScreen: 99, ScrollLock: 70, Pause: 119,

  // Number row
  Backquote: 41,
  Digit1: 2, Digit2: 3, Digit3: 4, Digit4: 5, Digit5: 6,
  Digit6: 7, Digit7: 8, Digit8: 9, Digit9: 10, Digit0: 11,
  Minus: 12, Equal: 13, Backspace: 14,

  // Top letter row
  Tab: 15,
  KeyQ: 16, KeyW: 17, KeyE: 18, KeyR: 19, KeyT: 20,
  KeyY: 21, KeyU: 22, KeyI: 23, KeyO: 24, KeyP: 25,
  BracketLeft: 26, BracketRight: 27, Enter: 28,

  // Middle letter row
  CapsLock: 58,
  KeyA: 30, KeyS: 31, KeyD: 32, KeyF: 33, KeyG: 34,
  KeyH: 35, KeyJ: 36, KeyK: 37, KeyL: 38,
  Semicolon: 39, Quote: 40, Backslash: 43,

  // Bottom letter row
  ShiftLeft: 42, ShiftRight: 54,
  KeyZ: 44, KeyX: 45, KeyC: 46, KeyV: 47, KeyB: 48,
  KeyN: 49, KeyM: 50,
  Comma: 51, Period: 52, Slash: 53,

  // Modifiers & space
  ControlLeft: 29, ControlRight: 97,
  AltLeft: 56, AltRight: 100,
  MetaLeft: 125, MetaRight: 126,
  Space: 57,
  ContextMenu: 127,

  // Navigation cluster
  Insert: 110, Delete: 111,
  Home: 102, End: 107,
  PageUp: 104, PageDown: 109,
  ArrowUp: 103, ArrowDown: 108, ArrowLeft: 105, ArrowRight: 106,

  // Numpad
  NumLock: 69,
  Numpad0: 82, Numpad1: 79, Numpad2: 80, Numpad3: 81,
  Numpad4: 75, Numpad5: 76, Numpad6: 77,
  Numpad7: 71, Numpad8: 72, Numpad9: 73,
  NumpadDecimal: 83, NumpadEnter: 96,
  NumpadAdd: 78, NumpadSubtract: 74, NumpadMultiply: 55, NumpadDivide: 98,
};

/** Mouse button index → server button code (evdev BTN_LEFT = 272). */
export const MOUSE_BUTTON_MAP: Record<number, number> = {
  0: 272, // BTN_LEFT
  1: 274, // BTN_MIDDLE
  2: 273, // BTN_RIGHT
  3: 275, // BTN_SIDE / back
  4: 276, // BTN_EXTRA / forward
};

/** Standard gamepad button layout (W3C Gamepad API). */
export const GAMEPAD_BUTTON_MAP: Record<number, string> = {
  0: 'a',
  1: 'b',
  2: 'x',
  3: 'y',
  4: 'lb',
  5: 'rb',
  6: 'lt',
  7: 'rt',
  8: 'select',
  9: 'start',
  10: 'ls',
  11: 'rs',
  12: 'dpad_up',
  13: 'dpad_down',
  14: 'dpad_left',
  15: 'dpad_right',
  16: 'home',
};

/** Standard gamepad axis layout. */
export const GAMEPAD_AXIS_MAP: Record<number, string> = {
  0: 'left_stick_x',
  1: 'left_stick_y',
  2: 'right_stick_x',
  3: 'right_stick_y',
};
