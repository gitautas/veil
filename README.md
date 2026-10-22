# Veil

Ultra‑low latency Wayland/X11 streaming with Vulkan Video encode and WebRTC signaling.

## What’s here (short)
- Rust server: Wayland compositor (smithay), Vulkan Video H.264/H.265 encoder (ash, KHR video), WebRTC signaling/peer plumbing (webrtc-rs).
- Web client: TypeScript SDK and demo (browser WebRTC decode + input).
- Entry point: `src/main.rs` (flags below) runs compositor + encoder; `--webrtc` starts HTTP/SSE signaling + RTP sender scaffolding.

## Requirements
- Rust 1.70+ (edition 2024)
- Vulkan SDK 1.3+ (source `.vulkanSDK/setup-env.sh` per shell)
- Linux with render node `/dev/dri/renderD*`
- Node.js 18+ (for web client)

## Build & run (server)
```bash
source .vulkanSDK/setup-env.sh

# Build
cargo build --release

# Minimal local run (no WebRTC, writes H.264 file)
RUST_LOG=info cargo run --release -- \
  --seconds 5 --out out.h264 --size 1280x720 --fps 60 --codec h264 -- vkcube

# WebRTC signaling + RTP sender scaffold (HTTP on default 8080)
RUST_LOG=info cargo run --release -- \
  --webrtc --webrtc-port 8080 --size 1920x1080 --fps 60 --codec h264 -- vkcube
```

Flags (common):
- `--webrtc` enable signaling/RTP server
- `--webrtc-port <port>` (default 8080)
- `--size <WxH>` (default 1280x720)
- `--fps <N>` (default 60)
- `--codec {h264|h265}` (default h264)
- `--seconds <N>` stop after N seconds (testing)
- `--out <path>` write elementary stream to file
- `--xwayland` / `--no-xwayland` toggle Xwayland

## Known runtime gotchas
- If the launched app exits immediately (e.g., Steam first-run), reactor reports: `client exited without doing anything`. Test with `vkcube`/`glxgears`/`weston-terminal` first.
- Ensure Vulkan render node is selected; log shows chosen GPU and encode support.
- For RADV, enable video: `RADV_PERFTEST=video_encode,video_decode` (set automatically in code).

## Web client (SDK + demo)
```bash
cd web-client/sdk
npm install
npm run build

cd ../demo
python3 -m http.server 3000
# open http://localhost:3000 and point signaling to http://localhost:8080
```

## Project layout
```
src/                    # Rust server
  main.rs               # CLI + startup (WebRTC flags)
  encoder/              # Vulkan Video H.264/H.265
  session/              # Wayland compositor, reactor, audio scaffolding
  webrtc/               # signaling, peers, RTP sender scaffold
web-client/
  sdk/                  # TypeScript SDK
  demo/                 # Reference UI
```

## WebRTC signaling (current)
- HTTP POST `/offer`, `/answer`, `/ice-candidate`; SSE `/connect/:peerId`; POST `/disconnect/:peerId`.
- RTP sender scaffold present; video payload wiring from encoder to RTP still in-progress.

## Input and control
- Browser captures keyboard/mouse/gamepad; server injects via evdev into compositor.
- Data channel plumbing exists; message parsing to session control is pending.

## Troubleshooting
- Vulkan SDK not found: `source .vulkanSDK/setup-env.sh`
- Permissions: add user to `input` group for evdev.
- Enable verbose logs: `RUST_LOG=debug` or `trace`.

## Features & roadmap (abridged)
- Wayland compositor with Xwayland ✅
- Vulkan Video H.264/H.265 encode ✅
- WebRTC signaling ✅ (media wiring WIP)
- Audio (Opus) WIP
- Multi-client, auth, reconnection WIP

## Optional profiling
- `tracy` feature available: `cargo run --features tracy --release -- --webrtc -- vkcube`
