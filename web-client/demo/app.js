// ../sdk/src/constants.ts
var DEFAULT_QUIC_PORT = 7801;
var DEFAULT_SERVER_URL = `https://localhost:${DEFAULT_QUIC_PORT}`;
var DEFAULT_VIDEO_CODEC = "h264";
var DEFAULT_WIDTH = 1280;
var DEFAULT_HEIGHT = 720;
var DEFAULT_FRAMERATE = 60;
var MSG_HELLO = 0;
var MSG_VIDEO = 1;
var MSG_AUDIO = 2;
var MSG_CTRL_DISPLAY = 16;
var MSG_CTRL_CURSOR = 17;
var MSG_CTRL_POINTER_LOCK = 18;
var MSG_CTRL_POINTER_RELEASE = 19;
var MSG_CTRL_PONG = 20;
var MSG_WINDOW_LIST = 21;
var MSG_SERVER_INFO = 22;
var MSG_SERVER_STATS = 23;
var MSG_GAMEPAD_STATUS = 24;
var MSG_COMPOSITOR_STATE = 25;
var MSG_VIEWERS = 26;
var MSG_GAMEPAD_RUMBLE = 27;
var FRAME_KEY = 0;
var CODEC_H265 = 2;
var CODEC_AV1 = 3;
var INPUT_KEY_DOWN = 32;
var INPUT_KEY_UP = 33;
var INPUT_MOUSE_MOVE = 34;
var INPUT_MOUSE_DOWN = 35;
var INPUT_MOUSE_UP = 36;
var INPUT_SCROLL = 37;
var INPUT_REQ_KEYFRAME = 40;
var INPUT_POINTER_ENTER = 41;
var INPUT_POINTER_LEAVE = 42;
var INPUT_PING = 43;
var INPUT_GAMEPAD_AVAILABLE = 44;
var INPUT_GAMEPAD_UNAVAILABLE = 45;
var INPUT_FOCUS_WINDOW = 46;
var INPUT_SET_PARAMS = 47;
var INPUT_SWITCH_GPU = 48;
var INPUT_GAMEPAD_STATE = 49;
var STATS_UPDATE_INTERVAL = 1e3;
var GAMEPAD_POLL_INTERVAL = 4;
var GAMEPAD_RESEND_INTERVAL = 100;
var GAMEPAD_DEADZONE = 0.05;
var H264_CODEC_STRING = "avc1.42E01F";
function h264CodecString(profileIdc, levelIdc) {
  let pp;
  let cc;
  if (profileIdc === 66) {
    pp = "42";
    cc = "E0";
  } else if (profileIdc === 77) {
    pp = "4D";
    cc = "40";
  } else {
    pp = "64";
    cc = "00";
  }
  const ll = levelIdc.toString(16).padStart(2, "0").toUpperCase();
  return `avc1.${pp}${cc}${ll}`;
}
var H265_CODEC_STRING = "hev1.1.6.L93.B0";
var AV1_CODEC_STRING = "av01.0.13M.08";
function defaultCodecString(codec) {
  if (codec === "h265") return H265_CODEC_STRING;
  if (codec === "av1") return AV1_CODEC_STRING;
  return H264_CODEC_STRING;
}
var SCAN_CODE_MAP = {
  // Function & system
  Escape: 1,
  F1: 59,
  F2: 60,
  F3: 61,
  F4: 62,
  F5: 63,
  F6: 64,
  F7: 65,
  F8: 66,
  F9: 67,
  F10: 68,
  F11: 87,
  F12: 88,
  PrintScreen: 99,
  ScrollLock: 70,
  Pause: 119,
  // Number row
  Backquote: 41,
  Digit1: 2,
  Digit2: 3,
  Digit3: 4,
  Digit4: 5,
  Digit5: 6,
  Digit6: 7,
  Digit7: 8,
  Digit8: 9,
  Digit9: 10,
  Digit0: 11,
  Minus: 12,
  Equal: 13,
  Backspace: 14,
  // Top letter row
  Tab: 15,
  KeyQ: 16,
  KeyW: 17,
  KeyE: 18,
  KeyR: 19,
  KeyT: 20,
  KeyY: 21,
  KeyU: 22,
  KeyI: 23,
  KeyO: 24,
  KeyP: 25,
  BracketLeft: 26,
  BracketRight: 27,
  Enter: 28,
  // Middle letter row
  CapsLock: 58,
  KeyA: 30,
  KeyS: 31,
  KeyD: 32,
  KeyF: 33,
  KeyG: 34,
  KeyH: 35,
  KeyJ: 36,
  KeyK: 37,
  KeyL: 38,
  Semicolon: 39,
  Quote: 40,
  Backslash: 43,
  // Bottom letter row
  ShiftLeft: 42,
  ShiftRight: 54,
  KeyZ: 44,
  KeyX: 45,
  KeyC: 46,
  KeyV: 47,
  KeyB: 48,
  KeyN: 49,
  KeyM: 50,
  Comma: 51,
  Period: 52,
  Slash: 53,
  // Modifiers & space
  ControlLeft: 29,
  ControlRight: 97,
  AltLeft: 56,
  AltRight: 100,
  MetaLeft: 125,
  MetaRight: 126,
  Space: 57,
  ContextMenu: 127,
  // Navigation cluster
  Insert: 110,
  Delete: 111,
  Home: 102,
  End: 107,
  PageUp: 104,
  PageDown: 109,
  ArrowUp: 103,
  ArrowDown: 108,
  ArrowLeft: 105,
  ArrowRight: 106,
  // Numpad
  NumLock: 69,
  Numpad0: 82,
  Numpad1: 79,
  Numpad2: 80,
  Numpad3: 81,
  Numpad4: 75,
  Numpad5: 76,
  Numpad6: 77,
  Numpad7: 71,
  Numpad8: 72,
  Numpad9: 73,
  NumpadDecimal: 83,
  NumpadEnter: 96,
  NumpadAdd: 78,
  NumpadSubtract: 74,
  NumpadMultiply: 55,
  NumpadDivide: 98
};
var MOUSE_BUTTON_MAP = {
  0: 272,
  // BTN_LEFT
  1: 274,
  // BTN_MIDDLE
  2: 273,
  // BTN_RIGHT
  3: 275,
  // BTN_SIDE / back
  4: 276
  // BTN_EXTRA / forward
};

// ../sdk/src/VideoDecoder.ts
var VeilVideoDecoder = class _VeilVideoDecoder {
  constructor(config) {
    this.decoder = null;
    this.initialized = false;
    // Drop delta frames until the first keyframe arrives — WebCodecs requires
    // a keyframe after configure() and the client often connects mid-GOP.
    this.waitingForKeyframe = true;
    /** Running count of decoded frames (for stats). */
    this.framesDecoded = 0;
    /** Running count of decode errors (dropped frames). */
    this.framesDropped = 0;
    /** Total bytes decoded (for bitrate estimation). */
    this.bytesDecoded = 0;
    this.config = config;
  }
  /** Initialize the underlying WebCodecs VideoDecoder. */
  async initialize() {
    if (!("VideoDecoder" in globalThis)) {
      throw new Error("WebCodecs VideoDecoder is not supported in this browser.");
    }
    const codecString = this.getCodecString(this.config.codec);
    const support = await globalThis.VideoDecoder.isConfigSupported({
      codec: codecString,
      codedWidth: this.config.width,
      codedHeight: this.config.height,
      optimizeForLatency: true
    });
    if (!support.supported) {
      throw new Error(`Codec ${codecString} is not supported by this browser.`);
    }
    this.decoder = new globalThis.VideoDecoder({
      output: (frame) => {
        this.framesDecoded++;
        try {
          this.config.onFrame(frame);
        } catch {
          frame.close();
        }
      },
      error: (err) => {
        this.framesDropped++;
        console.error("[VeilVideoDecoder] Decode error:", err);
        this.config.onError?.(err);
        this.waitingForKeyframe = true;
        if (this.decoder?.state === "closed") {
          this.decoder = null;
          this.initialized = false;
          this.initialize().catch(() => {
          });
        }
        this.config.onNeedKeyframe?.();
      }
    });
    this.decoder.configure({
      codec: codecString,
      codedWidth: this.config.width,
      codedHeight: this.config.height,
      optimizeForLatency: true
    });
    this.initialized = true;
  }
  /**
   * Submit an encoded video chunk for decoding.
   *
   * @param data - Annex B elementary stream bytes (the full frame payload
   *               as received from the Veil protocol stream).
   * @param isKeyframe - True when this is an IDR/keyframe.
   * @param timestampUs - Presentation timestamp in **microseconds**.
   */
  decode(data, isKeyframe, timestampUs) {
    if (!this.decoder || this.decoder.state === "closed") {
      return;
    }
    if (this.waitingForKeyframe) {
      if (!isKeyframe) return;
      this.waitingForKeyframe = false;
    }
    this.bytesDecoded += data.byteLength ?? data.byteLength;
    try {
      const chunk = new EncodedVideoChunk({
        type: isKeyframe ? "key" : "delta",
        timestamp: timestampUs,
        data
      });
      this.decoder.decode(chunk);
    } catch (err) {
      this.framesDropped++;
      console.warn("[VeilVideoDecoder] Failed to enqueue chunk:", err);
    }
  }
  /** Flush any pending frames from the decoder pipeline. */
  async flush() {
    if (this.decoder && this.decoder.state === "configured") {
      await this.decoder.flush();
    }
  }
  /**
   * Drop delta frames until the next keyframe, e.g. after a frame they
   * depend on was lost or arrived out of order.
   */
  resync() {
    this.waitingForKeyframe = true;
  }
  /** Close the decoder and release GPU resources. */
  close() {
    if (this.decoder && this.decoder.state !== "closed") {
      this.decoder.close();
    }
    this.decoder = null;
    this.initialized = false;
    this.waitingForKeyframe = true;
  }
  get isInitialized() {
    return this.initialized;
  }
  get state() {
    return this.decoder?.state ?? "closed";
  }
  /** Number of frames currently queued in the decoder pipeline. */
  get decodeQueueSize() {
    return this.decoder?.decodeQueueSize ?? 0;
  }
  /** True while the decoder is waiting for an IDR keyframe before it can begin decoding. */
  get isWaitingForKeyframe() {
    return this.waitingForKeyframe;
  }
  // ─── Static helpers ──────────────────────────────────────────────────────
  /** Returns true if the browser supports the WebCodecs VideoDecoder API. */
  static isSupported() {
    return "VideoDecoder" in globalThis;
  }
  /** Returns true if the given codec/resolution combination is supported. */
  static async isCodecSupported(codec, width, height) {
    if (!_VeilVideoDecoder.isSupported()) return false;
    try {
      const codecString = defaultCodecString(codec);
      const result = await globalThis.VideoDecoder.isConfigSupported({
        codec: codecString,
        codedWidth: width,
        codedHeight: height,
        optimizeForLatency: true
      });
      return result.supported ?? false;
    } catch {
      return false;
    }
  }
  // ─── Private helpers ─────────────────────────────────────────────────────
  getCodecString(codec) {
    if (this.config.codecString) return this.config.codecString;
    return defaultCodecString(codec);
  }
};

// ../sdk/src/AudioPlayer.ts
var SAMPLE_RATE = 48e3;
var CHANNELS = 2;
var TARGET_MS = 40;
var MAX_MS = 150;
var WORKLET_SOURCE = `
class VeilAudioProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.capacity = ${SAMPLE_RATE} * 2;
    this.left = new Float32Array(this.capacity);
    this.right = new Float32Array(this.capacity);
    this.read = 0;
    this.write = 0;
    this.queued = 0;
    this.primed = false;
    this.underruns = 0;
    this.trims = 0;
    this.target = Math.round(${SAMPLE_RATE} * ${TARGET_MS} / 1000);
    this.max = Math.round(${SAMPLE_RATE} * ${MAX_MS} / 1000);
    this.lastReport = 0;
    this.port.onmessage = (e) => {
      if (e.data.reset) {
        this.read = this.write = this.queued = 0;
        this.primed = false;
        return;
      }
      const { l, r } = e.data;
      for (let i = 0; i < l.length; i++) {
        this.left[this.write] = l[i];
        this.right[this.write] = r[i];
        this.write = (this.write + 1) % this.capacity;
      }
      this.queued = Math.min(this.capacity, this.queued + l.length);
      if (this.queued > this.max) {
        const drop = this.queued - this.target;
        this.read = (this.read + drop) % this.capacity;
        this.queued -= drop;
        this.trims++;
      }
    };
  }

  process(_inputs, outputs) {
    const [outL, outR] = outputs[0];
    const n = outL.length;
    if (!this.primed && this.queued >= this.target) this.primed = true;
    if (this.primed && this.queued >= n) {
      for (let i = 0; i < n; i++) {
        outL[i] = this.left[this.read];
        outR[i] = this.right[this.read];
        this.read = (this.read + 1) % this.capacity;
      }
      this.queued -= n;
    } else {
      outL.fill(0);
      outR.fill(0);
      if (this.primed) {
        // Ran dry: refill to the target before resuming.
        this.primed = false;
        this.underruns++;
      }
    }
    if (currentTime - this.lastReport > 0.25) {
      this.lastReport = currentTime;
      this.port.postMessage({
        bufferedMs: (this.queued / sampleRate) * 1000,
        underruns: this.underruns,
        trims: this.trims,
      });
    }
    return true;
  }
}
registerProcessor('veil-audio', VeilAudioProcessor);
`;
var AudioPlayer = class _AudioPlayer {
  constructor() {
    this.ctx = null;
    this.node = null;
    this.gain = null;
    this.decoder = null;
    this.volume = 1;
    this.muted = false;
    this.decoded = 0;
    this.workletStats = { bufferedMs: 0, underruns: 0, trims: 0 };
  }
  /** Whether `AudioDecoder` (WebCodecs audio) is available. */
  static isSupported() {
    return typeof AudioDecoder !== "undefined" && typeof AudioWorkletNode !== "undefined";
  }
  /** Start playback. Must be called from a user gesture the first time. */
  async enable() {
    if (!_AudioPlayer.isSupported()) {
      throw new Error("This browser has no WebCodecs AudioDecoder");
    }
    if (!this.ctx) {
      const ctx = new AudioContext({ sampleRate: SAMPLE_RATE, latencyHint: "interactive" });
      const url = URL.createObjectURL(new Blob([WORKLET_SOURCE], { type: "text/javascript" }));
      try {
        await ctx.audioWorklet.addModule(url);
      } finally {
        URL.revokeObjectURL(url);
      }
      const node = new AudioWorkletNode(ctx, "veil-audio", {
        numberOfInputs: 0,
        outputChannelCount: [CHANNELS]
      });
      node.port.onmessage = (e) => {
        this.workletStats = e.data;
      };
      const gain = ctx.createGain();
      node.connect(gain).connect(ctx.destination);
      this.ctx = ctx;
      this.node = node;
      this.gain = gain;
      this.applyGain();
    }
    await this.ctx.resume();
    this.ensureDecoder();
  }
  /** Stop playback and release the audio device. */
  async disable() {
    this.closeDecoder();
    const ctx = this.ctx;
    this.ctx = this.node = this.gain = null;
    await ctx?.close();
  }
  get enabled() {
    return this.ctx?.state === "running";
  }
  setVolume(volume) {
    this.volume = Math.min(1, Math.max(0, volume));
    this.applyGain();
  }
  setMuted(muted) {
    this.muted = muted;
    this.applyGain();
  }
  /** Drop anything buffered, e.g. after reconnecting to a new server. */
  reset() {
    this.closeDecoder();
    this.node?.port.postMessage({ reset: true });
    if (this.enabled) this.ensureDecoder();
  }
  /** Queue one Opus packet. Ignored until `enable()` has succeeded. */
  push(ptsMs, data) {
    if (!this.enabled) return;
    this.ensureDecoder();
    if (this.decoder?.state !== "configured") return;
    this.decoder.decode(new EncodedAudioChunk({
      type: "key",
      // Every Opus packet decodes on its own.
      timestamp: ptsMs * 1e3,
      data
    }));
  }
  getStats() {
    return {
      enabled: this.enabled,
      muted: this.muted,
      volume: this.volume,
      ...this.workletStats,
      decoded: this.decoded,
      outputLatencyMs: ((this.ctx?.outputLatency ?? 0) + (this.ctx?.baseLatency ?? 0)) * 1e3
    };
  }
  applyGain() {
    if (this.gain) this.gain.gain.value = this.muted ? 0 : this.volume;
  }
  ensureDecoder() {
    if (this.decoder && this.decoder.state !== "closed") return;
    this.decoder = new AudioDecoder({
      output: (audio) => this.onAudio(audio),
      error: (err) => {
        console.warn("[VeilAudio] decode error:", err.message);
      }
    });
    this.decoder.configure({ codec: "opus", sampleRate: SAMPLE_RATE, numberOfChannels: CHANNELS });
  }
  closeDecoder() {
    if (this.decoder && this.decoder.state !== "closed") this.decoder.close();
    this.decoder = null;
  }
  onAudio(audio) {
    try {
      const frames = audio.numberOfFrames;
      const l = new Float32Array(frames);
      const r = new Float32Array(frames);
      audio.copyTo(l, { planeIndex: 0, format: "f32-planar" });
      audio.copyTo(r, { planeIndex: audio.numberOfChannels > 1 ? 1 : 0, format: "f32-planar" });
      this.decoded++;
      this.node?.port.postMessage({ l, r }, [l.buffer, r.buffer]);
    } finally {
      audio.close();
    }
  }
};

// ../sdk/src/VeilClient.ts
var VeilClient = class {
  constructor(config) {
    this.state = "disconnected" /* Disconnected */;
    this.transport = null;
    this.datagramWriter = null;
    this.videoDecoder = null;
    this.inputManager = null;
    this.canvas = null;
    this.ctx2d = null;
    this.statsInterval = null;
    this.gamepadInterval = null;
    // Stores a deep snapshot of button/axis state (NOT a Gamepad reference).
    // Chrome returns the same live Gamepad object on every getGamepads() call,
    // so storing the object reference and comparing against it would always
    // compare the current state against itself → no events ever sent.
    /** Per browser pad: snapshot sequence, last snapshot sent, and when. */
    this.gamepads = /* @__PURE__ */ new Map();
    /** Rumble that outlasts the browser's 5 s limit is replayed by this timer. */
    this.rumbleTimers = /* @__PURE__ */ new Map();
    this.abortController = null;
    // Stats counters.
    this.framesDecoded = 0;
    this.framesDropped = 0;
    this.bytesVideo = 0;
    this.bytesAudio = 0;
    this.audioFramesDecoded = 0;
    this.lastStatsAt = Date.now();
    this.serverCodec = "h264";
    this.serverWidth = 0;
    this.serverHeight = 0;
    this.serverFps = 0;
    this.listeners = /* @__PURE__ */ new Map();
    this.lastCursorObjectUrl = null;
    // Latency tracking (ping/pong).
    this.rttMs = 0;
    this.pendingPings = /* @__PURE__ */ new Map();
    // ts → performance.now() when sent
    // Encoder info and decode latency tracking.
    this.encoderInfo = null;
    this.serverInfo = null;
    // Mouse capture. While captured, the browser freezes and hides its cursor,
    // so we track a virtual one in stream pixels and send absolute positions,
    // unless the app has locked the pointer, in which case we send deltas.
    this.cursorX = 0;
    this.cursorY = 0;
    this.appPointerLocked = false;
    /** Swallow the mouseup of the click that captured the mouse. */
    this.suppressMouseUp = -1;
    this.onPointerLockChange = null;
    /** Codec, size and codec string the decoder was configured with. */
    this.decoderConfigKey = "";
    this.decodeLatencyMs = 0;
    this.frameSubmitTimes = /* @__PURE__ */ new Map();
    // timestampUs → performance.now()
    this.lastVideoPtsMs = -1;
    this.audioPlayer = config.audioPlayer ?? new AudioPlayer();
    this.cfg = {
      serverUrl: config.serverUrl ?? DEFAULT_SERVER_URL,
      certFingerprint: config.certFingerprint ?? "",
      certHashUrl: config.certHashUrl ?? "",
      videoCodec: config.videoCodec ?? DEFAULT_VIDEO_CODEC,
      width: config.width ?? DEFAULT_WIDTH,
      height: config.height ?? DEFAULT_HEIGHT,
      framerate: config.framerate ?? DEFAULT_FRAMERATE,
      debug: config.debug ?? false,
      captureKeyboard: config.captureKeyboard !== false,
      captureMouse: config.captureMouse !== false,
      captureGamepad: config.captureGamepad !== false,
      pointerLock: config.pointerLock ?? false
    };
  }
  // ─── Public API ────────────────────────────────────────────────────────────
  /**
   * Connect to the Veil server and begin rendering to `canvas`.
   *
   * The canvas will be sized to match the server's announced resolution
   * after the first hello message is received.
   */
  async connect(canvas2) {
    if (this.state !== "disconnected" /* Disconnected */) {
      throw new Error("Already connected or connecting");
    }
    if (typeof WebTransport === "undefined") {
      throw new Error(
        "WebTransport is not supported in this browser. Please use Chrome 97+ (desktop or Android). Safari and Firefox do not support WebTransport."
      );
    }
    this.canvas = canvas2;
    this.ctx2d = canvas2.getContext("2d");
    if (!this.ctx2d) throw new Error("Could not get 2D canvas context");
    this.setState("connecting" /* Connecting */);
    this.abortController = new AbortController();
    try {
      const fingerprint = await this.resolveCertFingerprint();
      if (!fingerprint && typeof window !== "undefined" && window.location?.protocol === "https:") {
        const hashPort = parseInt(new URL(this.cfg.serverUrl).port || String(DEFAULT_QUIC_PORT)) + 1;
        throw new Error(
          `TLS certificate fingerprint unavailable (required for HTTPS pages). Fix: configure your reverse proxy to forward "/cert-hash" \u2192 http://server:${hashPort}/cert-hash. Or paste the fingerprint from the server startup logs into the "Cert fingerprint" field above.`
        );
      }
      const wtOptions = {};
      if (fingerprint) {
        wtOptions.serverCertificateHashes = [
          { algorithm: "sha-256", value: hexToBuffer(fingerprint) }
        ];
      }
      this.log("Opening WebTransport to", this.cfg.serverUrl);
      this.transport = new WebTransport(this.cfg.serverUrl, wtOptions);
      await this.transport.ready;
      this.log("WebTransport connected");
      this.audioPlayer.reset();
      this.datagramWriter = this.transport.datagrams.writable.getWriter();
      this.readIncomingStreams();
      this.readIncomingDatagrams();
      this.setupInput(canvas2);
      this.statsInterval = setInterval(() => this.emitStats(), STATS_UPDATE_INTERVAL);
      this.transport.closed.then(() => {
        this.log("WebTransport closed");
        this.handleClose();
      }).catch(() => this.handleClose());
    } catch (err) {
      this.setState("failed" /* Failed */);
      this.emit("error", err instanceof Error ? err : new Error(String(err)));
      throw err;
    }
  }
  /** Disconnect and release all resources. */
  async disconnect() {
    this.log("Disconnecting");
    this.abortController?.abort();
    this.cleanup();
    this.transport?.close({ closeCode: 0, reason: "client disconnect" });
    this.transport = null;
    this.setState("disconnected" /* Disconnected */);
    this.emit("disconnected", void 0);
  }
  /** Ask the server to send an IDR keyframe immediately. */
  async requestKeyframe() {
    await this.sendDatagram(new Uint8Array([INPUT_REQ_KEYFRAME]));
  }
  /**
   * Change encoding settings on the server. It rebuilds its encoder (and
   * resizes the display for size/framerate changes), then sends a new
   * `serverinfo`.
   */
  setStreamParams(req) {
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
  switchGpu(gpu) {
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
  enableAudio() {
    return this.audioPlayer.enable();
  }
  /** Stop audio and release the output device. */
  disableAudio() {
    return this.audioPlayer.disable();
  }
  setVolume(volume) {
    this.audioPlayer.setVolume(volume);
  }
  setMuted(muted) {
    this.audioPlayer.setMuted(muted);
  }
  /** The latest `serverinfo`, if any has arrived. */
  getServerInfo() {
    return this.serverInfo;
  }
  /** Raise the window with the given ID to the top and give it keyboard focus. */
  focusWindow(windowId) {
    const buf = new Uint8Array(5);
    const view = new DataView(buf.buffer);
    buf[0] = INPUT_FOCUS_WINDOW;
    view.setUint32(1, windowId, false);
    this.sendDatagram(buf);
  }
  /** Get the current connection state. */
  getState() {
    return this.state;
  }
  /** Get current stats snapshot. */
  getStats() {
    return this.buildStats();
  }
  on(event, listener) {
    if (!this.listeners.has(event)) this.listeners.set(event, /* @__PURE__ */ new Set());
    this.listeners.get(event).add(listener);
  }
  off(event, listener) {
    this.listeners.get(event)?.delete(listener);
  }
  // ─── Stream reader ────────────────────────────────────────────────────────
  async readIncomingStreams() {
    if (!this.transport) return;
    const reader = this.transport.incomingUnidirectionalStreams.getReader();
    let previous = Promise.resolve();
    try {
      while (true) {
        const { value: stream, done } = await reader.read();
        if (done) break;
        const data = readFullStream(stream);
        data.catch(() => {
        });
        previous = previous.then(async () => this.processMessage(await data)).catch((e) => this.log("Stream processing error:", e));
      }
    } catch (e) {
      this.log("Incoming stream reader ended:", e);
    } finally {
      reader.releaseLock();
    }
  }
  async readIncomingDatagrams() {
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
          this.handleAudio(payload);
        }
      }
    } catch (e) {
      this.log("Incoming datagram reader ended:", e);
    } finally {
      reader.releaseLock();
    }
  }
  processMessage(data) {
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
        this.serverInfo = JSON.parse(new TextDecoder().decode(payload));
        this.emit("serverinfo", this.serverInfo);
        break;
      case MSG_SERVER_STATS:
        this.emit("serverstats", JSON.parse(new TextDecoder().decode(payload)));
        break;
      case MSG_GAMEPAD_STATUS:
        this.emit("gamepadstatus", JSON.parse(new TextDecoder().decode(payload)));
        break;
      case MSG_COMPOSITOR_STATE:
        this.emit("compositorstate", JSON.parse(new TextDecoder().decode(payload)));
        break;
      case MSG_GAMEPAD_RUMBLE:
        this.handleGamepadRumble(payload);
        break;
      case MSG_VIEWERS:
        this.emit("viewers", JSON.parse(new TextDecoder().decode(payload)));
        break;
      default:
        this.log("Unknown message type:", msgType.toString(16));
    }
  }
  // ─── Message handlers ─────────────────────────────────────────────────────
  async handleHello(payload) {
    this.lastVideoPtsMs = -1;
    if (payload.length < 7) return;
    const view = new DataView(payload.buffer, payload.byteOffset);
    const codecByte = view.getUint8(0);
    const width = view.getUint16(1);
    const height = view.getUint16(3);
    const fps = view.getUint16(5);
    this.serverCodec = codecByte === CODEC_H265 ? "h265" : codecByte === CODEC_AV1 ? "av1" : "h264";
    this.serverWidth = width;
    this.serverHeight = height;
    this.serverFps = fps;
    this.log("Hello received:", { codec: this.serverCodec, width, height, fps });
    let encoderInfo = null;
    if (payload.length >= 12) {
      const profileIdc = view.getUint8(7);
      const levelIdc = view.getUint8(8);
      const gopSize = view.getUint16(9);
      const inFlight = view.getUint8(11);
      const profileName = profileIdc === 66 ? "Baseline" : profileIdc === 77 ? "Main" : "High";
      const levelStr = `${Math.floor(levelIdc / 10)}.${levelIdc % 10}`;
      encoderInfo = {
        codec: { h264: "H.264", h265: "H.265", av1: "AV1" }[this.serverCodec],
        profile: profileName,
        level: levelStr,
        gopSize,
        inFlight
      };
    }
    this.encoderInfo = encoderInfo;
    if (this.canvas) {
      this.canvas.width = width;
      this.canvas.height = height;
    }
    let codecString;
    if (this.serverCodec === "h264" && this.encoderInfo) {
      const profileIdc = payload.length >= 12 ? view.getUint8(7) : 100;
      const levelIdc = payload.length >= 12 ? view.getUint8(8) : 51;
      codecString = h264CodecString(profileIdc, levelIdc);
      this.log("H264 codec string:", codecString);
    }
    const configKey = `${this.serverCodec}|${width}x${height}|${codecString ?? ""}`;
    if (configKey === this.decoderConfigKey && this.videoDecoder?.isInitialized) {
      this.videoDecoder.resync();
      return;
    }
    this.decoderConfigKey = configKey;
    const firstHello = this.state !== "connected" /* Connected */;
    this.videoDecoder?.close();
    this.videoDecoder = new VeilVideoDecoder({
      codec: this.serverCodec,
      width,
      height,
      codecString,
      onFrame: (frame) => this.renderFrame(frame),
      onError: (err) => this.log("Decode error:", err),
      onNeedKeyframe: () => {
        void this.requestKeyframe();
      }
    });
    try {
      await this.videoDecoder.initialize();
      if (firstHello) {
        this.setState("connected" /* Connected */);
        this.emit("connected", void 0);
      }
      this.emit("videostart", void 0);
      this.canvas?.focus();
      await this.requestKeyframe();
    } catch (err) {
      this.emit("error", err instanceof Error ? err : new Error(String(err)));
      this.setState("failed" /* Failed */);
    }
  }
  handleVideo(payload) {
    if (payload.length < 9) return;
    const view = new DataView(payload.buffer, payload.byteOffset);
    const frameType = view.getUint8(0);
    const ptsMs = view.getUint32(5);
    const frameData = payload.subarray(9);
    const isKeyframe = frameType === FRAME_KEY;
    const timestampUs = ptsMs * 1e3;
    if (!isKeyframe && ptsMs <= this.lastVideoPtsMs) {
      this.log("Out-of-order video frame; waiting for keyframe");
      this.videoDecoder?.resync();
      void this.requestKeyframe();
      return;
    }
    this.lastVideoPtsMs = ptsMs;
    this.bytesVideo += frameData.byteLength;
    this.frameSubmitTimes.set(timestampUs, performance.now());
    if (this.frameSubmitTimes.size > 120) {
      const oldest = this.frameSubmitTimes.keys().next().value;
      if (oldest !== void 0) this.frameSubmitTimes.delete(oldest);
    }
    this.videoDecoder?.decode(frameData.buffer.slice(
      frameData.byteOffset,
      frameData.byteOffset + frameData.byteLength
    ), isKeyframe, timestampUs);
  }
  handleAudio(payload) {
    if (payload.length < 8) return;
    const view = new DataView(payload.buffer, payload.byteOffset);
    const ptsMs = view.getUint32(4);
    const audioData = payload.subarray(8);
    this.bytesAudio += audioData.byteLength;
    this.audioFramesDecoded++;
    this.audioPlayer.push(ptsMs, audioData);
  }
  handleCtrlDisplay(payload) {
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
    this.emit("displaychange", { width, height, framerate });
  }
  handleCtrlCursor(payload) {
    if (payload.length < 5) return;
    const view = new DataView(payload.buffer, payload.byteOffset);
    const hotspotX = view.getUint16(0);
    const hotspotY = view.getUint16(2);
    const kind = view.getUint8(4);
    if (this.lastCursorObjectUrl) {
      URL.revokeObjectURL(this.lastCursorObjectUrl);
      this.lastCursorObjectUrl = null;
    }
    const cursor = { hotspotX, hotspotY };
    if (kind === 1 && payload.length >= 9) {
      const imgLen = view.getUint32(5);
      const imgBytes = payload.subarray(9, 9 + imgLen);
      const blob = new Blob([imgBytes], { type: "image/png" });
      cursor.image = URL.createObjectURL(blob);
      this.lastCursorObjectUrl = cursor.image;
    } else if (kind === 2 && payload.length >= 6) {
      const nameLen = view.getUint8(5);
      const nameBytes = payload.subarray(6, 6 + nameLen);
      cursor.icon = new TextDecoder().decode(nameBytes);
    }
    this.emit("cursor", cursor);
  }
  handleCtrlPong(payload) {
    if (payload.length < 8) return;
    const view = new DataView(payload.buffer, payload.byteOffset);
    const ts = Number(view.getBigUint64(0, false));
    const sentAt = this.pendingPings.get(ts);
    if (sentAt !== void 0) {
      this.rttMs = Math.round(performance.now() - sentAt);
      this.pendingPings.delete(ts);
    }
  }
  sendPing() {
    const ts = Date.now();
    const buf = new Uint8Array(9);
    const view = new DataView(buf.buffer);
    buf[0] = INPUT_PING;
    view.setBigUint64(1, BigInt(ts), false);
    this.pendingPings.set(ts, performance.now());
    for (const [k] of this.pendingPings) {
      if (ts - k > 5e3) this.pendingPings.delete(k);
    }
    this.sendDatagram(buf);
  }
  handleWindowList(payload) {
    if (payload.length < 1) return;
    const count = payload[0];
    const windows = [];
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
    this.emit("windowlist", windows);
  }
  /** A game asked for rumble on one of our pads. */
  handleGamepadRumble(payload) {
    if (payload.length < 7) return;
    const view = new DataView(payload.buffer, payload.byteOffset);
    const index = view.getUint8(0);
    const strong = view.getUint16(1) / 65535;
    const weak = view.getUint16(3) / 65535;
    const durationMs = view.getUint16(5);
    this.emit("gamepadrumble", { index, strong, weak, durationMs });
    const timer = this.rumbleTimers.get(index);
    if (timer !== void 0) clearTimeout(timer);
    this.rumbleTimers.delete(index);
    const actuator = navigator.getGamepads()[index]?.vibrationActuator;
    if (!actuator) return;
    if (strong === 0 && weak === 0) {
      void actuator.reset().catch(() => {
      });
      return;
    }
    const play = (remaining) => {
      const chunk = Math.min(remaining, RUMBLE_MAX_EFFECT_MS);
      void actuator.playEffect("dual-rumble", { startDelay: 0, duration: chunk, strongMagnitude: strong, weakMagnitude: weak }).catch(() => {
      });
      if (remaining > chunk) {
        this.rumbleTimers.set(index, setTimeout(() => play(remaining - chunk), chunk - 50));
      } else {
        this.rumbleTimers.delete(index);
      }
    };
    play(durationMs);
  }
  handleCtrlPointerLock(payload) {
    if (payload.length < 8) return;
    const view = new DataView(payload.buffer, payload.byteOffset);
    const x = view.getFloat32(0);
    const y = view.getFloat32(4);
    this.appPointerLocked = true;
    this.cursorX = x;
    this.cursorY = y;
    this.emit("pointerlock", { x, y });
    this.emitMouseCapture();
  }
  handlePointerRelease() {
    this.appPointerLocked = false;
    this.emit("pointerrelease", void 0);
    this.emitMouseCapture();
  }
  // ─── Mouse capture ────────────────────────────────────────────────────────
  /**
   * Lock the browser's pointer to the stream. The cursor then moves
   * absolutely within the stream (draw it yourself; see `getCursorPosition`),
   * or sends relative motion while the app has locked the pointer. Must be
   * called from a user gesture (click or key press). Esc always releases.
   */
  captureMouse() {
    if (!this.canvas || this.isMouseCaptured()) return;
    const fail = (err) => console.warn("[VeilClient] Mouse capture refused by the browser:", err instanceof Error ? err.message : err);
    const req = this.canvas.requestPointerLock({ unadjustedMovement: true });
    if (req instanceof Promise) {
      req.catch((err) => {
        if (!this.canvas || err instanceof DOMException && err.name === "WrongDocumentError") {
          fail(err);
          return;
        }
        const retry = this.canvas.requestPointerLock();
        if (retry instanceof Promise) retry.catch(fail);
      });
    }
  }
  releaseMouse() {
    if (this.isMouseCaptured()) document.exitPointerLock();
  }
  isMouseCaptured() {
    return !!this.canvas && document.pointerLockElement === this.canvas;
  }
  getMouseCapture() {
    const captured = this.isMouseCaptured();
    return {
      captured,
      relative: captured && this.appPointerLocked,
      appWantsLock: this.appPointerLocked
    };
  }
  /** The cursor position in stream pixels (the virtual one while captured). */
  getCursorPosition() {
    return { x: this.cursorX, y: this.cursorY };
  }
  emitMouseCapture() {
    this.emit("mousecapture", this.getMouseCapture());
  }
  /** Map a mouse event to stream pixels. */
  streamCoords(e) {
    if (this.isMouseCaptured()) return { x: this.cursorX, y: this.cursorY };
    const canvas2 = e.target;
    const rect = canvas2.getBoundingClientRect();
    return {
      x: (e.clientX - rect.left) * (canvas2.width / rect.width),
      y: (e.clientY - rect.top) * (canvas2.height / rect.height)
    };
  }
  // ─── Rendering ────────────────────────────────────────────────────────────
  renderFrame(frame) {
    const submitTime = this.frameSubmitTimes.get(frame.timestamp);
    if (submitTime !== void 0) {
      this.decodeLatencyMs = Math.round(performance.now() - submitTime);
      this.frameSubmitTimes.delete(frame.timestamp);
    }
    if (this.ctx2d && this.canvas) {
      const sw = Math.min(frame.displayWidth, this.serverWidth || frame.displayWidth);
      const sh = Math.min(frame.displayHeight, this.serverHeight || frame.displayHeight);
      this.ctx2d.drawImage(frame, 0, 0, sw, sh, 0, 0, this.canvas.width, this.canvas.height);
    }
    this.framesDecoded++;
    frame.close();
  }
  // ─── Input capture ────────────────────────────────────────────────────────
  setupInput(element) {
    if (this.cfg.captureKeyboard) {
      if (!element.hasAttribute("tabindex")) element.setAttribute("tabindex", "0");
      element.addEventListener("keydown", (e) => this.onKeyDown(e));
      element.addEventListener("keyup", (e) => this.onKeyUp(e));
    }
    if (this.cfg.captureMouse) {
      element.addEventListener("mouseenter", () => this.onPointerEnter());
      element.addEventListener("mouseleave", () => this.onPointerLeave());
      element.addEventListener("mousedown", (e) => this.onMouseDown(e));
      element.addEventListener("mouseup", (e) => this.onMouseUp(e));
      element.addEventListener("mousemove", (e) => this.onMouseMove(e));
      element.addEventListener("wheel", (e) => this.onWheel(e), { passive: false });
      element.addEventListener("contextmenu", (e) => e.preventDefault());
      this.onPointerLockChange = () => {
        if (this.isMouseCaptured()) {
          this.onPointerEnter();
        }
        this.emitMouseCapture();
      };
      document.addEventListener("pointerlockchange", this.onPointerLockChange);
    }
    if (this.cfg.captureGamepad) {
      this.gamepadInterval = setInterval(() => this.pollGamepads(), GAMEPAD_POLL_INTERVAL);
    }
  }
  onPointerEnter() {
    this.sendDatagram(new Uint8Array([INPUT_POINTER_ENTER]));
  }
  onPointerLeave() {
    this.sendDatagram(new Uint8Array([INPUT_POINTER_LEAVE]));
  }
  onKeyDown(e) {
    e.preventDefault();
    const keycode = SCAN_CODE_MAP[e.code] ?? 0;
    if (keycode === 0) return;
    const char = e.key.length === 1 ? e.key.codePointAt(0) ?? 0 : 0;
    const buf = new Uint8Array(9);
    const view = new DataView(buf.buffer);
    buf[0] = INPUT_KEY_DOWN;
    view.setUint32(1, keycode, false);
    view.setUint32(5, char, false);
    this.sendDatagram(buf);
  }
  onKeyUp(e) {
    e.preventDefault();
    const keycode = SCAN_CODE_MAP[e.code] ?? 0;
    if (keycode === 0) return;
    const buf = new Uint8Array(5);
    const view = new DataView(buf.buffer);
    buf[0] = INPUT_KEY_UP;
    view.setUint32(1, keycode, false);
    this.sendDatagram(buf);
  }
  onMouseDown(e) {
    e.preventDefault();
    if (this.cfg.pointerLock && !this.isMouseCaptured()) {
      this.suppressMouseUp = e.button;
      this.captureMouse();
      return;
    }
    const { x, y } = this.streamCoords(e);
    const btn = MOUSE_BUTTON_MAP[e.button] ?? e.button;
    const buf = new Uint8Array(13);
    const view = new DataView(buf.buffer);
    buf[0] = INPUT_MOUSE_DOWN;
    view.setUint32(1, btn, false);
    view.setFloat32(5, x, false);
    view.setFloat32(9, y, false);
    this.sendDatagram(buf);
  }
  onMouseUp(e) {
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
  onMouseMove(e) {
    const canvas2 = e.target;
    const rect = canvas2.getBoundingClientRect();
    const scaleX = canvas2.width / rect.width;
    const scaleY = canvas2.height / rect.height;
    let x, y, dx = 0, dy = 0;
    if (!this.isMouseCaptured()) {
      x = (e.clientX - rect.left) * scaleX;
      y = (e.clientY - rect.top) * scaleY;
      this.cursorX = x;
      this.cursorY = y;
    } else if (this.appPointerLocked) {
      x = this.cursorX;
      y = this.cursorY;
      dx = e.movementX * scaleX;
      dy = e.movementY * scaleY;
    } else {
      this.cursorX = Math.min(canvas2.width - 1, Math.max(0, this.cursorX + e.movementX * scaleX));
      this.cursorY = Math.min(canvas2.height - 1, Math.max(0, this.cursorY + e.movementY * scaleY));
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
  onWheel(e) {
    e.preventDefault();
    const buf = new Uint8Array(9);
    const view = new DataView(buf.buffer);
    buf[0] = INPUT_SCROLL;
    view.setFloat32(1, e.deltaX, false);
    view.setFloat32(5, e.deltaY, false);
    this.sendDatagram(buf);
  }
  pollGamepads() {
    const pads = navigator.getGamepads();
    const now = performance.now();
    const currentIndices = /* @__PURE__ */ new Set();
    for (const pad of pads) {
      if (!pad || pad.index > 255) continue;
      currentIndices.add(pad.index);
      let tracked = this.gamepads.get(pad.index);
      if (!tracked) {
        tracked = { seq: 0, last: null, sentAt: 0 };
        this.gamepads.set(pad.index, tracked);
        this.emit("gamepadconnected", { index: pad.index, id: pad.id });
        this.sendDatagram(new Uint8Array([INPUT_GAMEPAD_AVAILABLE, pad.index]));
      }
      const snapshot = gamepadSnapshot(pad);
      if (snapshot.key !== tracked.last || now - tracked.sentAt >= GAMEPAD_RESEND_INTERVAL) {
        tracked.seq = tracked.seq + 1 >>> 0;
        const buf = new Uint8Array(20);
        const view = new DataView(buf.buffer);
        buf[0] = INPUT_GAMEPAD_STATE;
        buf[1] = pad.index;
        view.setUint32(2, tracked.seq);
        buf.set(snapshot.bytes, 6);
        this.sendDatagram(buf);
        tracked.last = snapshot.key;
        tracked.sentAt = now;
      }
    }
    for (const index of [...this.gamepads.keys()]) {
      if (!currentIndices.has(index)) {
        this.gamepads.delete(index);
        this.emit("gamepaddisconnected", { index, id: "" });
        this.sendDatagram(new Uint8Array([INPUT_GAMEPAD_UNAVAILABLE, index]));
      }
    }
  }
  // ─── Datagram sender ──────────────────────────────────────────────────────
  sendDatagram(data) {
    if (!this.datagramWriter) return;
    this.datagramWriter.write(data).catch(
      (e) => this.log("Datagram send failed:", e)
    );
  }
  // ─── Certificate resolution ───────────────────────────────────────────────
  async resolveCertFingerprint() {
    if (this.cfg.certFingerprint) return this.cfg.certFingerprint;
    const url = new URL(this.cfg.serverUrl);
    const directUrl = `http://${url.hostname}:${parseInt(url.port || String(DEFAULT_QUIC_PORT)) + 1}/cert-hash`;
    const candidates = [];
    if (this.cfg.certHashUrl) {
      candidates.push(this.cfg.certHashUrl);
    }
    if (typeof window !== "undefined" && window.location?.origin && window.location.hostname === url.hostname) {
      const pageOriginUrl = `${window.location.origin}/cert-hash`;
      if (pageOriginUrl !== directUrl) {
        candidates.push(pageOriginUrl);
      }
    }
    candidates.push(directUrl);
    for (const hashUrl of candidates) {
      try {
        this.log("Fetching cert fingerprint from", hashUrl);
        const res = await fetch(hashUrl);
        if (!res.ok) continue;
        const json = await res.json();
        if (json.sha256_hex) {
          console.info("[VeilClient] cert fingerprint resolved from", hashUrl);
          this.log("Cert fingerprint:", json.sha256_hex);
          return json.sha256_hex;
        }
      } catch {
        console.warn("[VeilClient] cert fingerprint fetch failed for", hashUrl);
        this.log("Cert fingerprint fetch failed for", hashUrl);
      }
    }
    console.warn("[VeilClient] Could not fetch cert fingerprint from any endpoint");
    return "";
  }
  // ─── Stats ────────────────────────────────────────────────────────────────
  emitStats() {
    this.sendPing();
    if (this.videoDecoder?.isWaitingForKeyframe) {
      this.log("Decoder waiting for keyframe \u2014 retrying FIR");
      void this.requestKeyframe();
    }
    const stats = this.buildStats();
    this.emit("stats", stats);
  }
  buildStats() {
    const now = Date.now();
    const dt = (now - this.lastStatsAt) / 1e3;
    const videoBitrate = dt > 0 ? this.bytesVideo * 8 / dt : 0;
    const audioBitrate = dt > 0 ? this.bytesAudio * 8 / dt : 0;
    const fps = dt > 0 ? this.framesDecoded / dt : 0;
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
        decodeQueueSize: this.videoDecoder?.decodeQueueSize ?? 0
      },
      audio: {
        framesDecoded: this.audioFramesDecoded,
        bitrateBps: audioBitrate,
        codec: "opus",
        player: this.audioPlayer.getStats()
      },
      rttMs: this.rttMs,
      encoderInfo: this.encoderInfo,
      decodeLatencyMs: this.decodeLatencyMs
    };
  }
  // ─── Lifecycle helpers ────────────────────────────────────────────────────
  setState(s) {
    if (this.state !== s) {
      this.state = s;
      this.emit("statechange", s);
    }
  }
  handleClose() {
    this.cleanup();
    this.setState("closed" /* Closed */);
    this.emit("disconnected", void 0);
  }
  cleanup() {
    this.releaseMouse();
    if (this.onPointerLockChange) {
      document.removeEventListener("pointerlockchange", this.onPointerLockChange);
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
    this.videoDecoder?.close();
    this.videoDecoder = null;
    this.decoderConfigKey = "";
    this.datagramWriter?.close().catch(() => {
    });
    this.datagramWriter = null;
    this.gamepads.clear();
    for (const timer of this.rumbleTimers.values()) clearTimeout(timer);
    this.rumbleTimers.clear();
    this.pendingPings.clear();
    this.frameSubmitTimes.clear();
    this.rttMs = 0;
  }
  emit(event, data) {
    this.listeners.get(event)?.forEach((fn) => {
      try {
        fn(data);
      } catch (e) {
        this.log("Listener error:", e);
      }
    });
  }
  log(...args) {
    if (this.cfg.debug) console.log("[VeilClient]", ...args);
  }
};
async function readFullStream(stream) {
  const reader = stream.getReader();
  const chunks = [];
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
function hexToBuffer(hex) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes.buffer;
}
var RUMBLE_MAX_EFFECT_MS = 5e3;
function gamepadSnapshot(pad) {
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
  return { bytes, key: bytes.join(",") };
}
function radialDeadzone(x, y, deadzone = GAMEPAD_DEADZONE) {
  const magnitude = Math.hypot(x, y);
  if (magnitude <= deadzone) return [0, 0];
  const scaled = Math.min(1, (magnitude - deadzone) / (1 - deadzone));
  return [x / magnitude * scaled, y / magnitude * scaled];
}

// app.ts
var $ = (id) => document.getElementById(id);
var canvas = $("stream-canvas");
var canvasWrap = canvas.parentElement;
var vcursor = $("vcursor");
var captureBadge = $("captureBadge");
var overlay = $("overlay");
var overlayStatus = $("overlay-status");
var panelToggle = $("panelToggle");
var connDot = $("connDot");
var connState = $("connState");
var connNote = $("connNote");
var viewersEl = $("viewers");
var viewersSummary = $("viewersSummary");
var viewerList = $("viewerList");
var serverUrl = $("serverUrl");
var connectBtn = $("connectBtn");
var disconnBtn = $("disconnBtn");
var infoGpu = $("infoGpu");
var infoDriver = $("infoDriver");
var infoWrite = $("infoWrite");
var infoCodec = $("infoCodec");
var infoRc = $("infoRc");
var infoStream = $("infoStream");
var infoGop = $("infoGop");
var liveNote = $("liveNote");
var fpsCanvas = $("fpsCanvas");
var bitrateCanvas = $("bitrateCanvas");
var latencyCanvas = $("latencyCanvas");
var valFps = $("valFps");
var valBitrate = $("valBitrate");
var valLatency = $("valLatency");
var barEncode = $("barEncode");
var barNet = $("barNet");
var barDecode = $("barDecode");
var legEncode = $("legEncode");
var legNet = $("legNet");
var legDecode = $("legDecode");
var statDecoded = $("statDecoded");
var statDropped = $("statDropped");
var statQueue = $("statQueue");
var statEncMax = $("statEncMax");
var statKeys = $("statKeys");
var statFrameKb = $("statFrameKb");
var soundNote = $("soundNote");
var soundEnable = $("soundEnable");
var soundMute = $("soundMute");
var soundVolume = $("soundVolume");
var soundVolumeLabel = $("soundVolumeLabel");
var soundOverlay = $("soundOverlay");
var soundBuffered = $("soundBuffered");
var soundUnderruns = $("soundUnderruns");
var soundTrims = $("soundTrims");
var soundRate = $("soundRate");
var soundBitrate = $("soundBitrate");
var soundLatency = $("soundLatency");
var settingsNote = $("settingsNote");
var setGpu = $("setGpu");
var switchGpuBtn = $("switchGpuBtn");
var setCodec = $("setCodec");
var setPreset = $("setPreset");
var presetLabel = $("presetLabel");
var setRes = $("setRes");
var setFps = $("setFps");
var setGop = $("setGop");
var setInFlight = $("setInFlight");
var applyBtn = $("applyBtn");
var revertBtn = $("revertBtn");
var applyHint = $("applyHint");
var logBody = $("logBody");
var logEmpty = $("logEmpty");
var copyLogBtn = $("copyLogBtn");
var clearLogBtn = $("clearLogBtn");
var padList = $("padList");
var padEmpty = $("padEmpty");
var padNote = $("padNote");
var compNote = $("compNote");
var compDisplay = $("compDisplay");
var compApp = $("compApp");
var compSockets = $("compSockets");
var compClients = $("compClients");
var compWindows = $("compWindows");
var compX11Wrap = $("compX11Wrap");
var compX11Summary = $("compX11Summary");
var compX11 = $("compX11");
var toastEl = $("toast");
var toastTimer = null;
function toast(msg) {
  toastEl.textContent = msg;
  toastEl.classList.add("show");
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => toastEl.classList.remove("show"), 3e3);
}
var fmtMs = (v) => v > 0 ? `${v < 10 ? v.toFixed(1) : Math.round(v)} ms` : "\u2014";
var codecName = (c) => ({ h265: "H.265", av1: "AV1" })[c] ?? "H.264";
function presetDescription(preset) {
  if (preset >= 7) return `Const QP ${40 - 2 * preset}`;
  return `VBR \xB7 ${["lowest", "low", "low+", "medium", "medium+", "high", "highest"][preset]}`;
}
function shortGpu(name) {
  return name.replace(/^AMD Radeon /, "").replace(/^NVIDIA GeForce /, "").replace(/\s*\(.*\)$/, "");
}
function gpuVendorTag(name) {
  if (/radeon|amd|radv/i.test(name)) return '<span class="tag amd">AMD</span>';
  if (/nvidia|geforce/i.test(name)) return '<span class="tag nvidia">NVIDIA</span>';
  return "";
}
function escapeHtml(s) {
  return s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
}
var MAX_SAMPLES = 60;
var fpsHistory = new Array(MAX_SAMPLES).fill(0);
var bitrateHistory = new Array(MAX_SAMPLES).fill(0);
var latencyHistory = new Array(MAX_SAMPLES).fill(0);
function push(arr, v) {
  arr.push(v);
  if (arr.length > MAX_SAMPLES) arr.shift();
}
function drawSparkline(cnv, data, lineHex, fillRgba) {
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const rect = cnv.getBoundingClientRect();
  const w = rect.width || 100;
  const h = rect.height || 30;
  if (cnv.width !== Math.round(w * dpr) || cnv.height !== Math.round(h * dpr)) {
    cnv.width = Math.round(w * dpr);
    cnv.height = Math.round(h * dpr);
  }
  const ctx = cnv.getContext("2d");
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);
  if (data.filter((v) => v > 0).length < 2) return;
  const max = Math.max(...data) * 1.2;
  const len = data.length;
  const pad = 2;
  const px = (i) => i / (len - 1) * w;
  const py = (v) => pad + (1 - v / max) * (h - pad * 2);
  ctx.beginPath();
  ctx.moveTo(px(0), h);
  for (let i = 0; i < len; i++) ctx.lineTo(px(i), py(data[i]));
  ctx.lineTo(px(len - 1), h);
  ctx.closePath();
  ctx.fillStyle = fillRgba;
  ctx.fill();
  ctx.beginPath();
  for (let i = 0; i < len; i++) {
    if (i === 0) ctx.moveTo(px(i), py(data[i]));
    else ctx.lineTo(px(i), py(data[i]));
  }
  ctx.strokeStyle = lineHex;
  ctx.lineWidth = 1.5;
  ctx.lineJoin = "round";
  ctx.stroke();
  ctx.setTransform(1, 0, 0, 1, 0, 0);
}
function drawGraphs() {
  drawSparkline(fpsCanvas, fpsHistory, "#4ade80", "rgba(74,222,128,0.13)");
  drawSparkline(bitrateCanvas, bitrateHistory, "#60a5fa", "rgba(96,165,250,0.13)");
  drawSparkline(latencyCanvas, latencyHistory, "#fb923c", "rgba(251,146,60,0.13)");
}
function resetLive() {
  fpsHistory.fill(0);
  bitrateHistory.fill(0);
  latencyHistory.fill(0);
  for (const el of [
    valFps,
    valBitrate,
    valLatency,
    legEncode,
    legNet,
    legDecode,
    statDecoded,
    statDropped,
    statQueue,
    statEncMax,
    statKeys,
    statFrameKb
  ]) {
    el.textContent = "\u2014";
  }
  for (const el of [barEncode, barNet, barDecode]) el.style.width = "0";
  drawGraphs();
}
function togglePanel() {
  document.body.classList.toggle("panel-collapsed");
  drawGraphs();
}
panelToggle.addEventListener("click", togglePanel);
document.addEventListener("keydown", (e) => {
  const t = e.target;
  if (e.key === "Tab" && !t.closest("input, select, textarea")) {
    e.preventDefault();
    e.stopPropagation();
    togglePanel();
  }
}, { capture: true });
var SETTLE_MS = 2e3;
var segments = [];
var current = null;
function segmentKey(info) {
  return [
    info.gpu,
    info.codec,
    info.width,
    info.height,
    info.framerate,
    info.preset,
    info.gop_size,
    info.in_flight_frames
  ].join("|");
}
function startSegment(info, droppedNow) {
  const key = segmentKey(info);
  if (current?.key === key) return;
  current = {
    key,
    label: `${shortGpu(info.gpu)} \xB7 ${codecName(info.codec)}`,
    detail: `${info.width}\xD7${info.height}@${info.framerate} \xB7 q${info.preset} \xB7 gop ${info.gop_size} \xB7 if ${info.in_flight_frames}`,
    start: performance.now(),
    samples: 0,
    fps: 0,
    mbps: 0,
    encodeMs: 0,
    encodeSamples: 0,
    decodeMs: 0,
    rttMs: 0,
    droppedAtStart: droppedNow,
    dropped: 0
  };
  segments.push(current);
  renderLog();
}
function settled(seg) {
  return performance.now() - seg.start >= SETTLE_MS;
}
function avg(total, n) {
  return n > 0 ? total / n : 0;
}
function renderLog() {
  logEmpty.style.display = segments.length ? "none" : "";
  const rows = segments.filter((s) => s.samples > 0);
  const best = {
    fps: Math.max(...rows.map((s) => avg(s.fps, s.samples))),
    enc: Math.min(...rows.map((s) => avg(s.encodeMs, s.encodeSamples) || Infinity)),
    dec: Math.min(...rows.map((s) => avg(s.decodeMs, s.samples) || Infinity))
  };
  logBody.innerHTML = "";
  for (const seg of [...segments].reverse()) {
    const tr = document.createElement("tr");
    if (seg === current) tr.className = "live";
    const secs = Math.round((performance.now() - seg.start) / 1e3);
    const has = seg.samples > 0;
    const fps = avg(seg.fps, seg.samples);
    const enc = avg(seg.encodeMs, seg.encodeSamples);
    const dec = avg(seg.decodeMs, seg.samples);
    const cls = (isBest) => isBest && rows.length > 1 ? ' class="best"' : "";
    tr.innerHTML = `
      <td class="cfg">${escapeHtml(seg.label)}<small>${escapeHtml(seg.detail)}</small></td>
      <td>${seg === current ? secs : Math.round(seg.samples)}s</td>
      <td${cls(has && fps === best.fps)}>${has ? fps.toFixed(0) : "\u2026"}</td>
      <td>${has ? avg(seg.mbps, seg.samples).toFixed(1) : "\u2026"}</td>
      <td${cls(has && enc === best.enc)}>${has && enc ? enc.toFixed(1) : "\u2026"}</td>
      <td${cls(has && dec === best.dec)}>${has ? dec.toFixed(1) : "\u2026"}</td>
      <td>${has ? avg(seg.rttMs, seg.samples).toFixed(0) : "\u2026"}</td>
      <td>${seg.dropped}</td>`;
    logBody.appendChild(tr);
  }
}
function logAsMarkdown() {
  const lines = [
    "| Config | Settings | Seconds | FPS | Mbps | Encode ms | Decode ms | RTT ms | Dropped |",
    "|---|---|---|---|---|---|---|---|---|"
  ];
  for (const s of segments) {
    lines.push(`| ${s.label} | ${s.detail} | ${s.samples} | ${avg(s.fps, s.samples).toFixed(1)} | ${avg(s.mbps, s.samples).toFixed(2)} | ${avg(s.encodeMs, s.encodeSamples).toFixed(2)} | ${avg(s.decodeMs, s.samples).toFixed(2)} | ${avg(s.rttMs, s.samples).toFixed(1)} | ${s.dropped} |`);
  }
  return lines.join("\n");
}
copyLogBtn.addEventListener("click", async () => {
  try {
    await navigator.clipboard.writeText(logAsMarkdown());
    toast("Session log copied as Markdown");
  } catch {
    toast("Clipboard unavailable \u2014 see the console");
    console.log(logAsMarkdown());
  }
});
clearLogBtn.addEventListener("click", () => {
  segments.length = 0;
  if (current) {
    current.start = performance.now();
    current.samples = current.fps = current.mbps = 0;
    current.encodeMs = current.encodeSamples = current.decodeMs = current.rttMs = 0;
    current.dropped = 0;
    current.droppedAtStart = lastStats?.video.framesDropped ?? 0;
    segments.push(current);
  }
  renderLog();
});
var lastStats = null;
var lastServerStats = null;
function updateLatency() {
  const enc = lastServerStats?.encode_ms_avg ?? 0;
  const net = (lastStats?.rttMs ?? 0) / 2;
  const dec = lastStats?.decodeLatencyMs ?? 0;
  const total = enc + net + dec;
  push(latencyHistory, total);
  valLatency.innerHTML = total > 0 ? `${total.toFixed(1)}<small> ms</small>` : "\u2014";
  legEncode.textContent = fmtMs(enc);
  legNet.textContent = fmtMs(net);
  legDecode.textContent = fmtMs(dec);
  const pct = (v) => `${Math.min(100, v / 50 * 100)}%`;
  barEncode.style.width = pct(enc);
  barNet.style.width = pct(net);
  barDecode.style.width = pct(dec);
}
function onStats(stats) {
  lastStats = stats;
  onAudioStats(stats);
  const mbps = stats.video.bitrateBps / 1e6;
  push(fpsHistory, stats.video.fps);
  push(bitrateHistory, mbps);
  valFps.textContent = String(stats.video.fps);
  valBitrate.textContent = mbps.toFixed(1);
  statDecoded.textContent = stats.video.framesDecoded.toLocaleString();
  statDropped.textContent = String(stats.video.framesDropped);
  statQueue.textContent = String(stats.video.decodeQueueSize);
  updateLatency();
  drawGraphs();
  if (current) {
    current.dropped = stats.video.framesDropped - current.droppedAtStart;
    if (settled(current) && stats.video.fps > 0) {
      current.samples += 1;
      current.fps += stats.video.fps;
      current.mbps += mbps;
      current.decodeMs += stats.decodeLatencyMs;
      current.rttMs += stats.rttMs;
    }
    renderLog();
  }
  liveNote.textContent = current && !settled(current) ? "settling\u2026" : "";
}
function onServerStats(s) {
  lastServerStats = s;
  const perSec = 1e3 / Math.max(1, s.window_ms);
  statEncMax.textContent = fmtMs(s.encode_ms_max);
  statKeys.textContent = (s.keyframes * perSec).toFixed(1);
  statFrameKb.textContent = s.frames > 0 ? `${(s.bytes / s.frames / 1024).toFixed(1)} KB` : "\u2014";
  if (current && settled(current) && s.frames > 0) {
    current.encodeMs += s.encode_ms_avg;
    current.encodeSamples += 1;
  }
}
var serverInfo = null;
function onServerInfo(info) {
  serverInfo = info;
  pendingApply = false;
  infoGpu.innerHTML = `${escapeHtml(info.gpu)}${gpuVendorTag(info.gpu)}`;
  infoGpu.title = info.gpu;
  infoDriver.textContent = info.gpu_driver;
  infoWrite.textContent = info.write_path === "direct" ? "Direct (storage writes)" : "Copy (per-plane images)";
  infoCodec.textContent = `${codecName(info.codec)} ${info.codec_profile}`;
  infoRc.textContent = info.rate_control;
  infoRc.title = info.rate_control;
  infoStream.textContent = `${info.width}\xD7${info.height} @ ${info.framerate} fps \xB7 in flight ${info.in_flight_frames}`;
  infoGop.textContent = `${info.gop_size} frames \xB7 ${info.temporal_layers} temporal layer${info.temporal_layers === 1 ? "" : "s"}`;
  setGpu.innerHTML = "";
  for (const name of info.gpus) {
    const opt = document.createElement("option");
    opt.value = name;
    opt.textContent = name === info.gpu ? `${name} (current)` : name;
    setGpu.appendChild(opt);
  }
  setGpu.value = info.gpu;
  fillForm(info);
  startSegment(info, lastStats?.video.framesDropped ?? 0);
  settingsNote.textContent = "";
  if (expectingRestart) {
    expectingRestart = false;
    toast(`Now running on ${info.gpu}`);
  }
}
function ensureOption(sel, value, label) {
  if (![...sel.options].some((o) => o.value === value)) {
    const opt = document.createElement("option");
    opt.value = value;
    opt.textContent = label;
    sel.appendChild(opt);
  }
  sel.value = value;
}
function fillForm(info) {
  for (const opt of setCodec.options) {
    opt.disabled = !!info.codecs && !info.codecs.includes(opt.value);
    opt.title = opt.disabled ? `${info.gpu} can't encode this` : "";
  }
  setCodec.value = info.codec;
  setPreset.value = String(info.preset);
  ensureOption(setRes, `${info.width}x${info.height}`, `${info.width}\xD7${info.height}`);
  ensureOption(setFps, String(info.framerate), `${info.framerate} fps`);
  setGop.value = String(info.gop_size);
  ensureOption(setInFlight, String(info.in_flight_frames), String(info.in_flight_frames));
  updateFormState();
}
function formChanges() {
  if (!serverInfo) return {};
  const req = {};
  const [w, h] = setRes.value.split("x").map(Number);
  if (setCodec.value !== serverInfo.codec) req.codec = setCodec.value;
  if (Number(setPreset.value) !== serverInfo.preset) req.preset = Number(setPreset.value);
  if (w !== serverInfo.width || h !== serverInfo.height) {
    req.width = w;
    req.height = h;
  }
  if (Number(setFps.value) !== serverInfo.framerate) req.framerate = Number(setFps.value);
  const gop = Number(setGop.value);
  if (gop > 0 && gop !== serverInfo.gop_size) req.gop_size = gop;
  if (Number(setInFlight.value) !== serverInfo.in_flight_frames) req.in_flight_frames = Number(setInFlight.value);
  return req;
}
var pendingApply = false;
function updateFormState() {
  presetLabel.textContent = presetDescription(Number(setPreset.value));
  const connected = client?.getState() === "connected" /* Connected */ && !!serverInfo;
  const changes = formChanges();
  const changed = Object.keys(changes).length > 0;
  setCodec.classList.toggle("changed", "codec" in changes);
  setPreset.classList.toggle("changed", "preset" in changes);
  setRes.classList.toggle("changed", "width" in changes);
  setFps.classList.toggle("changed", "framerate" in changes);
  setGop.classList.toggle("changed", "gop_size" in changes);
  setInFlight.classList.toggle("changed", "in_flight_frames" in changes);
  applyBtn.disabled = !connected || !changed || pendingApply;
  revertBtn.disabled = !changed;
  applyHint.textContent = pendingApply ? "Applying\u2026" : "width" in changes || "framerate" in changes ? "Resizes the app and rebuilds the encoder" : "Rebuilds the encoder";
  const gpuChanged = !!serverInfo && setGpu.value !== serverInfo.gpu;
  switchGpuBtn.disabled = !connected || !gpuChanged || expectingRestart;
  setGpu.classList.toggle("changed", gpuChanged);
}
for (const el of [setCodec, setPreset, setRes, setFps, setGop, setInFlight, setGpu]) {
  el.addEventListener("input", updateFormState);
}
applyBtn.addEventListener("click", () => {
  const changes = formChanges();
  if (!client || Object.keys(changes).length === 0) return;
  pendingApply = true;
  settingsNote.textContent = "applying\u2026";
  client.setStreamParams(changes);
  updateFormState();
  setTimeout(() => {
    if (pendingApply) {
      pendingApply = false;
      settingsNote.textContent = "";
      toast("No response to the settings change");
      updateFormState();
    }
  }, 8e3);
});
revertBtn.addEventListener("click", () => {
  if (serverInfo) {
    fillForm(serverInfo);
    setGpu.value = serverInfo.gpu;
    updateFormState();
  }
});
var expectingRestart = false;
var reconnectTimer = null;
switchGpuBtn.addEventListener("click", () => {
  if (!client || !serverInfo) return;
  const gpu = setGpu.value;
  expectingRestart = true;
  settingsNote.textContent = "restarting\u2026";
  toast(`Restarting on ${gpu}\u2026`);
  client.switchGpu(gpu);
  updateFormState();
});
function scheduleReconnect(attempt) {
  if (reconnectTimer) clearTimeout(reconnectTimer);
  if (attempt > 30) {
    expectingRestart = false;
    settingsNote.textContent = "";
    toast("Server did not come back after the restart");
    return;
  }
  connNote.textContent = `server restarting\u2026 (${attempt})`;
  reconnectTimer = setTimeout(async () => {
    const ok = await connect();
    if (!ok && expectingRestart) scheduleReconnect(attempt + 1);
  }, attempt === 1 ? 1500 : 1e3);
}
var STANDARD_BUTTONS = [
  "A",
  "B",
  "X",
  "Y",
  "LB",
  "RB",
  "LT",
  "RT",
  "View",
  "Menu",
  "LS",
  "RS",
  "\u2191",
  "\u2193",
  "\u2190",
  "\u2192",
  "Home"
];
var padStatus = /* @__PURE__ */ new Map();
var otherPads = 0;
var myViewerId = null;
var padRumbleUntil = /* @__PURE__ */ new Map();
var padPrevEvents = /* @__PURE__ */ new Map();
var padRate = /* @__PURE__ */ new Map();
function onGamepadStatus(list) {
  const now = performance.now();
  padStatus.clear();
  otherPads = 0;
  for (const s of list) {
    if (s.viewer !== myViewerId) {
      otherPads++;
      continue;
    }
    padStatus.set(s.index, s);
    const prev = padPrevEvents.get(s.index);
    if (prev && now > prev.at) {
      padRate.set(s.index, (s.events - prev.events) * 1e3 / (now - prev.at));
    }
    padPrevEvents.set(s.index, { events: s.events, at: now });
  }
  renderPadInfo();
}
function parsePadId(id) {
  const m = id.match(/^(.*?)\s*\((?:.*?)Vendor:\s*([0-9a-f]{4})\s*Product:\s*([0-9a-f]{4})\)\s*$/i);
  if (m) return { name: m[1] || id, usb: `${m[2]}:${m[3]}` };
  const f = id.match(/^([0-9a-f]{4})-([0-9a-f]{4})-(.*)$/i);
  if (f) return { name: f[3], usb: `${f[1]}:${f[2]}` };
  return { name: id, usb: "" };
}
var padCards = /* @__PURE__ */ new Map();
function makeCard(pad) {
  const { name, usb } = parsePadId(pad.id);
  const root = document.createElement("div");
  root.className = "pad-card";
  root.innerHTML = `
    <div class="pad-head">
      <span class="pad-index">#${pad.index}</span>
      <span class="pad-name" title="${escapeHtml(pad.id)}">${escapeHtml(name)}</span>
      <span class="pad-rumble" title="The game is rumbling this pad">Rumble</span>
    </div>
    <div class="pad-row"><span class="k">Browser</span><span class="v" data-browser></span></div>
    <div class="pad-row"><span class="k">Server</span><span class="v" data-server></span></div>
    <div class="pad-viz">
      <div class="stick" data-left><div class="knob"></div></div>
      <div class="stick" data-right><div class="knob"></div></div>
      <div class="triggers">
        <div class="trigger">LT <div class="bar"><div class="fill" data-lt></div></div></div>
        <div class="trigger">RT <div class="bar"><div class="fill" data-rt></div></div></div>
      </div>
      <div class="buttons" data-buttons></div>
    </div>`;
  const q = (sel) => root.querySelector(sel);
  const buttons = q("[data-buttons]");
  const standard = pad.mapping === "standard";
  const chips = pad.buttons.map((_, i) => {
    const chip = document.createElement("span");
    chip.className = "btn-chip";
    chip.textContent = standard ? STANDARD_BUTTONS[i] ?? String(i) : String(i);
    buttons.appendChild(chip);
    return chip;
  });
  const card = {
    root,
    browser: q("[data-browser]"),
    server: q("[data-server]"),
    left: q("[data-left]"),
    right: q("[data-right]"),
    lt: q("[data-lt]"),
    rt: q("[data-rt]"),
    chips
  };
  card.browser.textContent = [
    usb && `USB ${usb}`,
    standard ? "standard mapping" : "non-standard mapping (buttons may be mislabelled)",
    `${pad.buttons.length} buttons, ${pad.axes.length} axes`
  ].filter(Boolean).join(" \xB7 ");
  return card;
}
function renderPadInfo() {
  const connected = client?.getState() === "connected" /* Connected */;
  for (const [index, card] of padCards) {
    const s = padStatus.get(index);
    if (!connected) {
      card.server.className = "v";
      card.server.textContent = "Not connected";
    } else if (!s) {
      card.server.className = "v pad-wait";
      card.server.textContent = "Waiting for the virtual device\u2026";
    } else if (s.error) {
      card.server.className = "v pad-err";
      card.server.textContent = `Virtual device failed: ${s.error}`;
    } else {
      const rate = padRate.get(index);
      card.server.className = "v pad-ok";
      card.server.textContent = `${s.emulates} at ${s.device ?? "unknown node"} \xB7 ${s.events.toLocaleString()} events` + (rate !== void 0 ? ` (${rate.toFixed(0)}/s)` : "");
    }
  }
  const failed = [...padStatus.values()].filter((s) => s.error).length;
  padNote.textContent = [
    failed && `${failed} failed on server`,
    otherPads && `${otherPads} from other viewers`
  ].filter(Boolean).join(" \xB7 ");
  padNote.style.color = failed ? "var(--bad)" : "";
}
function placeKnob(stick, x, y) {
  const knob = stick.firstElementChild;
  knob.style.transform = `translate(${(x || 0) * 16}px, ${(y || 0) * 16}px)`;
}
function renderPads() {
  const pads = [...navigator.getGamepads?.() ?? []].filter((p) => !!p);
  const seen = new Set(pads.map((p) => p.index));
  for (const [index, card] of padCards) {
    if (!seen.has(index)) {
      card.root.remove();
      padCards.delete(index);
    }
  }
  for (const pad of pads) {
    let card = padCards.get(pad.index);
    if (!card) {
      card = makeCard(pad);
      padCards.set(pad.index, card);
      padList.appendChild(card.root);
      renderPadInfo();
    }
    const ax = pad.axes;
    placeKnob(card.left, ax[0] ?? 0, ax[1] ?? 0);
    placeKnob(card.right, ax[2] ?? 0, ax[3] ?? 0);
    card.left.classList.toggle("pressed", !!pad.buttons[10]?.pressed);
    card.right.classList.toggle("pressed", !!pad.buttons[11]?.pressed);
    card.lt.style.width = `${(pad.buttons[6]?.value ?? 0) * 100}%`;
    card.rt.style.width = `${(pad.buttons[7]?.value ?? 0) * 100}%`;
    pad.buttons.forEach((b, i) => card.chips[i]?.classList.toggle("on", b.pressed));
    card.root.classList.toggle("rumbling", (padRumbleUntil.get(pad.index) ?? 0) > performance.now());
  }
  padEmpty.style.display = pads.length ? "none" : "";
  requestAnimationFrame(renderPads);
}
window.addEventListener("gamepadconnected", () => renderPadInfo());
var audioPlayer = new AudioPlayer();
var soundMuted = false;
var lastAudioPackets = 0;
function loadVolume() {
  try {
    const raw = localStorage.getItem("veil.volume");
    const v = Number(raw);
    return raw !== null && Number.isFinite(v) ? v : 100;
  } catch {
    return 100;
  }
}
function saveVolume(v) {
  try {
    localStorage.setItem("veil.volume", String(v));
  } catch {
  }
}
async function enableSound() {
  try {
    await audioPlayer.enable();
  } catch (err) {
    toast(`Couldn't start sound: ${err instanceof Error ? err.message : err}`);
  }
  updateSoundUi();
}
function updateSoundUi() {
  const on = audioPlayer.enabled;
  soundEnable.style.display = on ? "none" : "";
  soundMute.style.display = on ? "" : "none";
  soundMute.textContent = soundMuted ? "Unmute" : "Mute";
  soundOverlay.style.display = !on && client?.getState() === "connected" /* Connected */ ? "block" : "none";
  if (!AudioPlayer.isSupported()) {
    soundEnable.disabled = true;
    soundNote.textContent = "not supported in this browser";
  } else {
    soundNote.textContent = on ? soundMuted ? "muted" : "playing" : "off";
  }
}
function onAudioStats(stats) {
  const p = stats.audio.player;
  const rate = stats.audio.framesDecoded - lastAudioPackets;
  lastAudioPackets = stats.audio.framesDecoded;
  soundRate.textContent = String(rate);
  soundBitrate.textContent = stats.audio.bitrateBps > 0 ? `${Math.round(stats.audio.bitrateBps / 1e3)} kbps` : "\u2014";
  soundBuffered.textContent = p.enabled ? `${Math.round(p.bufferedMs)} ms` : "\u2014";
  soundUnderruns.textContent = p.enabled ? String(p.underruns) : "\u2014";
  soundTrims.textContent = p.enabled ? String(p.trims) : "\u2014";
  soundLatency.textContent = p.enabled && p.outputLatencyMs > 0 ? `${Math.round(p.outputLatencyMs)} ms` : "\u2014";
  updateSoundUi();
}
soundEnable.addEventListener("click", () => {
  void enableSound();
});
soundOverlay.addEventListener("click", (e) => {
  e.stopPropagation();
  void enableSound();
});
soundMute.addEventListener("click", () => {
  soundMuted = !soundMuted;
  audioPlayer.setMuted(soundMuted);
  updateSoundUi();
});
soundVolume.addEventListener("input", () => {
  const v = Number(soundVolume.value);
  soundVolumeLabel.textContent = `${v}%`;
  audioPlayer.setVolume(v / 100);
  saveVolume(v);
});
{
  const v = loadVolume();
  soundVolume.value = String(v);
  soundVolumeLabel.textContent = `${v}%`;
  audioPlayer.setVolume(v / 100);
}
function fmtDuration(secs) {
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m`;
  return `${Math.floor(secs / 3600)}h ${Math.floor(secs % 3600 / 60)}m`;
}
function onViewers(list) {
  myViewerId = list.you;
  const n = list.viewers.length;
  const controlling = list.viewers.filter((v) => v.active).length;
  const remote = list.viewers.filter((v) => v.network === "internet").length;
  viewersEl.style.display = "";
  viewersSummary.innerHTML = `<b>${n}</b> viewer${n === 1 ? "" : "s"} connected` + (remote ? ` \xB7 ${remote} from the internet` : "") + (controlling ? ` \xB7 <b>${controlling}</b> controlling` : "");
  viewerList.innerHTML = list.viewers.map((v) => {
    const you = v.id === list.you;
    const cls = ["viewer", you && "you", v.active && "active"].filter(Boolean).join(" ");
    const title = `${v.network === "lan" ? "Local network" : "Internet"}, connected ${fmtDuration(v.connected_secs)}` + (v.active ? ", sending input" : "");
    return `<span class="${cls}" title="${title}">${you ? "You" : `#${v.id}`} <small>${v.network === "lan" ? "LAN" : "net"} \xB7 ${fmtDuration(v.connected_secs)}</small></span>`;
  }).join("");
  panelToggle.textContent = `Panel \u21E5 \xB7 \u{1F441} ${n}`;
}
function resetViewers() {
  viewersEl.style.display = "none";
  viewerList.innerHTML = "";
  panelToggle.textContent = "Panel \u21E5";
}
function onCompositorState(st) {
  compDisplay.textContent = `${st.width}\xD7${st.height} @ ${st.framerate} fps`;
  const appState = st.app_exit ? `<span style="color:var(--warn)">exited (${escapeHtml(st.app_exit)})</span>` : '<span style="color:var(--good)">running</span>';
  compApp.innerHTML = `${escapeHtml(st.app_command)} \xB7 pid ${st.app_pid ?? "?"} \xB7 ${appState}`;
  compApp.title = st.app_exit ? "The launched process exited. Launchers like Steam do this and leave their children running." : st.app_command;
  compSockets.textContent = `WAYLAND_DISPLAY=${st.wayland_socket}` + (st.x11_display ? ` \xB7 DISPLAY=${st.x11_display}` : " \xB7 no Xwayland");
  const mapped = st.surfaces.filter((s) => s.mapped).length;
  compClients.textContent = `${st.clients} Wayland client${st.clients === 1 ? "" : "s"} \xB7 ${mapped}/${st.surfaces.length} windows mapped \xB7 ${st.other_surfaces} other surfaces`;
  const isNote = (s) => s.mapped && !s.composited;
  const problems = st.surfaces.filter((s) => s.problem && !isNote(s)).length + st.x11_windows.filter((w) => w.problem).length;
  compNote.textContent = problems ? `${problems} issue${problems === 1 ? "" : "s"}` : "";
  compNote.style.color = problems ? "var(--warn)" : "";
  compWindows.innerHTML = "";
  if (st.surfaces.length === 0) {
    compWindows.innerHTML = `<div class="comp-empty">No windows yet \u2014 the app hasn't created one.</div>`;
  }
  for (const s of st.surfaces) {
    const card = document.createElement("div");
    const notMapped = !s.mapped;
    card.className = "win-card" + (notMapped ? " bad" : s.problem && !isNote(s) ? " problem" : "");
    const name = s.title || s.app_id || `surface ${s.key}`;
    const state = s.composited ? '<span class="chip good">IN VIDEO</span>' : s.mapped ? '<span class="chip warn">HIDDEN</span>' : '<span class="chip bad">NOT MAPPED</span>';
    const kind = s.kind === "x11" ? '<span class="chip x11">X11</span>' : '<span class="chip wl">WAYLAND</span>';
    const flags = [s.active && "ACTIVE", s.fullscreen && "FULLSCREEN"].filter(Boolean).map((f) => `<span class="chip">${f}</span>`).join("");
    const meta = [
      s.stack_index !== null ? `stack <b>#${s.stack_index}</b>` : null,
      s.configured ? `configured <b>${s.configured[0]}\xD7${s.configured[1]}</b>` : "not configured",
      s.buffer ? `buffer <b>${s.buffer.width}\xD7${s.buffer.height}</b> ${s.buffer.kind}` + (s.buffer.kind === "shm" ? " (CPU copy)" : "") : "no buffer",
      `<b>${s.commits_per_sec.toFixed(0)}</b> commits/s`,
      s.app_id && s.title ? `app_id <b>${escapeHtml(s.app_id)}</b>` : null,
      s.x11_window !== null ? `X11 0x${s.x11_window.toString(16)}` : null
    ].filter(Boolean).join(" \xB7 ");
    card.innerHTML = `
      <div class="win-head">
        <span class="win-title" title="${escapeHtml(name)}">${escapeHtml(name)}</span>
        ${kind}${flags}${state}
      </div>
      <div class="win-meta">${meta}</div>
      ${s.problem ? `<div class="win-problem"${isNote(s) ? ' style="color:var(--dim)"' : ""}>${escapeHtml(s.problem)}</div>` : ""}`;
    if (s.window_id !== null && !s.active) {
      const btn = document.createElement("button");
      btn.className = "btn btn-ghost win-raise";
      btn.textContent = "Raise";
      btn.title = "Bring this window to the top and focus it";
      const wid = s.window_id;
      btn.addEventListener("click", () => client?.focusWindow(wid));
      card.querySelector(".win-head").appendChild(btn);
    }
    compWindows.appendChild(card);
  }
  compX11Wrap.style.display = st.x11_display ? "" : "none";
  const x11Problems = st.x11_windows.filter((w) => w.problem).length;
  compX11Summary.textContent = `X11 windows (${st.x11_windows.length}` + (x11Problems ? `, ${x11Problems} with issues` : "") + ")";
  if (x11Problems) compX11Wrap.open = true;
  compX11.innerHTML = st.x11_windows.map((w) => {
    const flags = [
      w.mapped ? "mapped" : "unmapped",
      w.override_redirect && "override-redirect",
      w.fullscreen && "fullscreen",
      w.has_surface ? "linked" : "no surface"
    ].filter(Boolean).join(", ");
    const [x, y, ww, hh] = w.geometry;
    return `<div class="x11-row"><b>0x${w.id.toString(16)}</b> ${escapeHtml(w.title || w.class || "(untitled)")}${w.class && w.title ? ` <span style="color:var(--dim)">(${escapeHtml(w.class)})</span>` : ""}<br><span style="color:var(--dim)">${ww}\xD7${hh}+${x}+${y} \xB7 ${flags}</span>${w.problem ? `<br><span class="p">${escapeHtml(w.problem)}</span>` : ""}</div>`;
  }).join("");
}
function resetCompositor() {
  for (const el of [compDisplay, compApp, compSockets, compClients]) el.textContent = "\u2014";
  compNote.textContent = "";
  compWindows.innerHTML = "";
  compX11.innerHTML = "";
}
var ARROW_CURSOR = "data:image/svg+xml," + encodeURIComponent(
  '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="24" viewBox="0 0 20 24"><path d="M1 1v19l5-5 3.5 7.5 3-1.4L9 13.7h7z" fill="#fff" stroke="#000" stroke-width="1.2" stroke-linejoin="round"/></svg>'
);
var cursorImage = ARROW_CURSOR;
var cursorHotspot = { x: 1, y: 1 };
var cursorHidden = false;
var captureState = { captured: false, relative: false, appWantsLock: false };
var badgeTimer = null;
var RELEASE_HINT = "<kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>M</kbd> or <kbd>Esc</kbd> to release";
function showBadge(html, opts = {}) {
  captureBadge.innerHTML = html;
  captureBadge.classList.toggle("want", !!opts.want);
  captureBadge.classList.add("show");
  if (badgeTimer) clearTimeout(badgeTimer);
  badgeTimer = opts.sticky ? null : setTimeout(() => captureBadge.classList.remove("show"), 3500);
}
function onMouseCapture(state) {
  const was = captureState;
  captureState = state;
  if (state.captured) {
    if (!was.captured || was.relative !== state.relative) {
      showBadge(state.relative ? `<b>Mouse captured</b> \xB7 relative (app has locked the pointer) \u2014 ${RELEASE_HINT}` : `<b>Mouse captured</b> \xB7 absolute \u2014 ${RELEASE_HINT}`);
    }
  } else if (state.appWantsLock) {
    showBadge("<b>The app wants the mouse</b> \u2014 click the stream or press <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>M</kbd>", { sticky: true, want: true });
  } else if (was.captured || was.appWantsLock) {
    showBadge("Mouse released \u2014 click the stream or press <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>M</kbd> to capture");
  }
}
function renderCursor() {
  const show = !!client && captureState.captured && !captureState.relative && !cursorHidden;
  vcursor.style.display = show ? "block" : "none";
  if (show && client) {
    const pos = client.getCursorPosition();
    const rect = canvas.getBoundingClientRect();
    const wrap = canvasWrap.getBoundingClientRect();
    const sx = rect.width / canvas.width;
    const sy = rect.height / canvas.height;
    const left = rect.left - wrap.left + pos.x * sx - cursorHotspot.x;
    const top = rect.top - wrap.top + pos.y * sy - cursorHotspot.y;
    vcursor.style.transform = `translate(${left}px, ${top}px)`;
  }
  requestAnimationFrame(renderCursor);
}
document.addEventListener("pointerlockerror", () => {
  showBadge("<b>Couldn\u2019t capture the mouse</b> \u2014 the browser refused (is this window focused?)", { want: true });
});
document.addEventListener("keydown", (e) => {
  if (e.ctrlKey && e.altKey && !e.shiftKey && !e.metaKey && e.code === "KeyM") {
    e.preventDefault();
    e.stopPropagation();
    if (!client) return;
    if (client.isMouseCaptured()) client.releaseMouse();
    else client.captureMouse();
  }
}, { capture: true });
var STATE_LABEL = {
  ["disconnected" /* Disconnected */]: "Disconnected",
  ["connecting" /* Connecting */]: "Connecting\u2026",
  ["connected" /* Connected */]: "Streaming",
  ["failed" /* Failed */]: "Failed",
  ["closed" /* Closed */]: "Closed"
};
var client = null;
function setStatus(state) {
  const connected = state === "connected" /* Connected */;
  const idle = state === "disconnected" /* Disconnected */ || state === "closed" /* Closed */ || state === "failed" /* Failed */;
  overlay.classList.toggle("hidden", connected);
  overlayStatus.textContent = expectingRestart ? "Restarting on another GPU\u2026" : STATE_LABEL[state];
  connDot.className = `dot ${state}`;
  connState.textContent = STATE_LABEL[state];
  if (!expectingRestart) connNote.textContent = "";
  connectBtn.disabled = !idle;
  disconnBtn.disabled = idle;
  if (idle) {
    resetViewers();
    lastAudioPackets = 0;
    captureState = { captured: false, relative: false, appWantsLock: false };
    captureBadge.classList.remove("show");
    resetLive();
    current = null;
    renderLog();
    padStatus.clear();
    padRumbleUntil.clear();
    myViewerId = null;
    padPrevEvents.clear();
    padRate.clear();
  }
  renderPadInfo();
  updateSoundUi();
  if (connected) {
    connNote.textContent = (() => {
      try {
        return new URL(serverUrl.value).host;
      } catch {
        return "";
      }
    })();
    canvas.focus();
  }
  updateFormState();
}
async function connect() {
  if (client) return true;
  const url = serverUrl.value.trim();
  const c = new VeilClient({
    serverUrl: url,
    videoCodec: "h264",
    width: 1920,
    height: 1080,
    framerate: 60,
    captureKeyboard: true,
    captureMouse: true,
    captureGamepad: true,
    // Click the stream to capture the mouse.
    pointerLock: true,
    audioPlayer,
    debug: false
  });
  client = c;
  c.on("statechange", (state) => {
    if (client !== c) return;
    setStatus(state);
    if (state === "closed" /* Closed */ || state === "failed" /* Failed */) {
      client = null;
      if (expectingRestart) scheduleReconnect(1);
    }
  });
  c.on("stats", (s) => client === c && onStats(s));
  c.on("serverinfo", (i) => client === c && onServerInfo(i));
  c.on("serverstats", (s) => client === c && onServerStats(s));
  c.on("gamepadstatus", (s) => client === c && onGamepadStatus(s));
  c.on("gamepadrumble", (r) => {
    if (client !== c) return;
    const off = r.strong === 0 && r.weak === 0;
    padRumbleUntil.set(r.index, off ? 0 : performance.now() + r.durationMs);
  });
  c.on("error", (err) => console.error("[veil]", err.message));
  c.on("mousecapture", (s) => client === c && onMouseCapture(s));
  c.on("cursor", (cursor) => {
    cursorHidden = false;
    if (cursor.image) {
      canvas.style.cursor = `url(${cursor.image}) ${cursor.hotspotX} ${cursor.hotspotY}, auto`;
      cursorImage = cursor.image;
      cursorHotspot = { x: cursor.hotspotX, y: cursor.hotspotY };
    } else if (cursor.icon) {
      canvas.style.cursor = cursor.icon;
      cursorImage = ARROW_CURSOR;
      cursorHotspot = { x: 1, y: 1 };
    } else {
      canvas.style.cursor = "none";
      cursorHidden = true;
    }
    if (vcursor.src !== cursorImage) vcursor.src = cursorImage;
  });
  c.on("compositorstate", (st) => client === c && onCompositorState(st));
  c.on("viewers", (list) => client === c && onViewers(list));
  setStatus("connecting" /* Connecting */);
  try {
    await c.connect(canvas);
    return true;
  } catch (err) {
    console.error("[veil] connect failed:", err);
    if (client === c) client = null;
    setStatus("failed" /* Failed */);
    return false;
  }
}
async function disconnect() {
  expectingRestart = false;
  if (reconnectTimer) clearTimeout(reconnectTimer);
  const c = client;
  client = null;
  await c?.disconnect();
  canvas.style.cursor = "";
  resetCompositor();
  setStatus("disconnected" /* Disconnected */);
  canvas.getContext("2d")?.clearRect(0, 0, canvas.width, canvas.height);
}
connectBtn.addEventListener("click", () => {
  void connect();
});
disconnBtn.addEventListener("click", () => {
  void disconnect();
});
serverUrl.addEventListener("keydown", (e) => {
  if (e.key === "Enter") void connect();
});
window.addEventListener("resize", drawGraphs);
setStatus("disconnected" /* Disconnected */);
renderLog();
requestAnimationFrame(renderPads);
vcursor.src = cursorImage;
requestAnimationFrame(renderCursor);
setTimeout(() => {
  void connect();
}, 300);
