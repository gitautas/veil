/**
 * Plays the server's Opus audio: WebCodecs decodes it, and an AudioWorklet
 * plays it from a small jitter buffer.
 *
 * Browsers only allow audio after a user gesture, so call `enable()` from a
 * click or key handler. One player can be shared across connections (pass it
 * as `VeilClientConfig.audioPlayer`), so sound survives reconnects.
 */

const SAMPLE_RATE = 48000;
const CHANNELS = 2;

/** Buffer this much before starting (and after an underrun). */
const TARGET_MS = 40;
/** Trim back to the target if this much builds up, so latency can't creep. */
const MAX_MS = 150;

export interface AudioPlayerStats {
  /** `enable()` succeeded and the context is running. */
  enabled: boolean;
  muted: boolean;
  volume: number;
  /** Audio queued for playback. */
  bufferedMs: number;
  /** Times playback ran dry and played silence. */
  underruns: number;
  /** Times the buffer grew too large and was trimmed. */
  trims: number;
  /** Packets decoded since enabling. */
  decoded: number;
  /** The output device's own latency, as reported by the browser. */
  outputLatencyMs: number;
}

// Runs on the audio thread. Kept as a string so the SDK stays one bundle.
const WORKLET_SOURCE = `
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

export class AudioPlayer {
  private ctx: AudioContext | null = null;
  private node: AudioWorkletNode | null = null;
  private gain: GainNode | null = null;
  private decoder: AudioDecoder | null = null;
  private volume = 1;
  private muted = false;
  private decoded = 0;
  private workletStats = { bufferedMs: 0, underruns: 0, trims: 0 };

  /** Whether `AudioDecoder` (WebCodecs audio) is available. */
  static isSupported(): boolean {
    return typeof AudioDecoder !== 'undefined' && typeof AudioWorkletNode !== 'undefined';
  }

  /** Start playback. Must be called from a user gesture the first time. */
  async enable(): Promise<void> {
    if (!AudioPlayer.isSupported()) {
      throw new Error('This browser has no WebCodecs AudioDecoder');
    }
    if (!this.ctx) {
      const ctx = new AudioContext({ sampleRate: SAMPLE_RATE, latencyHint: 'interactive' });
      const url = URL.createObjectURL(new Blob([WORKLET_SOURCE], { type: 'text/javascript' }));
      try {
        await ctx.audioWorklet.addModule(url);
      } finally {
        URL.revokeObjectURL(url);
      }
      const node = new AudioWorkletNode(ctx, 'veil-audio', {
        numberOfInputs: 0,
        outputChannelCount: [CHANNELS],
      });
      node.port.onmessage = (e) => { this.workletStats = e.data; };
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
  async disable(): Promise<void> {
    this.closeDecoder();
    const ctx = this.ctx;
    this.ctx = this.node = this.gain = null;
    await ctx?.close();
  }

  get enabled(): boolean {
    return this.ctx?.state === 'running';
  }

  setVolume(volume: number): void {
    this.volume = Math.min(1, Math.max(0, volume));
    this.applyGain();
  }

  setMuted(muted: boolean): void {
    this.muted = muted;
    this.applyGain();
  }

  /** Drop anything buffered, e.g. after reconnecting to a new server. */
  reset(): void {
    this.closeDecoder();
    this.node?.port.postMessage({ reset: true });
    if (this.enabled) this.ensureDecoder();
  }

  /** Queue one Opus packet. Ignored until `enable()` has succeeded. */
  push(ptsMs: number, data: Uint8Array): void {
    if (!this.enabled) return;
    this.ensureDecoder();
    if (this.decoder?.state !== 'configured') return;
    this.decoder.decode(new EncodedAudioChunk({
      type: 'key', // Every Opus packet decodes on its own.
      timestamp: ptsMs * 1000,
      data,
    }));
  }

  getStats(): AudioPlayerStats {
    return {
      enabled: this.enabled,
      muted: this.muted,
      volume: this.volume,
      ...this.workletStats,
      decoded: this.decoded,
      outputLatencyMs: ((this.ctx?.outputLatency ?? 0) + (this.ctx?.baseLatency ?? 0)) * 1000,
    };
  }

  private applyGain(): void {
    if (this.gain) this.gain.gain.value = this.muted ? 0 : this.volume;
  }

  private ensureDecoder(): void {
    if (this.decoder && this.decoder.state !== 'closed') return;
    this.decoder = new AudioDecoder({
      output: (audio) => this.onAudio(audio),
      error: (err) => {
        console.warn('[VeilAudio] decode error:', err.message);
        // A closed decoder is recreated on the next packet.
      },
    });
    this.decoder.configure({ codec: 'opus', sampleRate: SAMPLE_RATE, numberOfChannels: CHANNELS });
  }

  private closeDecoder(): void {
    if (this.decoder && this.decoder.state !== 'closed') this.decoder.close();
    this.decoder = null;
  }

  private onAudio(audio: AudioData): void {
    try {
      const frames = audio.numberOfFrames;
      const l = new Float32Array(frames);
      const r = new Float32Array(frames);
      audio.copyTo(l, { planeIndex: 0, format: 'f32-planar' });
      audio.copyTo(r, { planeIndex: audio.numberOfChannels > 1 ? 1 : 0, format: 'f32-planar' });
      this.decoded++;
      this.node?.port.postMessage({ l, r }, [l.buffer, r.buffer]);
    } finally {
      audio.close();
    }
  }
}
