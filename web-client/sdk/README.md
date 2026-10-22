# Veil SDK

TypeScript/JavaScript SDK for connecting to Veil streaming servers for ultra-low latency cloud gaming and remote desktop streaming.

## Features

- 🚀 **Ultra-low latency** streaming using WebRTC
- 🎮 **Full input support** for keyboard, mouse, and gamepad
- 🎥 **Hardware-accelerated** video decoding (H.264/H.265)
- 🔊 **Audio streaming** with Opus codec
- 📊 **Real-time statistics** for monitoring connection quality
- 🎯 **Pointer lock** support for gaming applications
- 🔌 **Simple API** for easy integration

## Installation

```bash
npm install @veil/sdk
# or
yarn add @veil/sdk
# or
pnpm add @veil/sdk
```

## Quick Start

```typescript
import { VeilClient } from '@veil/sdk';

// Create a client instance
const client = new VeilClient({
  signalingUrl: 'http://localhost:8080',
  videoCodec: 'h264',
  width: 1920,
  height: 1080,
  framerate: 60,
  debug: true,
});

// Get video element
const videoElement = document.getElementById('video') as HTMLVideoElement;

// Connect to the server
await client.connect(videoElement);

// Listen for events
client.on('connected', () => {
  console.log('Connected to Veil server!');
});

client.on('stats', (stats) => {
  console.log('FPS:', stats.video.fps);
  console.log('Latency:', stats.video.latency, 'ms');
});

client.on('error', (error) => {
  console.error('Connection error:', error);
});

// Disconnect when done
await client.disconnect();
```

## Configuration

### VeilClientConfig

```typescript
interface VeilClientConfig {
  // Required: URL of the Veil signaling server
  signalingUrl: string;

  // Optional: STUN/TURN servers for WebRTC
  iceServers?: RTCIceServer[];

  // Optional: Video codec (default: 'h264')
  videoCodec?: 'h264' | 'h265';

  // Optional: Video resolution (default: 1280x720)
  width?: number;
  height?: number;

  // Optional: Target framerate (default: 60)
  framerate?: number;

  // Optional: Enable debug logging (default: false)
  debug?: boolean;

  // Optional: Input capture settings (default: true for all)
  captureKeyboard?: boolean;
  captureMouse?: boolean;
  captureGamepad?: boolean;

  // Optional: Enable pointer lock on click (default: false)
  pointerLock?: boolean;

  // Optional: Custom peer ID
  peerId?: string;
}
```

## Events

The VeilClient emits the following events:

### Connection Events

```typescript
// Connection state changed
client.on('statechange', (state: ConnectionState) => {
  console.log('State:', state); // 'disconnected' | 'connecting' | 'connected' | 'failed' | 'closed'
});

// Successfully connected
client.on('connected', () => {
  console.log('Connected!');
});

// Disconnected from server
client.on('disconnected', () => {
  console.log('Disconnected');
});

// Connection error
client.on('error', (error: Error) => {
  console.error('Error:', error);
});
```

### Stream Events

```typescript
// Video stream started
client.on('videostart', () => {
  console.log('Video streaming started');
});

// Audio stream started
client.on('audiostart', () => {
  console.log('Audio streaming started');
});
```

### Statistics

```typescript
// Statistics updated (every second)
client.on('stats', (stats: ConnectionStats) => {
  console.log('Video FPS:', stats.video.fps);
  console.log('Video bitrate:', stats.video.bitrate);
  console.log('Latency:', stats.video.latency, 'ms');
  console.log('Packet loss:', stats.packetLoss, '%');
});
```

### Display Events

```typescript
// Display parameters changed
client.on('displaychange', ({ width, height, framerate }) => {
  console.log(`Display: ${width}x${height}@${framerate}Hz`);
});

// Cursor update
client.on('cursor', (cursor: CursorUpdate) => {
  console.log('Cursor:', cursor.icon || 'custom');
});

// Pointer locked
client.on('pointerlock', ({ x, y }) => {
  console.log('Pointer locked at', x, y);
});

// Pointer released
client.on('pointerrelease', () => {
  console.log('Pointer released');
});
```

## Input Management

The SDK automatically captures keyboard, mouse, and gamepad input when configured. You can control this behavior through the configuration:

```typescript
const client = new VeilClient({
  signalingUrl: 'http://localhost:8080',
  captureKeyboard: true,  // Capture keyboard
  captureMouse: true,     // Capture mouse
  captureGamepad: true,   // Capture gamepad
  pointerLock: true,      // Request pointer lock on click
});
```

### Supported Input

- **Keyboard**: All standard keys including modifiers (Ctrl, Shift, Alt, Meta)
- **Mouse**: Movement, buttons (left, right, middle, back, forward), and wheel
- **Gamepad**: Standard gamepad layout (Xbox/PlayStation style) with buttons and axes

## Statistics

Get real-time connection statistics:

```typescript
const stats = await client.getStats();

if (stats) {
  console.log('Video:', {
    fps: stats.video.fps,
    bitrate: stats.video.bitrate,
    resolution: `${stats.video.width}x${stats.video.height}`,
    codec: stats.video.codec,
    latency: stats.video.latency,
    framesReceived: stats.video.framesReceived,
    framesDropped: stats.video.framesDropped,
  });

  console.log('Audio:', {
    bitrate: stats.audio.bitrate,
    codec: stats.audio.codec,
    sampleRate: stats.audio.sampleRate,
  });

  console.log('Connection:', {
    rtt: stats.rtt,
    packetLoss: stats.packetLoss,
    bandwidth: stats.bandwidth,
  });
}
```

## Advanced Usage

### Custom ICE Servers

```typescript
const client = new VeilClient({
  signalingUrl: 'http://localhost:8080',
  iceServers: [
    { urls: 'stun:stun.l.google.com:19302' },
    {
      urls: 'turn:turn.example.com:3478',
      username: 'user',
      credential: 'pass',
    },
  ],
});
```

### Request Video Refresh

Request a keyframe when experiencing video issues:

```typescript
await client.requestRefresh();
```

### Manual Input Management

For advanced use cases, you can disable automatic input capture and use the InputManager directly:

```typescript
import { InputManager } from '@veil/sdk';

const inputManager = new InputManager(
  videoElement,
  true,  // keyboard
  true,  // mouse
  true,  // gamepad
  false  // pointer lock
);

inputManager.on('input', (event) => {
  console.log('Input event:', event);
  // Send to server manually
});

// Clean up
inputManager.stop();
```

## Browser Support

The Veil SDK requires:
- **WebRTC** support (all modern browsers)
- **WebCodecs API** (optional, for advanced video decoding)
- **Gamepad API** (optional, for gamepad support)
- **Pointer Lock API** (optional, for FPS-style games)

Supported browsers:
- Chrome/Edge 89+
- Firefox 100+
- Safari 15.4+

## Examples

Check out the [demo application](../demo) for a complete working example.

## API Reference

### VeilClient

Main client class for managing connections and streams.

#### Methods

- `constructor(config: VeilClientConfig)` - Create a new client instance
- `connect(videoElement: HTMLVideoElement): Promise<void>` - Connect to server
- `disconnect(): Promise<void>` - Disconnect from server
- `requestRefresh(): Promise<void>` - Request video keyframe
- `getStats(): Promise<ConnectionStats | null>` - Get connection statistics
- `on(event, listener)` - Add event listener
- `off(event, listener)` - Remove event listener
- `getState(): ConnectionState` - Get current connection state
- `getPeerId(): string` - Get peer ID

### InputManager

Manages input capture from keyboard, mouse, and gamepad.

#### Methods

- `constructor(element, keyboard, mouse, gamepad, pointerLock)` - Create input manager
- `on(event, handler)` - Add input event handler
- `off(event, handler)` - Remove input event handler
- `stop()` - Stop input capture
- `requestPointerLock(): Promise<void>` - Request pointer lock
- `exitPointerLock()` - Exit pointer lock
- `isPointerLocked(): boolean` - Check if pointer is locked

### SignalingClient

Handles WebRTC signaling with the server.

#### Methods

- `constructor(url, peerId)` - Create signaling client
- `connect(): Promise<void>` - Connect to signaling server
- `disconnect(): Promise<void>` - Disconnect from server
- `sendOffer(sdp): Promise<void>` - Send SDP offer
- `sendAnswer(sdp): Promise<void>` - Send SDP answer
- `sendIceCandidate(candidate): Promise<void>` - Send ICE candidate
- `isConnected(): boolean` - Check connection status
- `getPeerId(): string` - Get peer ID

## License

MIT

## Contributing

Contributions are welcome! Please see the main repository for guidelines.

## Support

For issues and questions, please visit the [GitHub repository](https://github.com/yourusername/veil).