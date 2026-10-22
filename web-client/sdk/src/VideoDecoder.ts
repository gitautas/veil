/**
 * VeilVideoDecoder — hardware-accelerated video decoding via the WebCodecs API.
 *
 * Receives raw H.264 / H.265 Annex B elementary stream chunks from the
 * WebTransport layer, decodes them using the browser's built-in hardware
 * decoder, and renders each VideoFrame to a Canvas2D context.
 */

import { defaultCodecString } from './constants';
import type { VideoCodecName } from './types';

export type FrameOutputCallback = (frame: VideoFrame) => void;

export interface VeilVideoDecoderConfig {
  codec: VideoCodecName;
  width: number;
  height: number;
  onFrame: FrameOutputCallback;
  onError?: (error: DOMException) => void;
  /** Called when a decode error forces a keyframe request (FIR). */
  onNeedKeyframe?: () => void;
  /** Optional explicit WebCodecs codec string (overrides default derivation from codec field). */
  codecString?: string;
}

/**
 * WebCodecs-based video decoder for Veil stream data.
 *
 * Usage:
 * ```ts
 * const dec = new VeilVideoDecoder({ codec: 'h264', width: 1280, height: 720,
 *   onFrame: (f) => { ctx.drawImage(f, 0, 0); f.close(); }
 * });
 * await dec.initialize();
 * dec.decode(annexBData, isKeyframe, timestampUs);
 * ```
 */
export class VeilVideoDecoder {
  private decoder: globalThis.VideoDecoder | null = null;
  private config: VeilVideoDecoderConfig;
  private initialized = false;
  // Drop delta frames until the first keyframe arrives — WebCodecs requires
  // a keyframe after configure() and the client often connects mid-GOP.
  private waitingForKeyframe = true;

  /** Running count of decoded frames (for stats). */
  framesDecoded = 0;
  /** Running count of decode errors (dropped frames). */
  framesDropped = 0;
  /** Total bytes decoded (for bitrate estimation). */
  bytesDecoded = 0;

  constructor(config: VeilVideoDecoderConfig) {
    this.config = config;
  }

  /** Initialize the underlying WebCodecs VideoDecoder. */
  async initialize(): Promise<void> {
    if (!('VideoDecoder' in globalThis)) {
      throw new Error('WebCodecs VideoDecoder is not supported in this browser.');
    }

    const codecString = this.getCodecString(this.config.codec);

    const support = await globalThis.VideoDecoder.isConfigSupported({
      codec: codecString,
      codedWidth: this.config.width,
      codedHeight: this.config.height,
      optimizeForLatency: true,
    });

    if (!support.supported) {
      throw new Error(`Codec ${codecString} is not supported by this browser.`);
    }

    this.decoder = new globalThis.VideoDecoder({
      output: (frame: VideoFrame) => {
        this.framesDecoded++;
        try {
          this.config.onFrame(frame);
        } catch {
          frame.close();
        }
      },
      error: (err: DOMException) => {
        this.framesDropped++;
        console.error('[VeilVideoDecoder] Decode error:', err);
        this.config.onError?.(err);
        // The decoder may have transitioned to 'closed' on a fatal error.
        // Either way, drop delta frames until the next IDR and ask the server
        // for a new keyframe (FIR / Full Intra Request).
        this.waitingForKeyframe = true;
        if (this.decoder?.state === 'closed') {
          this.decoder = null;
          // Re-initialise so subsequent IDR frames can be decoded.
          this.initialized = false;
          this.initialize().catch(() => {});
        }
        this.config.onNeedKeyframe?.();
      },
    });

    this.decoder.configure({
      codec: codecString,
      codedWidth: this.config.width,
      codedHeight: this.config.height,
      optimizeForLatency: true,
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
  decode(data: BufferSource, isKeyframe: boolean, timestampUs: number): void {
    if (!this.decoder || this.decoder.state === 'closed') {
      return;
    }

    // WebCodecs requires a keyframe after configure()/flush().  Drop delta
    // frames that arrive before the first IDR (common when joining mid-GOP).
    if (this.waitingForKeyframe) {
      if (!isKeyframe) return;
      this.waitingForKeyframe = false;
    }

    this.bytesDecoded += (data as ArrayBuffer).byteLength ?? (data as ArrayBufferView).byteLength;

    try {
      const chunk = new EncodedVideoChunk({
        type: isKeyframe ? 'key' : 'delta',
        timestamp: timestampUs,
        data,
      });
      this.decoder.decode(chunk);
    } catch (err) {
      this.framesDropped++;
      console.warn('[VeilVideoDecoder] Failed to enqueue chunk:', err);
    }
  }

  /** Flush any pending frames from the decoder pipeline. */
  async flush(): Promise<void> {
    if (this.decoder && this.decoder.state === 'configured') {
      await this.decoder.flush();
    }
  }

  /**
   * Drop delta frames until the next keyframe, e.g. after a frame they
   * depend on was lost or arrived out of order.
   */
  resync(): void {
    this.waitingForKeyframe = true;
  }

  /** Close the decoder and release GPU resources. */
  close(): void {
    if (this.decoder && this.decoder.state !== 'closed') {
      this.decoder.close();
    }
    this.decoder = null;
    this.initialized = false;
    this.waitingForKeyframe = true;
  }

  get isInitialized(): boolean {
    return this.initialized;
  }

  get state(): string {
    return this.decoder?.state ?? 'closed';
  }

  /** Number of frames currently queued in the decoder pipeline. */
  get decodeQueueSize(): number {
    return this.decoder?.decodeQueueSize ?? 0;
  }

  /** True while the decoder is waiting for an IDR keyframe before it can begin decoding. */
  get isWaitingForKeyframe(): boolean {
    return this.waitingForKeyframe;
  }

  // ─── Static helpers ──────────────────────────────────────────────────────

  /** Returns true if the browser supports the WebCodecs VideoDecoder API. */
  static isSupported(): boolean {
    return 'VideoDecoder' in globalThis;
  }

  /** Returns true if the given codec/resolution combination is supported. */
  static async isCodecSupported(
    codec: VideoCodecName,
    width: number,
    height: number
  ): Promise<boolean> {
    if (!VeilVideoDecoder.isSupported()) return false;
    try {
      const codecString = defaultCodecString(codec);
      const result = await globalThis.VideoDecoder.isConfigSupported({
        codec: codecString,
        codedWidth: width,
        codedHeight: height,
        optimizeForLatency: true,
      });
      return result.supported ?? false;
    } catch {
      return false;
    }
  }

  // ─── Private helpers ─────────────────────────────────────────────────────

  private getCodecString(codec: VideoCodecName): string {
    if (this.config.codecString) return this.config.codecString;
    return defaultCodecString(codec);
  }
}
