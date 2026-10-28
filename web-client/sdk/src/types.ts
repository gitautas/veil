/**
 * Type definitions for the Veil SDK (QUIC/WebTransport transport).
 */

/**
 * Configuration options for VeilClient.
 */
import type { AudioPlayer, AudioPlayerStats } from './AudioPlayer';

export interface VeilClientConfig {
  /**
   * WebTransport server URL (e.g. "https://localhost:7801").
   */
  serverUrl: string;

  /**
   * SHA-256 fingerprint (hex string) of the server's TLS certificate.
   * Required when the server uses a self-signed certificate.
   * If omitted the SDK will attempt to fetch it from `certHashUrl`.
   */
  certFingerprint?: string;

  /**
   * URL to fetch the server certificate fingerprint from.
   * Defaults to `http://<serverHost>:<serverPort+1>/cert-hash`.
   * Only used when `certFingerprint` is not provided.
   */
  certHashUrl?: string;

  /**
   * Video codec preference.
   * @default "h264"
   */
  videoCodec?: VideoCodecName;

  /**
   * Target video width (used to configure the WebCodecs decoder).
   * @default 1280
   */
  width?: number;

  /**
   * Target video height.
   * @default 720
   */
  height?: number;

  /**
   * Target framerate (used for stats display only).
   * @default 60
   */
  framerate?: number;

  /**
   * Enable verbose console logging.
   * @default false
   */
  debug?: boolean;

  /**
   * Capture keyboard input.
   * @default true
   */
  captureKeyboard?: boolean;

  /**
   * Capture mouse input.
   * @default true
   */
  captureMouse?: boolean;

  /**
   * Capture gamepad input.
   * @default true
   */
  captureGamepad?: boolean;

  /**
   * Capture the mouse when the stream is clicked (the capturing click isn't
   * sent to the app). See `VeilClient.captureMouse()`.
   * @default false
   */
  pointerLock?: boolean;

  /**
   * Where to play audio. Pass the same player to every connection so sound
   * keeps playing across reconnects; otherwise each client makes its own.
   * Either way, call `enableAudio()` from a user gesture to start sound.
   */
  audioPlayer?: AudioPlayer;
}

/** Mouse capture state. See `VeilClient.captureMouse()`. */
export interface MouseCaptureState {
  /** The browser's pointer is locked to the stream. */
  captured: boolean;
  /**
   * While captured: true when the app has locked the pointer and is getting
   * relative motion (mouselook); false when the cursor moves absolutely.
   */
  relative: boolean;
  /** The app has asked for a pointer lock, whether or not we're captured. */
  appWantsLock: boolean;
}

/**
 * Connection state of the Veil client.
 */
export enum ConnectionState {
  Disconnected = 'disconnected',
  Connecting = 'connecting',
  Connected = 'connected',
  Failed = 'failed',
  Closed = 'closed',
}

/**
 * Video statistics collected from the WebCodecs decoder.
 */
export interface VideoStats {
  fps: number;
  framesDecoded: number;
  framesDropped: number;
  bitrateBps: number;
  width: number;
  height: number;
  codec: string;
  /** End-to-end latency estimate (pts_ms delta), in ms. */
  latencyMs: number;
  /** Current WebCodecs decode queue depth. */
  decodeQueueSize: number;
}

/**
 * Audio statistics.
 */
export interface AudioStats {
  /** Packets received. */
  framesDecoded: number;
  bitrateBps: number;
  codec: string;
  /** Playback state (buffer, underruns, volume). */
  player: AudioPlayerStats;
}

export interface EncoderInfo {
    /** H.264/H.265 codec name */
    codec: string;
    /** Profile name: "Baseline" | "Main" | "High" | "H.265 Main" */
    profile: string;
    /** Level string: "4.1" | "5.1" | "5.2" etc. */
    level: string;
    /** GOP size in frames */
    gopSize: number;
    /** Encoder output buffers in flight */
    inFlight: number;
}

/**
 * Aggregate connection statistics.
 */
export interface ConnectionStats {
  video: VideoStats;
  audio: AudioStats;
  /** Approximate round-trip time in ms (derived from QUIC internals, if available). */
  rttMs: number;
  /** Encoder parameters received in the hello message. */
  encoderInfo: EncoderInfo | null;
  /** Frame decode latency: time from frame receive to WebCodecs output callback, in ms. */
  decodeLatencyMs: number;
}

// ─── Input event types ──────────────────────────────────────────────────────

export interface KeyboardInputEvent {
  type: 'keydown' | 'keyup';
  keyCode: number;
  key: string;
  char?: string;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  metaKey: boolean;
}

export interface MouseButtonEvent {
  type: 'mousedown' | 'mouseup';
  button: number;
  x: number;
  y: number;
}

export interface MouseMoveEvent {
  type: 'mousemove';
  x: number;
  y: number;
  deltaX?: number;
  deltaY?: number;
}

export interface MouseWheelEvent {
  type: 'wheel';
  deltaX: number;
  deltaY: number;
  deltaMode: number;
}

export interface GamepadButtonEvent {
  type: 'gamepadbutton';
  gamepadId: number;
  button: number;
  pressed: boolean;
  value: number;
}

export interface GamepadAxisEvent {
  type: 'gamepadaxis';
  gamepadId: number;
  axis: number;
  value: number;
}

export type InputEvent =
  | KeyboardInputEvent
  | MouseButtonEvent
  | MouseMoveEvent
  | MouseWheelEvent
  | GamepadButtonEvent
  | GamepadAxisEvent;

// ─── Veil event map ─────────────────────────────────────────────────────────

export interface CursorUpdate {
  /** PNG image as a data URL, or undefined for a named/hidden cursor. */
  image?: string;
  /** CSS cursor name (e.g. "default", "pointer"). */
  icon?: string;
  hotspotX: number;
  hotspotY: number;
}

export interface GamepadInfo {
  index: number;
  id: string;
}

export interface WindowInfo {
  /** Stable per-session numeric ID. */
  id: number;
  title: string;
  appId: string;
  /** True for the surface that currently has keyboard focus. */
  active: boolean;
}

// ─── Server-side info ───────────────────────────────────────────────────────

/**
 * Which controller to emulate for a pad: 'auto' picks by the pad's vendor,
 * 'off' doesn't emulate it at all (e.g. it's passed through as raw HID).
 */
export type GamepadType = 'auto' | 'xbox' | 'playstation' | 'off';

/** A video codec, as the server names it. AV1 is experimental. */
export type VideoCodecName = 'h264' | 'h265' | 'av1';

/** What the server is doing, sent on connect and whenever settings change. */
export interface ServerInfo {
  gpu: string;
  gpu_driver: string;
  /** Every GPU the server can encode on; `gpu` is one of these. */
  gpus: string[];
  codec: VideoCodecName;
  /** The codecs the current GPU can encode. */
  codecs: VideoCodecName[];
  codec_profile: string;
  width: number;
  height: number;
  framerate: number;
  gop_size: number;
  in_flight_frames: number;
  /** 0-6 = VBR (higher is more bitrate), 7-9 = constant QP. */
  preset: number;
  rate_control: string;
  temporal_layers: number;
  /** How frames reach the encoder: storage writes, or per-plane copies. */
  write_path: 'direct' | 'copy';
}

/** Encoder statistics over the last reporting window (about a second). */
export interface ServerStats {
  window_ms: number;
  frames: number;
  keyframes: number;
  bytes: number;
  /** Submit-to-bitstream time on the server, in ms. */
  encode_ms_avg: number;
  encode_ms_max: number;
  /** The encoder's current target bitrate, and the preset's (VBR only). */
  target_bitrate: number | null;
  preset_bitrate: number | null;
  /** The network limit in effect, if any. */
  bitrate_limit: number | null;
}

/** The server side of one client gamepad. */
export interface GamepadStatus {
  /** Server-wide pad ID. */
  id: number;
  /** The viewer the pad belongs to (compare with `ViewerList.you`). */
  viewer: number;
  /** The pad's index in that viewer's browser. */
  index: number;
  name: string;
  /** The pad type this client asked for. */
  layout: 'xbox360' | 'dualsense';
  /** What the virtual device presents itself as to games. */
  emulates: string;
  /** The virtual device's evdev node, if it was created. */
  device: string | null;
  /** Why the virtual device couldn't be created, if it wasn't. */
  error: string | null;
  /** Why it isn't the type asked for, if it isn't (e.g. a DualSense that fell back to Xbox). */
  notice: string | null;
  /** Input events the server has received for this gamepad. */
  events: number;
}

/** A Wayland surface with a window role, as the compositor sees it. */
export interface SurfaceState {
  key: string;
  /** Pass to `focusWindow()` to raise it; null if not mapped. */
  window_id: number | null;
  kind: 'wayland' | 'x11' | 'cursor';
  title: string;
  app_id: string;
  x11_window: number | null;
  /** In the window stack (configured and has a buffer). */
  mapped: boolean;
  /** 0 = top of the stack. */
  stack_index: number | null;
  /** Drawn into the video. */
  composited: boolean;
  active: boolean;
  fullscreen: boolean;
  configured: [number, number] | null;
  buffer: { width: number; height: number; kind: 'shm' | 'dmabuf'; format: string } | null;
  awaiting_configure_ack: boolean;
  commits: number;
  commits_per_sec: number;
  /** Why it isn't visible (or looks wrong), if anything. */
  problem: string | null;
}

export interface X11WindowState {
  id: number;
  title: string;
  class: string;
  /** x, y, width, height */
  geometry: [number, number, number, number];
  override_redirect: boolean;
  mapped: boolean;
  fullscreen: boolean;
  has_surface: boolean;
  problem: string | null;
}

/** The compositor's view of what's on screen. */
export interface CompositorState {
  width: number;
  height: number;
  framerate: number;
  wayland_socket: string;
  x11_display: string | null;
  app_command: string;
  app_pid: number | null;
  /** How the launched process exited, if it has (launchers exit early). */
  app_exit: string | null;
  clients: number;
  /** Top of the stack first, then unmapped windows. */
  surfaces: SurfaceState[];
  /** Surfaces without a window role. */
  other_surfaces: number;
  x11_windows: X11WindowState[];
}

/** One connected client. No addresses are shared. */
export interface ViewerInfo {
  id: number;
  connected_secs: number;
  network: 'lan' | 'internet';
  /** Sent input in the last few seconds. */
  active: boolean;
}

export interface ViewerList {
  /** This client's ID in `viewers`. */
  you: number;
  viewers: ViewerInfo[];
}

/** Settings to change; unset fields keep their current value. */
export interface StreamParamsRequest {
  codec?: VideoCodecName;
  /** Let the server pick the codec every viewer decodes best. */
  auto_codec?: boolean;
  preset?: number;
  framerate?: number;
  gop_size?: number;
  in_flight_frames?: number;
  width?: number;
  height?: number;
}

export interface VeilEventMap {
  /** Connection state changed. */
  statechange: ConnectionState;
  /** Successfully connected and streaming. */
  connected: void;
  /** Disconnected from the server. */
  disconnected: void;
  /** An error occurred. */
  error: Error;
  /** Video decoding started. */
  videostart: void;
  /** Audio decoding started. */
  audiostart: void;
  /** Statistics updated (roughly once per second). */
  stats: ConnectionStats;
  /** Server reported a display parameter change. */
  displaychange: { width: number; height: number; framerate: number };
  /** Server sent a cursor update. */
  cursor: CursorUpdate;
  /** Pointer lock engaged. */
  pointerlock: { x: number; y: number };
  /** Pointer lock released. */
  pointerrelease: void;
  /** A gamepad was connected. */
  gamepadconnected: GamepadInfo;
  /** A gamepad was disconnected. */
  gamepaddisconnected: GamepadInfo;
  /** The server sent an updated list of mapped toplevel windows. */
  windowlist: WindowInfo[];
  /** The server's settings or GPU changed (also sent on connect). */
  serverinfo: ServerInfo;
  /** Encoder statistics from the server, about once a second. */
  serverstats: ServerStats;
  /** The server's virtual gamepads changed (also sent about once a second while any exist). */
  gamepadstatus: GamepadStatus[];
  /** A game rumbled one of this client's pads (magnitudes 0-1). */
  gamepadrumble: { index: number; strong: number; weak: number; durationMs: number };
  /** Mouse capture started, stopped, or switched between absolute and relative. */
  mousecapture: MouseCaptureState;
  /** The compositor's windows, about once a second while streaming. */
  compositorstate: CompositorState;
  /** Who's connected; on connect/disconnect and every couple of seconds. */
  viewers: ViewerList;
  /** This viewer's network and the stream's adaptation, every second. */
  network: NetworkStatus;
  /** The passed-through HID devices changed (added, removed, opened or failed). */
  hidpassthrough: HidPassthroughInfo[];
  /** What this browser decodes, as reported to the server. */
  decodesupport: DecodeSupport;
}

/** A HID device passed through to the server (see `VeilClient.passThroughHid`). */
export interface HidPassthroughInfo {
  handle: number;
  name: string;
  vendorId: number;
  productId: number;
  bus: 'usb' | 'bluetooth';
  /** Waiting for the server, recreated there, or refused (see `error`). */
  status: 'opening' | 'active' | 'failed';
  error: string | null;
}

/** How well this browser decodes a codec at the stream's size. */
export type DecodeLevel = 'hardware' | 'software' | 'none';

export interface DecodeSupport {
  h264: DecodeLevel;
  h265: DecodeLevel;
  av1: DecodeLevel;
  /** Codecs the decoder fell behind on; Auto avoids them. */
  struggling: VideoCodecName[];
}

/** The server's view of this viewer's network, and how the stream adapts. */
export interface NetworkStatus {
  rtt_ms: number;
  min_rtt_ms: number;
  /** How much longer than a round trip frames take to arrive. */
  queue_ms: number;
  /** Packet loss, 0-1. */
  loss: number;
  delivered_bps: number;
  congested: boolean;
  /** What this viewer's network carries, if it's the limit (null: no limit). */
  budget_bps: number | null;
  /** Whether the server picks the codec. */
  auto_codec: boolean;
  /** Why it picked the current codec, in Auto. */
  codec_reason: string | null;
  /** The bitrate cap across all viewers (null: none). */
  limit_bps: number | null;
}
