/**
 * VeilClient — WebTransport-based streaming client.
 *
 * Connects directly to the Veil server over QUIC/HTTP3 (no SDP, no ICE).
 * Video is decoded via the WebCodecs API and rendered to a Canvas element.
 * Input events are sent as QUIC datagrams for ultra-low latency.
 */

import { VeilVideoDecoder } from './VideoDecoder';
import { AudioPlayer } from './AudioPlayer';
import { InputManager } from './InputManager';
import {
  VeilClientConfig,
  ConnectionState,
  ConnectionStats,
  VeilEventMap,
  InputEvent,
  CursorUpdate,
  GamepadInfo,
  WindowInfo,
  EncoderInfo,
  ServerInfo,
  ServerStats,
  StreamParamsRequest,
  GamepadStatus,
  MouseCaptureState,
  CompositorState,
  ViewerList,
  VideoCodecName,
  GamepadType,
  DecodeLevel,
  DecodeSupport,
  NetworkStatus,
} from './types';
import {
  DEFAULT_SERVER_URL,
  DEFAULT_QUIC_PORT,
  DEFAULT_VIDEO_CODEC,
  DEFAULT_WIDTH,
  DEFAULT_HEIGHT,
  DEFAULT_FRAMERATE,
  STATS_UPDATE_INTERVAL,
  MSG_HELLO,
  MSG_SERVER_INFO,
  MSG_SERVER_STATS,
  MSG_GAMEPAD_STATUS,
  MSG_COMPOSITOR_STATE,
  MSG_VIEWERS,
  INPUT_SET_PARAMS,
  INPUT_SWITCH_GPU,
  MSG_VIDEO,
  MSG_AUDIO,
  MSG_CTRL_DISPLAY,
  MSG_CTRL_CURSOR,
  MSG_CTRL_POINTER_LOCK,
  MSG_CTRL_POINTER_RELEASE,
  FRAME_KEY,
  CODEC_H264,
  CODEC_H265,
  CODEC_AV1,
  INPUT_KEY_DOWN,
  INPUT_KEY_UP,
  INPUT_MOUSE_MOVE,
  INPUT_MOUSE_DOWN,
  INPUT_MOUSE_UP,
  INPUT_SCROLL,
  INPUT_GAMEPAD_STATE,
  MSG_GAMEPAD_RUMBLE,
  MSG_NETWORK,
  INPUT_DECODE_CAPS,
  H265_CODEC_STRING,
  AV1_CODEC_STRING,
  INPUT_REQ_KEYFRAME,
  INPUT_POINTER_ENTER,
  INPUT_POINTER_LEAVE,
  INPUT_PING,
  INPUT_GAMEPAD_AVAILABLE,
  INPUT_GAMEPAD_UNAVAILABLE,
  MSG_CTRL_PONG,
  MSG_WINDOW_LIST,
  INPUT_FOCUS_WINDOW,
  SCAN_CODE_MAP,
  MOUSE_BUTTON_MAP,
  GAMEPAD_DEADZONE,
  GAMEPAD_POLL_INTERVAL,
  GAMEPAD_RESEND_INTERVAL,
  h264CodecString,
} from './constants';

// ─── WebTransport type declarations (supplement lib.dom.d.ts if needed) ────

declare class WebTransport {
  constructor(url: string, options?: WebTransportOptions);
  readonly ready: Promise<void>;
  readonly closed: Promise<WebTransportCloseInfo>;
  readonly incomingUnidirectionalStreams: ReadableStream<ReadableStream<Uint8Array>>;
  readonly datagrams: WebTransportDatagramDuplexStream;
  close(info?: WebTransportCloseInfo): void;
}

interface WebTransportOptions {
  serverCertificateHashes?: WebTransportHash[];
  allowPooling?: boolean;
  requireUnreliable?: boolean;
}

interface WebTransportHash {
  algorithm: string;
  value: BufferSource;
}

interface WebTransportCloseInfo {
  closeCode?: number;
  reason?: string;
}

interface WebTransportDatagramDuplexStream {
  readonly readable: ReadableStream<Uint8Array>;
  readonly writable: WritableStream<Uint8Array>;
}

// ─── Main client class ───────────────────────────────────────────────────────

export class VeilClient {
  private cfg: Required<Omit<VeilClientConfig, 'audioPlayer'>>;
  private readonly audioPlayer: AudioPlayer;
  private state: ConnectionState = ConnectionState.Disconnected;

  private transport: WebTransport | null = null;
  private datagramWriter: WritableStreamDefaultWriter<Uint8Array> | null = null;
  private videoDecoder: VeilVideoDecoder | null = null;
  private inputManager: InputManager | null = null;
  private canvas: HTMLCanvasElement | null = null;
  private ctx2d: CanvasRenderingContext2D | null = null;

  private statsInterval: ReturnType<typeof setInterval> | null = null;
  private gamepadInterval: ReturnType<typeof setInterval> | null = null;
  /** Re-sends decode support and watches for the decoder falling behind. */
  private decodeInterval: ReturnType<typeof setInterval> | null = null;
  private decodeSupport: DecodeSupport | null = null;
  /** The stream size decode support was probed at. */
  private decodeProbedFor = '';
  /** Consecutive seconds the decoder has had a backlog. */
  private decodeBacklogSecs = 0;
  private decodeTicks = 0;
  // Stores a deep snapshot of button/axis state (NOT a Gamepad reference).
  // Chrome returns the same live Gamepad object on every getGamepads() call,
  // so storing the object reference and comparing against it would always
  // compare the current state against itself → no events ever sent.
  /** Per browser pad: snapshot sequence, last snapshot sent, and when. */
  private gamepads = new Map<number, { seq: number; last: string | null; sentAt: number }>();
  /** Per browser pad: which controller to emulate, if not detected. */
  private gamepadTypes = new Map<number, GamepadType>();
  /** Rumble that outlasts the browser's 5 s limit is replayed by this timer. */
  private rumbleTimers = new Map<number, ReturnType<typeof setTimeout>>();
  private abortController: AbortController | null = null;

  // Stats counters.
  private framesDecoded = 0;
  private framesDropped = 0;
  private bytesVideo = 0;
  private bytesAudio = 0;
  private audioFramesDecoded = 0;
  private lastStatsAt = Date.now();
  private serverCodec: VideoCodecName = 'h264';
  private serverWidth = 0;
  private serverHeight = 0;
  private serverFps = 0;

  private listeners = new Map<keyof VeilEventMap, Set<(d: any) => void>>();
  private lastCursorObjectUrl: string | null = null;

  // Latency tracking (ping/pong).
  private rttMs = 0;
  private pendingPings = new Map<number, number>(); // ts → performance.now() when sent

  // Encoder info and decode latency tracking.
  private encoderInfo: EncoderInfo | null = null;
  private serverInfo: ServerInfo | null = null;

  // Mouse capture. While captured, the browser freezes and hides its cursor,
  // so we track a virtual one in stream pixels and send absolute positions,
  // unless the app has locked the pointer, in which case we send deltas.
  private cursorX = 0;
  private cursorY = 0;
  private appPointerLocked = false;
  /** Swallow the mouseup of the click that captured the mouse. */
  private suppressMouseUp = -1;
  private onPointerLockChange: (() => void) | null = null;
  /** Codec, size and codec string the decoder was configured with. */
  private decoderConfigKey = '';
  private decodeLatencyMs = 0;
  private frameSubmitTimes = new Map<number, number>(); // timestampUs → performance.now()
  private lastVideoPtsMs = -1;

  constructor(config: VeilClientConfig) {
    this.audioPlayer = config.audioPlayer ?? new AudioPlayer();
    this.cfg = {
      serverUrl: config.serverUrl ?? DEFAULT_SERVER_URL,
      certFingerprint: config.certFingerprint ?? '',
      certHashUrl: config.certHashUrl ?? '',
      videoCodec: config.videoCodec ?? DEFAULT_VIDEO_CODEC,
      width: config.width ?? DEFAULT_WIDTH,
      height: config.height ?? DEFAULT_HEIGHT,
      framerate: config.framerate ?? DEFAULT_FRAMERATE,
      debug: config.debug ?? false,
      captureKeyboard: config.captureKeyboard !== false,
      captureMouse: config.captureMouse !== false,
      captureGamepad: config.captureGamepad !== false,
      pointerLock: config.pointerLock ?? false,
    };
  }

  // ─── Public API ────────────────────────────────────────────────────────────

  /**
   * Connect to the Veil server and begin rendering to `canvas`.
   *
   * The canvas will be sized to match the server's announced resolution
   * after the first hello message is received.
   */
  async connect(canvas: HTMLCanvasElement): Promise<void> {
    if (this.state !== ConnectionState.Disconnected) {
      throw new Error('Already connected or connecting');
    }

    if (typeof WebTransport === 'undefined') {
      throw new Error(
        'WebTransport is not supported in this browser. ' +
        'Please use Chrome 97+ (desktop or Android). ' +
        'Safari and Firefox do not support WebTransport.'
      );
    }

    this.canvas = canvas;
    this.ctx2d = canvas.getContext('2d');
    if (!this.ctx2d) throw new Error('Could not get 2D canvas context');

    this.setState(ConnectionState.Connecting);
    this.abortController = new AbortController();

    try {
      // ── Resolve TLS certificate fingerprint ─────────────────────────────
      const fingerprint = await this.resolveCertFingerprint();

      // When the page is served over HTTPS, the browser blocks mixed-content
      // fetches to HTTP endpoints, so the cert-hash fetch will have failed.
      // Without the fingerprint, Chrome cannot verify the self-signed QUIC
      // cert and the handshake will fail.  Surface a clear error now rather
      // than letting it fail opaquely as "Opening handshake failed".
      if (!fingerprint && typeof window !== 'undefined' && window.location?.protocol === 'https:') {
        const hashPort = parseInt(new URL(this.cfg.serverUrl).port || String(DEFAULT_QUIC_PORT)) + 1;
        throw new Error(
          `TLS certificate fingerprint unavailable (required for HTTPS pages). ` +
          `Fix: configure your reverse proxy to forward "/cert-hash" → ` +
          `http://server:${hashPort}/cert-hash. ` +
          `Or paste the fingerprint from the server startup logs into the ` +
          `"Cert fingerprint" field above.`
        );
      }

      // ── Open WebTransport connection ─────────────────────────────────────
      const wtOptions: WebTransportOptions = {};
      if (fingerprint) {
        wtOptions.serverCertificateHashes = [
          { algorithm: 'sha-256', value: hexToBuffer(fingerprint) },
        ];
      }

      this.log('Opening WebTransport to', this.cfg.serverUrl);
      this.transport = new WebTransport(this.cfg.serverUrl, wtOptions);
      await this.transport.ready;
      this.log('WebTransport connected');
      // A shared player may still hold audio from the previous connection.
      this.audioPlayer.reset();

      // ── Open datagram writer for input ──────────────────────────────────
      this.datagramWriter = this.transport.datagrams.writable.getWriter();

      // ── Start reading incoming streams (video / audio / control) ────────
      this.readIncomingStreams(); // fire-and-forget, runs until close

      // ── Start reading incoming datagrams (pong / low-latency ctrl) ──────
      this.readIncomingDatagrams(); // fire-and-forget, runs until close

      // ── Set up input capture ─────────────────────────────────────────────
      this.setupInput(canvas);

      // ── Stats collection ─────────────────────────────────────────────────
      this.statsInterval = setInterval(() => this.emitStats(), STATS_UPDATE_INTERVAL);
      this.decodeInterval = setInterval(() => this.decodeTick(), 1000);

      // ── Detect transport close ───────────────────────────────────────────
      this.transport.closed.then(() => {
        this.log('WebTransport closed');
        this.handleClose();
      }).catch(() => this.handleClose());

    } catch (err) {
      this.setState(ConnectionState.Failed);
      this.emit('error', err instanceof Error ? err : new Error(String(err)));
      throw err;
    }
  }

  /** Disconnect and release all resources. */
  async disconnect(): Promise<void> {
    this.log('Disconnecting');
    this.abortController?.abort();
    this.cleanup();
    this.transport?.close({ closeCode: 0, reason: 'client disconnect' });
    this.transport = null;
    this.setState(ConnectionState.Disconnected);
    this.emit('disconnected', undefined);
  }

  /** Ask the server to send an IDR keyframe immediately. */
  async requestKeyframe(): Promise<void> {
    await this.sendDatagram(new Uint8Array([INPUT_REQ_KEYFRAME]));
  }

  /**
   * Change encoding settings on the server. It rebuilds its encoder (and
   * resizes the display for size/framerate changes), then sends a new
   * `serverinfo`.
   */
  setStreamParams(req: StreamParamsRequest): void {
    const json = new TextEncoder().encode(JSON.stringify(req));
    const buf = new Uint8Array(1 + json.length);
    buf[0] = INPUT_SET_PARAMS;
    buf.set(json, 1);
    this.sendDatagram(buf);
  }

  /**
   * Restart the server on another GPU (one of `serverinfo.gpus`). The
   * connection drops while it restarts; reconnect afterwards.
   */
  switchGpu(gpu: string): void {
    const name = new TextEncoder().encode(gpu);
    const buf = new Uint8Array(1 + name.length);
    buf[0] = INPUT_SWITCH_GPU;
    buf.set(name, 1);
    this.sendDatagram(buf);
  }

  /**
   * Start playing the stream's audio. Browsers only allow this from a user
   * gesture (click or key press) the first time.
   */
  enableAudio(): Promise<void> {
    return this.audioPlayer.enable();
  }

  /** Stop audio and release the output device. */
  disableAudio(): Promise<void> {
    return this.audioPlayer.disable();
  }

  setVolume(volume: number): void {
    this.audioPlayer.setVolume(volume);
  }

  /**
   * Choose which controller the server emulates for a pad: 'auto' (the
   * default) emulates a DualSense for PlayStation pads and an Xbox 360 pad
   * for everything else. Changing it replaces the virtual device.
   */
  setGamepadType(index: number, type: GamepadType): void {
    if (type === 'auto') this.gamepadTypes.delete(index);
    else this.gamepadTypes.set(index, type);
  }

  /** What this browser decodes, as last reported to the server. */
  getDecodeSupport(): DecodeSupport | null {
    return this.decodeSupport;
  }

  /**
   * Find out which codecs this browser decodes at the stream's size, and in
   * hardware or software, and tell the server (its Auto codec uses this).
   */
  private async probeDecodeSupport(width: number, height: number): Promise<void> {
    const key = `${width}x${height}`;
    this.decodeProbedFor = key;
    const level = async (codec: string): Promise<DecodeLevel> => {
      if (typeof VideoDecoder === 'undefined') return 'none';
      for (const [accel, result] of [['prefer-hardware', 'hardware'], ['prefer-software', 'software']] as const) {
        try {
          const r = await VideoDecoder.isConfigSupported({
            codec, codedWidth: width, codedHeight: height, hardwareAcceleration: accel, optimizeForLatency: true,
          });
          if (r.supported) return result;
        } catch { /* unsupported */ }
      }
      return 'none';
    };
    const [h264, h265, av1] = await Promise.all([
      level(h264CodecString(66, 51)), level(H265_CODEC_STRING), level(AV1_CODEC_STRING),
    ]);
    if (this.decodeProbedFor !== key) return; // superseded
    this.decodeSupport = { h264, h265, av1, struggling: this.decodeSupport?.struggling ?? [] };
    this.log('Decode support:', this.decodeSupport);
    this.emit('decodesupport', this.decodeSupport);
    this.sendDecodeSupport();
  }

  private sendDecodeSupport(): void {
    if (!this.decodeSupport) return;
    const json = new TextEncoder().encode(JSON.stringify(this.decodeSupport));
    const buf = new Uint8Array(1 + json.length);
    buf[0] = INPUT_DECODE_CAPS;
    buf.set(json, 1);
    this.sendDatagram(buf);
  }

  /**
   * Every second: if the decoder has had a backlog for a few seconds, mark
   * the codec as one this browser can't keep up with; and every few seconds,
   * re-send decode support, since datagrams can be lost.
   */
  private decodeTick(): void {
    const queued = this.videoDecoder?.decodeQueueSize ?? 0;
    this.decodeBacklogSecs = queued >= DECODE_BACKLOG_FRAMES ? this.decodeBacklogSecs + 1 : 0;
    if (this.decodeSupport && this.decodeBacklogSecs >= DECODE_BACKLOG_SECS
        && !this.decodeSupport.struggling.includes(this.serverCodec)) {
      this.decodeSupport = { ...this.decodeSupport, struggling: [...this.decodeSupport.struggling, this.serverCodec] };
      this.log(`Decoder can't keep up with ${this.serverCodec}`);
      this.emit('decodesupport', this.decodeSupport);
      this.sendDecodeSupport();
      this.decodeBacklogSecs = 0;
    }
    if (++this.decodeTicks % 5 === 0) this.sendDecodeSupport();
  }

  /** The controller the server is asked to emulate for a pad. */
  getGamepadLayout(pad: Gamepad): 'xbox' | 'playstation' {
    const type = this.gamepadTypes.get(pad.index) ?? 'auto';
    return type === 'auto' ? detectGamepadLayout(pad.id) : type;
  }

  setMuted(muted: boolean): void {
    this.audioPlayer.setMuted(muted);
  }

  /** The latest `serverinfo`, if any has arrived. */
  getServerInfo(): ServerInfo | null {
    return this.serverInfo;
  }

  /** Raise the window with the given ID to the top and give it keyboard focus. */
  focusWindow(windowId: number): void {
    const buf = new Uint8Array(5);
    const view = new DataView(buf.buffer);
    buf[0] = INPUT_FOCUS_WINDOW;
    view.setUint32(1, windowId, false);
    this.sendDatagram(buf);
  }

  /** Get the current connection state. */
  getState(): ConnectionState {
    return this.state;
  }

  /** Get current stats snapshot. */
  getStats(): ConnectionStats {
    return this.buildStats();
  }

  on<K extends keyof VeilEventMap>(event: K, listener: (d: VeilEventMap[K]) => void): void {
    if (!this.listeners.has(event)) this.listeners.set(event, new Set());
    this.listeners.get(event)!.add(listener as any);
  }

  off<K extends keyof VeilEventMap>(event: K, listener: (d: VeilEventMap[K]) => void): void {
    this.listeners.get(event)?.delete(listener as any);
  }

  // ─── Stream reader ────────────────────────────────────────────────────────

  private async readIncomingStreams(): Promise<void> {
    if (!this.transport) return;
    const reader = this.transport.incomingUnidirectionalStreams.getReader();

    // Each stream carries one message. Streams are read concurrently, but
    // handled in the order they arrived: video frames must reach the decoder
    // in order, and a small stream can finish before a larger one opened
    // earlier.
    let previous: Promise<void> = Promise.resolve();
    try {
      while (true) {
        const { value: stream, done } = await reader.read();
        if (done) break;
        const data = readFullStream(stream);
        data.catch(() => {}); // Reported when its turn comes in the chain.
        previous = previous
          .then(async () => this.processMessage(await data))
          .catch((e) => this.log('Stream processing error:', e));
      }
    } catch (e) {
      this.log('Incoming stream reader ended:', e);
    } finally {
      reader.releaseLock();
    }
  }

  private async readIncomingDatagrams(): Promise<void> {
    if (!this.transport) return;
    const reader = this.transport.datagrams.readable.getReader();
    try {
      while (true) {
        const { value, done } = await reader.read();
        if (done) break;
        if (value.length === 0) continue;
        const msgType = value[0];
        const payload = value.subarray(1);
        if (msgType === MSG_CTRL_PONG) {
          this.handleCtrlPong(payload);
        } else if (msgType === MSG_AUDIO) {
          // Audio packets fit in datagrams, so that's how they usually come.
          this.handleAudio(payload);
        }
      }
    } catch (e) {
      this.log('Incoming datagram reader ended:', e);
    } finally {
      reader.releaseLock();
    }
  }

  private processMessage(data: Uint8Array): void {
    if (data.length === 0) return;

    const msgType = data[0];
    const payload = data.subarray(1);

    switch (msgType) {
      case MSG_HELLO:
        this.handleHello(payload);
        break;
      case MSG_VIDEO:
        this.handleVideo(payload);
        break;
      case MSG_AUDIO:
        this.handleAudio(payload);
        break;
      case MSG_CTRL_DISPLAY:
        this.handleCtrlDisplay(payload);
        break;
      case MSG_CTRL_CURSOR:
        this.handleCtrlCursor(payload);
        break;
      case MSG_CTRL_POINTER_LOCK:
        this.handleCtrlPointerLock(payload);
        break;
      case MSG_CTRL_POINTER_RELEASE:
        this.handlePointerRelease();
        break;
      case MSG_WINDOW_LIST:
        this.handleWindowList(payload);
        break;
      case MSG_SERVER_INFO:
        this.serverInfo = JSON.parse(new TextDecoder().decode(payload)) as ServerInfo;
        this.emit('serverinfo', this.serverInfo);
        break;
      case MSG_SERVER_STATS:
        this.emit('serverstats', JSON.parse(new TextDecoder().decode(payload)) as ServerStats);
        break;
      case MSG_GAMEPAD_STATUS:
        this.emit('gamepadstatus', JSON.parse(new TextDecoder().decode(payload)) as GamepadStatus[]);
        break;
      case MSG_COMPOSITOR_STATE:
        this.emit('compositorstate', JSON.parse(new TextDecoder().decode(payload)) as CompositorState);
        break;
      case MSG_GAMEPAD_RUMBLE:
        this.handleGamepadRumble(payload);
        break;
      case MSG_NETWORK:
        this.emit('network', JSON.parse(new TextDecoder().decode(payload)) as NetworkStatus);
        break;
      case MSG_VIEWERS:
        this.emit('viewers', JSON.parse(new TextDecoder().decode(payload)) as ViewerList);
        break;
      default:
        this.log('Unknown message type:', msgType.toString(16));
    }
  }

  // ─── Message handlers ─────────────────────────────────────────────────────

  private async handleHello(payload: Uint8Array): Promise<void> {
    // A new session (or server restart) restarts the timestamps.
    this.lastVideoPtsMs = -1;
    if (payload.length < 7) return;
    const view = new DataView(payload.buffer, payload.byteOffset);
    const codecByte = view.getUint8(0);
    const width = view.getUint16(1);
    const height = view.getUint16(3);
    const fps = view.getUint16(5);

    this.serverCodec = codecByte === CODEC_H265 ? 'h265' : codecByte === CODEC_AV1 ? 'av1' : 'h264';
    this.serverWidth = width;
    this.serverHeight = height;
    this.serverFps = fps;

    this.log('Hello received:', { codec: this.serverCodec, width, height, fps });

    // Parse extended encoder info (present in hello payload length >= 12)
    let encoderInfo: EncoderInfo | null = null;
    if (payload.length >= 12) {
      const profileIdc = view.getUint8(7);
      const levelIdc = view.getUint8(8);
      const gopSize = view.getUint16(9);
      const inFlight = view.getUint8(11);
      const profileName = profileIdc === 66 ? 'Baseline' : profileIdc === 77 ? 'Main' : 'High';
      const levelStr = `${Math.floor(levelIdc / 10)}.${levelIdc % 10}`;
      encoderInfo = {
        codec: { h264: 'H.264', h265: 'H.265', av1: 'AV1' }[this.serverCodec],
        profile: profileName,
        level: levelStr,
        gopSize,
        inFlight,
      };
    }
    this.encoderInfo = encoderInfo;

    // Size the canvas.
    if (this.canvas) {
      this.canvas.width = width;
      this.canvas.height = height;
    }

    // Build a proper codec string based on profile/level for H264.
    let codecString: string | undefined;
    if (this.serverCodec === 'h264' && this.encoderInfo) {
      const profileIdc = payload.length >= 12 ? view.getUint8(7) : 100;
      const levelIdc   = payload.length >= 12 ? view.getUint8(8) : 51;
      codecString = h264CodecString(profileIdc, levelIdc);
      this.log('H264 codec string:', codecString);
    }

    // The server re-sends the hello whenever it rebuilds its encoder. If the
    // decoder config is unchanged, keep the decoder and just wait for the new
    // stream's first keyframe.
    const configKey = `${this.serverCodec}|${width}x${height}|${codecString ?? ''}`;
    if (configKey === this.decoderConfigKey && this.videoDecoder?.isInitialized) {
      this.videoDecoder.resync();
      return;
    }
    this.decoderConfigKey = configKey;
    // Frames sent to the old decoder will never come out of this one.
    this.frameSubmitTimes.clear();
    this.decodeBacklogSecs = 0;
    if (this.decodeProbedFor !== `${width}x${height}`) {
      void this.probeDecodeSupport(width, height);
    }

    // Initialize the video decoder.
    const firstHello = this.state !== ConnectionState.Connected;
    this.videoDecoder?.close();
    this.videoDecoder = new VeilVideoDecoder({
      codec: this.serverCodec,
      width,
      height,
      codecString,
      onFrame: (frame) => this.renderFrame(frame),
      onError: (err) => this.log('Decode error:', err),
      onNeedKeyframe: () => { void this.requestKeyframe(); },
    });

    try {
      await this.videoDecoder.initialize();
      if (firstHello) {
        this.setState(ConnectionState.Connected);
        this.emit('connected', undefined);
      }
      this.emit('videostart', undefined);
      // Give the canvas keyboard focus so keydown/keyup events fire immediately.
      this.canvas?.focus();
      // Ask the server for an IDR immediately so the decoder doesn't have to
      // wait until the natural end of the current GOP before showing a frame.
      await this.requestKeyframe();
    } catch (err) {
      this.emit('error', err instanceof Error ? err : new Error(String(err)));
      this.setState(ConnectionState.Failed);
    }
  }

  private handleVideo(payload: Uint8Array): void {
    if (payload.length < 9) return;
    const view = new DataView(payload.buffer, payload.byteOffset);
    const frameType = view.getUint8(0);
    const ptsMs = view.getUint32(5);
    const frameData = payload.subarray(9);

    const isKeyframe = frameType === FRAME_KEY;
    const timestampUs = ptsMs * 1000;

    // Frames are dispatched in arrival order, so this only happens if the
    // server reorders. A delta frame out of order can't be decoded correctly.
    if (!isKeyframe && ptsMs <= this.lastVideoPtsMs) {
      this.log('Out-of-order video frame; waiting for keyframe');
      this.videoDecoder?.resync();
      void this.requestKeyframe();
      return;
    }
    this.lastVideoPtsMs = ptsMs;

    this.bytesVideo += frameData.byteLength;

    // Record when this frame was submitted to the decoder for latency tracking.
    this.frameSubmitTimes.set(timestampUs, performance.now());
    // Cap map size to avoid unbounded growth.
    if (this.frameSubmitTimes.size > 120) {
      const oldest = this.frameSubmitTimes.keys().next().value;
      if (oldest !== undefined) this.frameSubmitTimes.delete(oldest);
    }

    this.videoDecoder?.decode(frameData.buffer.slice(
      frameData.byteOffset,
      frameData.byteOffset + frameData.byteLength
    ), isKeyframe, timestampUs);
  }

  private handleAudio(payload: Uint8Array): void {
    if (payload.length < 8) return;
    // seq = payload[0..4], pts_ms = payload[4..8], data = payload[8..]
    const view = new DataView(payload.buffer, payload.byteOffset);
    const ptsMs = view.getUint32(4);
    const audioData = payload.subarray(8);
    this.bytesAudio += audioData.byteLength;
    this.audioFramesDecoded++;
    this.audioPlayer.push(ptsMs, audioData);
  }

  private handleCtrlDisplay(payload: Uint8Array): void {
    if (payload.length < 6) return;
    const view = new DataView(payload.buffer, payload.byteOffset);
    const width = view.getUint16(0);
    const height = view.getUint16(2);
    const framerate = view.getUint16(4);
    this.serverWidth = width;
    this.serverHeight = height;
    this.serverFps = framerate;
    if (this.canvas) {
      this.canvas.width = width;
      this.canvas.height = height;
    }
    this.emit('displaychange', { width, height, framerate });
  }

  private handleCtrlCursor(payload: Uint8Array): void {
    if (payload.length < 5) return;
    const view = new DataView(payload.buffer, payload.byteOffset);
    const hotspotX = view.getUint16(0);
    const hotspotY = view.getUint16(2);
    const kind = view.getUint8(4);

    // Revoke the previous object URL to avoid a memory leak.
    if (this.lastCursorObjectUrl) {
      URL.revokeObjectURL(this.lastCursorObjectUrl);
      this.lastCursorObjectUrl = null;
    }

    const cursor: CursorUpdate = { hotspotX, hotspotY };

    if (kind === 0x01 && payload.length >= 9) {
      // PNG image
      const imgLen = view.getUint32(5);
      const imgBytes = payload.subarray(9, 9 + imgLen);
      const blob = new Blob([imgBytes], { type: 'image/png' });
      cursor.image = URL.createObjectURL(blob);
      this.lastCursorObjectUrl = cursor.image;
    } else if (kind === 0x02 && payload.length >= 6) {
      // Named icon
      const nameLen = view.getUint8(5);
      const nameBytes = payload.subarray(6, 6 + nameLen);
      cursor.icon = new TextDecoder().decode(nameBytes);
    }
    // kind === 0x00 → hidden cursor; cursor has neither image nor icon.

    this.emit('cursor', cursor);
  }

  private handleCtrlPong(payload: Uint8Array): void {
    if (payload.length < 8) return;
    const view = new DataView(payload.buffer, payload.byteOffset);
    const ts = Number(view.getBigUint64(0, false));
    const sentAt = this.pendingPings.get(ts);
    if (sentAt !== undefined) {
      this.rttMs = Math.round(performance.now() - sentAt);
      this.pendingPings.delete(ts);
    }
  }

  private sendPing(): void {
    const ts = Date.now();
    const buf = new Uint8Array(9);
    const view = new DataView(buf.buffer);
    buf[0] = INPUT_PING;
    view.setBigUint64(1, BigInt(ts), false);
    this.pendingPings.set(ts, performance.now());
    // Evict stale pings (older than 5 s) to prevent unbounded growth.
    for (const [k] of this.pendingPings) {
      if (ts - k > 5000) this.pendingPings.delete(k);
    }
    this.sendDatagram(buf);
  }

  private handleWindowList(payload: Uint8Array): void {
    if (payload.length < 1) return;
    const count = payload[0];
    const windows: WindowInfo[] = [];
    let offset = 1;
    for (let i = 0; i < count && offset < payload.length; i++) {
      if (offset + 6 > payload.length) break;
      const view = new DataView(payload.buffer, payload.byteOffset + offset);
      const id = view.getUint32(0);
      const active = view.getUint8(4) !== 0;
      const titleLen = view.getUint8(5);
      offset += 6;
      if (offset + titleLen > payload.length) break;
      const title = new TextDecoder().decode(payload.subarray(offset, offset + titleLen));
      offset += titleLen;
      if (offset + 1 > payload.length) break;
      const appIdLen = payload[offset];
      offset += 1;
      if (offset + appIdLen > payload.length) break;
      const appId = new TextDecoder().decode(payload.subarray(offset, offset + appIdLen));
      offset += appIdLen;
      windows.push({ id, title, appId, active });
    }
    this.emit('windowlist', windows);
  }

  /** A game asked for rumble on one of our pads. */
  private handleGamepadRumble(payload: Uint8Array): void {
    if (payload.length < 7) return;
    const view = new DataView(payload.buffer, payload.byteOffset);
    const index = view.getUint8(0);
    const strong = view.getUint16(1) / 0xffff;
    const weak = view.getUint16(3) / 0xffff;
    const durationMs = view.getUint16(5);
    this.emit('gamepadrumble', { index, strong, weak, durationMs });

    const timer = this.rumbleTimers.get(index);
    if (timer !== undefined) clearTimeout(timer);
    this.rumbleTimers.delete(index);

    const actuator = navigator.getGamepads()[index]?.vibrationActuator;
    if (!actuator) return;
    if (strong === 0 && weak === 0) {
      void actuator.reset().catch(() => {});
      return;
    }
    // Browsers cap one effect at 5 s; replay to cover longer (or "until
    // stopped") requests. A stop message cancels the timer.
    const play = (remaining: number) => {
      const chunk = Math.min(remaining, RUMBLE_MAX_EFFECT_MS);
      void actuator
        .playEffect('dual-rumble', { startDelay: 0, duration: chunk, strongMagnitude: strong, weakMagnitude: weak })
        .catch(() => {});
      if (remaining > chunk) {
        this.rumbleTimers.set(index, setTimeout(() => play(remaining - chunk), chunk - 50));
      } else {
        this.rumbleTimers.delete(index);
      }
    };
    play(durationMs);
  }

  private handleCtrlPointerLock(payload: Uint8Array): void {
    if (payload.length < 8) return;
    const view = new DataView(payload.buffer, payload.byteOffset);
    const x = view.getFloat32(0);
    const y = view.getFloat32(4);
    this.appPointerLocked = true;
    // The pointer stays where the app locked it.
    this.cursorX = x;
    this.cursorY = y;
    this.emit('pointerlock', { x, y });
    this.emitMouseCapture();
  }

  private handlePointerRelease(): void {
    this.appPointerLocked = false;
    this.emit('pointerrelease', undefined);
    this.emitMouseCapture();
  }

  // ─── Mouse capture ────────────────────────────────────────────────────────

  /**
   * Lock the browser's pointer to the stream. The cursor then moves
   * absolutely within the stream (draw it yourself; see `getCursorPosition`),
   * or sends relative motion while the app has locked the pointer. Must be
   * called from a user gesture (click or key press). Esc always releases.
   */
  captureMouse(): void {
    if (!this.canvas || this.isMouseCaptured()) return;
    // Raw, unaccelerated movement where supported; fall back otherwise.
    const fail = (err: unknown) =>
      console.warn('[VeilClient] Mouse capture refused by the browser:', err instanceof Error ? err.message : err);
    const req = this.canvas.requestPointerLock({ unadjustedMovement: true } as any) as unknown;
    if (req instanceof Promise) {
      req.catch((err: unknown) => {
        if (!this.canvas || (err instanceof DOMException && err.name === 'WrongDocumentError')) {
          // Retrying won't help (e.g. the browser window isn't focused).
          fail(err);
          return;
        }
        const retry = this.canvas.requestPointerLock() as unknown;
        if (retry instanceof Promise) retry.catch(fail);
      });
    }
  }

  releaseMouse(): void {
    if (this.isMouseCaptured()) document.exitPointerLock();
  }

  isMouseCaptured(): boolean {
    return !!this.canvas && document.pointerLockElement === this.canvas;
  }

  getMouseCapture(): MouseCaptureState {
    const captured = this.isMouseCaptured();
    return {
      captured,
      relative: captured && this.appPointerLocked,
      appWantsLock: this.appPointerLocked,
    };
  }

  /** The cursor position in stream pixels (the virtual one while captured). */
  getCursorPosition(): { x: number; y: number } {
    return { x: this.cursorX, y: this.cursorY };
  }

  private emitMouseCapture(): void {
    this.emit('mousecapture', this.getMouseCapture());
  }

  /** Map a mouse event to stream pixels. */
  private streamCoords(e: MouseEvent): { x: number; y: number } {
    if (this.isMouseCaptured()) return { x: this.cursorX, y: this.cursorY };
    const canvas = e.target as HTMLCanvasElement;
    const rect = canvas.getBoundingClientRect();
    return {
      x: (e.clientX - rect.left) * (canvas.width / rect.width),
      y: (e.clientY - rect.top) * (canvas.height / rect.height),
    };
  }

  // ─── Rendering ────────────────────────────────────────────────────────────

  private renderFrame(frame: VideoFrame): void {
    const submitTime = this.frameSubmitTimes.get(frame.timestamp);
    if (submitTime !== undefined) {
      this.decodeLatencyMs = Math.round(performance.now() - submitTime);
      this.frameSubmitTimes.delete(frame.timestamp);
    }
    if (this.ctx2d && this.canvas) {
      // Crop to the stream's size: a decoder may hand back the padded frame
      // (AV1 frames are coded at e.g. 1088 rows for a 1080-row stream).
      const sw = Math.min(frame.displayWidth, this.serverWidth || frame.displayWidth);
      const sh = Math.min(frame.displayHeight, this.serverHeight || frame.displayHeight);
      this.ctx2d.drawImage(frame, 0, 0, sw, sh, 0, 0, this.canvas.width, this.canvas.height);
    }
    this.framesDecoded++;
    frame.close();
  }

  // ─── Input capture ────────────────────────────────────────────────────────

  private setupInput(element: HTMLElement): void {
    if (this.cfg.captureKeyboard) {
      if (!element.hasAttribute('tabindex')) element.setAttribute('tabindex', '0');
      element.addEventListener('keydown', (e) => this.onKeyDown(e as KeyboardEvent));
      element.addEventListener('keyup', (e) => this.onKeyUp(e as KeyboardEvent));
    }
    if (this.cfg.captureMouse) {
      element.addEventListener('mouseenter', () => this.onPointerEnter());
      element.addEventListener('mouseleave', () => this.onPointerLeave());
      element.addEventListener('mousedown', (e) => this.onMouseDown(e as MouseEvent));
      element.addEventListener('mouseup', (e) => this.onMouseUp(e as MouseEvent));
      element.addEventListener('mousemove', (e) => this.onMouseMove(e as MouseEvent));
      element.addEventListener('wheel', (e) => this.onWheel(e as WheelEvent), { passive: false });
      element.addEventListener('contextmenu', (e) => e.preventDefault());

      this.onPointerLockChange = () => {
        if (this.isMouseCaptured()) {
          // The browser cursor disappears; the app still sees it as present.
          this.onPointerEnter();
        }
        this.emitMouseCapture();
      };
      document.addEventListener('pointerlockchange', this.onPointerLockChange);
    }
    if (this.cfg.captureGamepad) {
      this.gamepadInterval = setInterval(() => this.pollGamepads(), GAMEPAD_POLL_INTERVAL);
    }
  }

  private onPointerEnter(): void {
    this.sendDatagram(new Uint8Array([INPUT_POINTER_ENTER]));
  }

  private onPointerLeave(): void {
    this.sendDatagram(new Uint8Array([INPUT_POINTER_LEAVE]));
  }

  private onKeyDown(e: KeyboardEvent): void {
    e.preventDefault();
    const keycode = SCAN_CODE_MAP[e.code] ?? 0;
    if (keycode === 0) return; // unknown physical key — drop it
    const char = e.key.length === 1 ? e.key.codePointAt(0) ?? 0 : 0;
    const buf = new Uint8Array(9);
    const view = new DataView(buf.buffer);
    buf[0] = INPUT_KEY_DOWN;
    view.setUint32(1, keycode, false);
    view.setUint32(5, char, false);
    this.sendDatagram(buf);
  }

  private onKeyUp(e: KeyboardEvent): void {
    e.preventDefault();
    const keycode = SCAN_CODE_MAP[e.code] ?? 0;
    if (keycode === 0) return;
    const buf = new Uint8Array(5);
    const view = new DataView(buf.buffer);
    buf[0] = INPUT_KEY_UP;
    view.setUint32(1, keycode, false);
    this.sendDatagram(buf);
  }

  private onMouseDown(e: MouseEvent): void {
    e.preventDefault();
    if (this.cfg.pointerLock && !this.isMouseCaptured()) {
      // This click captures the mouse; don't pass it on.
      this.suppressMouseUp = e.button;
      this.captureMouse();
      return;
    }
    const { x, y } = this.streamCoords(e);
    const btn = MOUSE_BUTTON_MAP[e.button] ?? e.button;
    const buf = new Uint8Array(13); // type:1 + button:4 + x:4 + y:4
    const view = new DataView(buf.buffer);
    buf[0] = INPUT_MOUSE_DOWN;
    view.setUint32(1, btn, false);
    view.setFloat32(5, x, false);
    view.setFloat32(9, y, false);
    this.sendDatagram(buf);
  }

  private onMouseUp(e: MouseEvent): void {
    e.preventDefault();
    if (e.button === this.suppressMouseUp) {
      this.suppressMouseUp = -1;
      return;
    }
    const { x, y } = this.streamCoords(e);
    const btn = MOUSE_BUTTON_MAP[e.button] ?? e.button;
    const buf = new Uint8Array(13);
    const view = new DataView(buf.buffer);
    buf[0] = INPUT_MOUSE_UP;
    view.setUint32(1, btn, false);
    view.setFloat32(5, x, false);
    view.setFloat32(9, y, false);
    this.sendDatagram(buf);
  }

  private onMouseMove(e: MouseEvent): void {
    const canvas = e.target as HTMLCanvasElement;
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    let x: number, y: number, dx = 0, dy = 0;
    if (!this.isMouseCaptured()) {
      x = (e.clientX - rect.left) * scaleX;
      y = (e.clientY - rect.top) * scaleY;
      this.cursorX = x;
      this.cursorY = y;
    } else if (this.appPointerLocked) {
      // The app locked the pointer (mouselook): relative motion only. The
      // compositor ignores relative motion without an app lock.
      x = this.cursorX;
      y = this.cursorY;
      dx = e.movementX * scaleX;
      dy = e.movementY * scaleY;
    } else {
      // Captured, absolute: move the virtual cursor within the stream.
      this.cursorX = Math.min(canvas.width - 1, Math.max(0, this.cursorX + e.movementX * scaleX));
      this.cursorY = Math.min(canvas.height - 1, Math.max(0, this.cursorY + e.movementY * scaleY));
      x = this.cursorX;
      y = this.cursorY;
    }
    const buf = new Uint8Array(17);
    const view = new DataView(buf.buffer);
    buf[0] = INPUT_MOUSE_MOVE;
    view.setFloat32(1, x, false);
    view.setFloat32(5, y, false);
    view.setFloat32(9, dx, false);
    view.setFloat32(13, dy, false);
    this.sendDatagram(buf);
  }

  private onWheel(e: WheelEvent): void {
    e.preventDefault();
    const buf = new Uint8Array(9);
    const view = new DataView(buf.buffer);
    buf[0] = INPUT_SCROLL;
    view.setFloat32(1, e.deltaX, false);
    view.setFloat32(5, e.deltaY, false);
    this.sendDatagram(buf);
  }

  private pollGamepads(): void {
    const pads = navigator.getGamepads();
    const now = performance.now();
    const currentIndices = new Set<number>();

    for (const pad of pads) {
      if (!pad || pad.index > 0xff) continue;
      currentIndices.add(pad.index);

      let tracked = this.gamepads.get(pad.index);
      if (!tracked) {
        tracked = { seq: 0, last: null, sentAt: 0 };
        this.gamepads.set(pad.index, tracked);
        this.emit('gamepadconnected', { index: pad.index, id: pad.id });
        // Tell the server to create the virtual device now, before any input,
        // so Steam has time to see it appear.
        const layout = GAMEPAD_LAYOUT_WIRE[this.getGamepadLayout(pad)];
        this.sendDatagram(new Uint8Array([INPUT_GAMEPAD_AVAILABLE, pad.index, layout]));
      }

      // Send the whole state on every change, and again periodically, so a
      // lost datagram can't leave a button held or a stick deflected. The
      // pad type goes with it, for the same reason.
      const snapshot = gamepadSnapshot(pad);
      const layout = GAMEPAD_LAYOUT_WIRE[this.getGamepadLayout(pad)];
      const key = `${snapshot.key}|${layout}`;
      if (key !== tracked.last || now - tracked.sentAt >= GAMEPAD_RESEND_INTERVAL) {
        tracked.seq = (tracked.seq + 1) >>> 0;
        const buf = new Uint8Array(21);
        const view = new DataView(buf.buffer);
        buf[0] = INPUT_GAMEPAD_STATE;
        buf[1] = pad.index;
        view.setUint32(2, tracked.seq);
        buf.set(snapshot.bytes, 6);
        buf[20] = layout;
        this.sendDatagram(buf);
        tracked.last = key;
        tracked.sentAt = now;
      }
    }

    for (const index of [...this.gamepads.keys()]) {
      if (!currentIndices.has(index)) {
        this.gamepads.delete(index);
        this.emit('gamepaddisconnected', { index, id: '' });
        this.sendDatagram(new Uint8Array([INPUT_GAMEPAD_UNAVAILABLE, index]));
      }
    }
  }

  // ─── Datagram sender ──────────────────────────────────────────────────────

  private sendDatagram(data: Uint8Array): void {
    if (!this.datagramWriter) return;
    this.datagramWriter.write(data).catch((e) =>
      this.log('Datagram send failed:', e)
    );
  }

  // ─── Certificate resolution ───────────────────────────────────────────────

  private async resolveCertFingerprint(): Promise<string> {
    if (this.cfg.certFingerprint) return this.cfg.certFingerprint;

    const url = new URL(this.cfg.serverUrl);

    // Build a prioritised list of URLs to try.
    // 1. Explicit override from config.
    // 2. Same origin as the page (works when the page is served via an HTTPS
    //    reverse proxy that also forwards /cert-hash — avoids mixed-content
    //    blocks when the direct cert-hash server is plain HTTP).
    // 3. Direct HTTP endpoint: http://host:(QUIC_port+1)/cert-hash
    const directUrl = `http://${url.hostname}:${parseInt(url.port || String(DEFAULT_QUIC_PORT)) + 1}/cert-hash`;
    const candidates: string[] = [];

    if (this.cfg.certHashUrl) {
      candidates.push(this.cfg.certHashUrl);
    }

    // Only add the page-origin candidate when the page is served from the
    // server's host (i.e. by its reverse proxy) and it differs from the direct
    // URL. Another host's /cert-hash is for a different server.
    if (typeof window !== 'undefined' && window.location?.origin &&
        window.location.hostname === url.hostname) {
      const pageOriginUrl = `${window.location.origin}/cert-hash`;
      if (pageOriginUrl !== directUrl) {
        candidates.push(pageOriginUrl);
      }
    }

    candidates.push(directUrl);

    for (const hashUrl of candidates) {
      try {
        this.log('Fetching cert fingerprint from', hashUrl);
        const res = await fetch(hashUrl);
        if (!res.ok) continue;
        const json = await res.json() as { sha256_hex: string };
        if (json.sha256_hex) {
          console.info('[VeilClient] cert fingerprint resolved from', hashUrl);
          this.log('Cert fingerprint:', json.sha256_hex);
          return json.sha256_hex;
        }
      } catch {
        console.warn('[VeilClient] cert fingerprint fetch failed for', hashUrl);
        this.log('Cert fingerprint fetch failed for', hashUrl);
      }
    }

    console.warn('[VeilClient] Could not fetch cert fingerprint from any endpoint');
    return '';
  }

  // ─── Stats ────────────────────────────────────────────────────────────────

  private emitStats(): void {
    this.sendPing();
    // Re-request a keyframe every stats tick while the decoder is still waiting
    // for its first IDR.  Guards against the initial FIR being dropped (e.g.
    // when it races with video-pipeline creation on the server).
    if (this.videoDecoder?.isWaitingForKeyframe) {
      this.log('Decoder waiting for keyframe — retrying FIR');
      void this.requestKeyframe();
    }
    const stats = this.buildStats();
    this.emit('stats', stats);
  }

  private buildStats(): ConnectionStats {
    const now = Date.now();
    const dt = (now - this.lastStatsAt) / 1000;
    const videoBitrate = dt > 0 ? (this.bytesVideo * 8) / dt : 0;
    const audioBitrate = dt > 0 ? (this.bytesAudio * 8) / dt : 0;
    const fps = dt > 0 ? this.framesDecoded / dt : 0;

    // Reset counters.
    this.bytesVideo = 0;
    this.bytesAudio = 0;
    this.framesDecoded = 0;
    this.lastStatsAt = now;

    return {
      video: {
        fps: Math.round(fps),
        framesDecoded: this.videoDecoder?.framesDecoded ?? 0,
        framesDropped: this.videoDecoder?.framesDropped ?? 0,
        bitrateBps: videoBitrate,
        width: this.serverWidth,
        height: this.serverHeight,
        codec: this.serverCodec,
        latencyMs: 0,
        decodeQueueSize: this.videoDecoder?.decodeQueueSize ?? 0,
      },
      audio: {
        framesDecoded: this.audioFramesDecoded,
        bitrateBps: audioBitrate,
        codec: 'opus',
        player: this.audioPlayer.getStats(),
      },
      rttMs: this.rttMs,
      encoderInfo: this.encoderInfo,
      decodeLatencyMs: this.decodeLatencyMs,
    };
  }

  // ─── Lifecycle helpers ────────────────────────────────────────────────────

  private setState(s: ConnectionState): void {
    if (this.state !== s) {
      this.state = s;
      this.emit('statechange', s);
    }
  }

  private handleClose(): void {
    this.cleanup();
    this.setState(ConnectionState.Closed);
    this.emit('disconnected', undefined);
  }

  private cleanup(): void {
    this.releaseMouse();
    if (this.onPointerLockChange) {
      document.removeEventListener('pointerlockchange', this.onPointerLockChange);
      this.onPointerLockChange = null;
    }
    this.appPointerLocked = false;
    if (this.statsInterval !== null) {
      clearInterval(this.statsInterval);
      this.statsInterval = null;
    }
    if (this.gamepadInterval !== null) {
      clearInterval(this.gamepadInterval);
      this.gamepadInterval = null;
    }
    if (this.decodeInterval !== null) {
      clearInterval(this.decodeInterval);
      this.decodeInterval = null;
    }
    this.decodeSupport = null;
    this.decodeProbedFor = '';
    this.videoDecoder?.close();
    this.videoDecoder = null;
    this.decoderConfigKey = '';
    this.datagramWriter?.close().catch(() => {});
    this.datagramWriter = null;
    this.gamepads.clear();
    for (const timer of this.rumbleTimers.values()) clearTimeout(timer);
    this.rumbleTimers.clear();
    this.pendingPings.clear();
    this.frameSubmitTimes.clear();
    this.rttMs = 0;
  }

  private emit<K extends keyof VeilEventMap>(event: K, data: VeilEventMap[K]): void {
    this.listeners.get(event)?.forEach((fn) => {
      try { fn(data); } catch (e) { this.log('Listener error:', e); }
    });
  }

  private log(...args: unknown[]): void {
    if (this.cfg.debug) console.log('[VeilClient]', ...args);
  }
}

// ─── Utility functions ────────────────────────────────────────────────────────

/** Read all chunks from a QUIC unidirectional stream into one Uint8Array. */
async function readFullStream(stream: ReadableStream<Uint8Array>): Promise<Uint8Array> {
  const reader = stream.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;

  try {
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      chunks.push(value);
      total += value.byteLength;
    }
  } finally {
    reader.releaseLock();
  }

  const result = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    result.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return result;
}

/** Convert a lowercase hex string to an ArrayBuffer (for serverCertificateHashes). */
function hexToBuffer(hex: string): ArrayBuffer {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes.buffer;
}

// ─── Gamepad snapshots ──────────────────────────────────────────────────────

/** Browsers cap a single vibration effect at this length. */
const RUMBLE_MAX_EFFECT_MS = 5000;

/**
 * The wire form of a pad's state (buttons, triggers, sticks), as 14 bytes,
 * plus the same bytes as a string so changes can be detected cheaply.
 */
interface GamepadSnapshot {
  bytes: Uint8Array;
  key: string;
}

function gamepadSnapshot(pad: Gamepad): GamepadSnapshot {
  const bytes = new Uint8Array(14);
  const view = new DataView(bytes.buffer);
  let buttons = 0;
  for (let i = 0; i < Math.min(pad.buttons.length, 32); i++) {
    if (pad.buttons[i]?.pressed) buttons |= 1 << i;
  }
  view.setUint32(0, buttons >>> 0);
  bytes[4] = Math.round(Math.min(1, Math.max(0, pad.buttons[6]?.value ?? 0)) * 255);
  bytes[5] = Math.round(Math.min(1, Math.max(0, pad.buttons[7]?.value ?? 0)) * 255);
  const [lx, ly] = radialDeadzone(pad.axes[0] ?? 0, pad.axes[1] ?? 0);
  const [rx, ry] = radialDeadzone(pad.axes[2] ?? 0, pad.axes[3] ?? 0);
  [lx, ly, rx, ry].forEach((v, i) => view.setInt16(6 + 2 * i, Math.round(v * 32767)));
  return { bytes, key: bytes.join(',') };
}

/**
 * One round deadzone, rescaled so movement starts smoothly at its edge
 * rather than jumping. Games apply their own deadzone too; this only hides
 * stick drift.
 */
export function radialDeadzone(x: number, y: number, deadzone = GAMEPAD_DEADZONE): [number, number] {
  const magnitude = Math.hypot(x, y);
  if (magnitude <= deadzone) return [0, 0];
  const scaled = Math.min(1, (magnitude - deadzone) / (1 - deadzone));
  return [(x / magnitude) * scaled, (y / magnitude) * scaled];
}

/** The pad-type byte in gamepad messages. */
const GAMEPAD_LAYOUT_WIRE = { xbox: 0, playstation: 1 } as const;

/**
 * Guess a pad's type from the browser's ID, e.g. Chrome's "DualSense Wireless
 * Controller (STANDARD GAMEPAD Vendor: 054c Product: 0ce6)" or Firefox's
 * "054c-0ce6-DualSense Wireless Controller". Sony's vendor ID is 054c.
 */
export function detectGamepadLayout(id: string): 'xbox' | 'playstation' {
  return /vendor:\s*054c\b|^054c-|dualsense|dualshock|playstation/i.test(id) ? 'playstation' : 'xbox';
}

/** A decoder this many frames behind... */
const DECODE_BACKLOG_FRAMES = 8;
/** ...for this many seconds in a row can't keep up with the codec. */
const DECODE_BACKLOG_SECS = 3;
