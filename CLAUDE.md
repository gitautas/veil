# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

Veil is a self-hosted cloud gaming platform: a Rust Wayland compositor (smithay) captures a GPU application (Steam, vkcube, etc.), encodes frames via Vulkan Video (H.264/H.265), and streams over QUIC/WebTransport to a browser client that decodes with WebCodecs.

## Build Requirements

- Vulkan SDK at `.vulkanSDK/` (source `.vulkanSDK/setup-env.sh` before building)
- System libs: `libwayland`, `libxkbcommon`, `libvulkan`, `libpulse`, `libinput`, `libgbm`, `libdrm`, `libudev`
- Node.js 24+ (web client)

**Always set env before building:**
```bash
source .vulkanSDK/setup-env.sh
export VULKAN_SDK=.vulkanSDK/x86_64
export LD_LIBRARY_PATH=$VULKAN_SDK/lib
```

## Commands

### Rust Backend
```bash
# Check (fast)
./build.sh --check
# or: source .vulkanSDK/setup-env.sh && cargo check

# Build debug
./build.sh

# Build release
./build.sh --release

# Run (file output mode, no browser needed)
RUST_LOG=info cargo run -- --seconds 5 --out out.h264 -- vkcube

# Run with WebTransport server (browser streaming)
RUST_LOG=info cargo run -- --transport --transport-addr 0.0.0.0:7801 -- vkcube
```

### Web Client (demo)
```bash
cd web-client/demo
npm install
npm run build        # one-shot
npm run dev          # watch mode
npm run serve        # HTTP server on :7800
```

### Run Everything at Once
```bash
./run.sh -- vkcube
./run.sh --release -- steam -gamepadui
./run.sh --port 7801 --web-port 7800 -- glxgears
```
`run.sh` builds Rust, starts the esbuild watcher, serves the demo, and launches the backend. Open Chrome (WebTransport required) at `http://<host>:7800`.

## Architecture

### Data Flow
```
App (Steam/vkcube) → Wayland compositor (smithay)
  → Vulkan Video encoder (H.264/H.265)
  → SessionEvent::VideoFrame broadcast channel
  → WebTransport QUIC stream (per client)
  → Browser WebCodecs decoder
```

### Key Source Modules

| Path | Role |
|------|------|
| `src/main.rs` | CLI flags, wires all modules together |
| `src/session/reactor.rs` | Main compositor event loop (smithay, mio); runs on a dedicated thread |
| `src/session/compositor/` | Smithay Wayland compositor state |
| `src/session/control.rs` | `ControlMessage` + `SessionEvent` enums — the internal message bus |
| `src/session/handle.rs` | `SessionHandle`: `Arc<broadcast::Sender<SessionEvent>>` + mio Waker |
| `src/encoder/` | Vulkan Video encode pipeline (`h264.rs`, `h265.rs`, experimental `av1.rs`, DPB, rate control) |
| `src/transport/mod.rs` | `TransportServer`: WebTransport QUIC + HTTP cert endpoint (axum) |
| `src/transport/session.rs` | Per-client handler; subscribes to broadcast channel, sends frames |
| `src/transport/proto.rs` | Binary wire protocol (encode/decode) |
| `src/vulkan.rs` | Vulkan instance/device setup |
| `web-client/sdk/src/VeilClient.ts` | TypeScript WebTransport client |
| `web-client/sdk/src/VideoDecoder.ts` | WebCodecs H.264/H.265 decoder |
| `web-client/sdk/src/constants.ts` | Wire protocol constants (mirrors `proto.rs`) |

> `src/webrtc/` is orphaned (not compiled) — safe to ignore.

### Internal Message Bus

All fan-out uses a single `tokio::sync::broadcast::channel::<SessionEvent>(512)`:
- **Reactor** sends events via `SessionHandle` (sync-safe, no tokio context needed)
- **Each transport client** subscribes independently — no relay tasks or bridges
- `ControlMessage` flows the other direction (client → compositor) via a `WakingSender<ControlMessage>`

### Adaptation (`src/transport/adapt.rs`)

One encoder serves every viewer, so it follows the most constrained one:
- **Bitrate**: each session times every video frame from write to the viewer's ack (`SendStream::finish`), and reads QUIC path stats (min RTT, loss). Once a second an AIMD estimator cuts its budget when frames queue (delivery > min RTT + 40 ms), packets drop (> 5%) or the viewer lags, and grows it 15%/s when clear. The lowest budget becomes `ControlMessage::SetBitrateLimit`, applied live (no keyframe) via `vkCmdControlVideoCoding`. Only VBR presets (0–6) adapt.
- **Codec (Auto)**: browsers report decode support (`0x32`, hardware/software/none per codec, plus codecs whose decoder fell behind); the transport picks the best codec all viewers share and sends `SetStreamParams`.
- Viewers get `0x1C` (JSON) each second with their RTT, queueing, loss, budget and the Auto reason.

### Wire Protocol

Server → client QUIC streams (byte prefix):
- `0x00` hello, `0x01` video frame, `0x02` audio, `0x10–0x15` control (0x15 = window list)

Client → server QUIC datagrams:
- `0x20` keydown, `0x21` keyup, `0x22` mousemove, `0x23–0x2E` mouse/scroll/gamepad/focus

### QUIC/WebTransport Notes
- Crate: `wtransport = "0.7"` (not 0.5 — incompatible API)
- Self-signed cert fingerprint served at `http://<host>:<quic_port+1>/cert-hash`
- `open_uni().await?.await?` — double await required to get `SendStream`
- `send_datagram(payload)?` — synchronous, not async

## Runtime Flags (common)

| Flag | Default | Description |
|------|---------|-------------|
| `--transport` | off | Enable QUIC/WebTransport server |
| `--transport-addr` | `0.0.0.0:7801` | Bind address (cert-hash HTTP on port+1) |
| `--size WxH` | `1920x1080` | Compositor resolution |
| `--fps N` | 60 | Encode framerate |
| `--codec auto\|h264\|h265\|av1` | auto | Video codec. `auto` picks what every viewer decodes best (hardware first); AV1 is experimental (RADV only here) |
| `--out <path>` | — | Write elementary stream to file |
| `--seconds N` | — | Stop after N seconds |
| `--xwayland` / `--no-xwayland` | on | Toggle Xwayland |

## Gotchas

- **AV1** (experimental): needs `VK_KHR_video_encode_av1` (the RX 7600 has it, the GTX 1660 doesn't; unsupported codecs fall back to H.264). Flat P structure, no temporal layers. RADV pads frames (1080 → 1088 rows), and Chrome ignores the AV1 render size, so the client crops to the stream size. Chrome here decodes AV1 in software, which can't hold 1080p60.
- **Wayland clients**: the compositor is hand-written (`src/session/compositor/dispatch/`), one module per protocol. Apps get `WAYLAND_DISPLAY` as an absolute socket path (their `XDG_RUNTIME_DIR` is Veil's private run dir). Subsurfaces are supported (all treated as desynchronized; input goes to the window), decorations are always "server-side" (none drawn), and xdg popups aren't supported yet (dismissed immediately, so app menus don't open). X11-capable apps hide gaps here by falling back to Xwayland.
- **RADV**: enable video encode/decode with `RADV_PERFTEST=video_encode,video_decode` (set automatically in code)
- **Steam**: single-instance; `run.sh` shuts down existing Steam and clears stale pid/lock files before launch
- **Browser**: Chrome only — Safari and Firefox lack WebTransport support
- **Gamepads**: user must be in the `input` group, with `udev/61-veil.rules` installed (uinput for Xbox 360 pads; uhid and hidraw for DualSense). PlayStation pads (Sony vendor 054c) are emulated as a DualSense that the kernel's `hid-playstation` driver binds to (`src/session/dualsense.rs`); everything else as an Xbox 360 pad. Without uhid access a DualSense falls back to Xbox. The pad type rides on `0x2C` (available) and `0x31` (snapshot) datagrams: 0 = Xbox 360, 1 = DualSense.
- **HID passthrough** (`src/transport/hid.rs`, `web-client/sdk/src/HidPassthrough.ts`): devices a viewer opens with WebHID are recreated per viewer with uhid (`src/session/uhid.rs`, shared with the emulated DualSense). WebHID only exposes parsed collections, so the SDK rebuilds the report descriptor. Open/close/replies go on client→server streams (`0x40`/`0x41`/`0x43`), input reports as datagrams (`0x42`); the server sends output and feature requests (`0x1D`) and open status (`0x1E`). Keyboards, mice, consumer controls and digitizers are refused server-side: uhid devices are system-wide, so they'd act on the host desktop (browsers keep them from WebHID anyway). A passed-through pad isn't also emulated.
- **Profiling**: `cargo run --features tracy --release -- ...`
