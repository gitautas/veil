/**
 * @veil/sdk — TypeScript client for ultra-low latency cloud gaming / remote desktop
 *
 * Transport: QUIC / WebTransport (no WebRTC, no SDP, no ICE)
 * Video:     WebCodecs VideoDecoder → Canvas2D
 * Input:     QUIC datagrams (keyboard / mouse / gamepad)
 */

export { VeilClient, detectGamepadLayout } from './VeilClient';
export { buildReportDescriptor, guessBus } from './HidPassthrough';
export type { HidDevice } from './HidPassthrough';
export { VeilVideoDecoder } from './VideoDecoder';
export { InputManager } from './InputManager';
export { AudioPlayer } from './AudioPlayer';
export type { AudioPlayerStats } from './AudioPlayer';
export * from './types';
export * from './constants';
