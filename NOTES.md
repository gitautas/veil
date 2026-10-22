# Veil Build Notes

## Quick Build Instructions

### Option 1: Using the Build Script (Recommended)

```bash
# Build in debug mode
./build.sh

# Build in release mode (optimized)
./build.sh --release

# Build and run
./build.sh --run -- --seconds 5 --out out.h264 --size 1280x720 --fps 60 --codec h264 -- vkcube

# Check without building
./build.sh --check

# Clean build artifacts
./build.sh --clean
```

### Option 2: Manual Build

```bash
# 1. Source the Vulkan SDK (required for every build session)
source .vulkanSDK/setup-env.sh

# 2. Set kernel version to work around southpaw build issue
export KERNEL_VERSION=$(uname -r | sed 's/-.*//g')

# 3. Build (first time downloads shader-slang from GitHub)
cargo build

# 4. Build release (optimized)
cargo build --release
```

## Running Veil

### Basic Usage

```bash
# Run with vkcube (simple test)
RUST_LOG=info cargo run -- --seconds 5 --out out.h264 --size 1280x720 --fps 60 --codec h264 -- vkcube

# Run with XWayland support (for X11 apps)
RUST_LOG=info cargo run -- --xwayland --seconds 10 --out out.h264 -- glxgears

# Run without time limit (Ctrl+C to stop)
RUST_LOG=info cargo run -- --out out.h264 --size 1920x1080 --fps 60 -- weston-terminal
```

### Command Line Options

```
veil [OPTIONS] -- <COMMAND> [ARGS...]

OPTIONS:
  --out <PATH>          Output file for encoded stream (default: out.h264)
  --size <WxH>          Resolution (default: 1280x720)
  --fps <N>             Framerate (default: 60)
  --codec <CODEC>       Video codec: h264 or h265 (default: h264)
  --xwayland            Enable XWayland (default: true)
  --no-xwayland         Disable XWayland
  --force-1x            Force 1x UI scaling
  --seconds <N>         Max duration in seconds (for testing)
  --webrtc              Enable WebRTC streaming (coming soon)
  --webrtc-port <N>     WebRTC signaling server port (default: 8080)

EXAMPLES:
  # Test with vkcube
  cargo run -- --seconds 5 --out out.h264 --size 1280x720 --fps 60 --codec h264 -- vkcube
  
  # Run Steam
  cargo run -- --xwayland --size 1920x1080 --fps 60 -- steam
  
  # Encode to H.265
  cargo run -- --codec h265 --out output.h265 -- glxgears
  
  # WebRTC streaming (after integration)
  cargo run -- --webrtc --webrtc-port 8080 -- steam
```

## Verifying Output

```bash
# Play encoded video with ffplay
ffplay out.h264

# Or with VLC
vlc out.h264

# Check video info
ffprobe out.h264
```

## Environment Variables

### Required

- `VULKAN_SDK` or `SLANG_DIR` - Path to Vulkan SDK (set by setup-env.sh)
- `KERNEL_VERSION` - Kernel version for southpaw (e.g., "6.18.5")

### Optional

- `RUST_LOG` - Logging level (error, warn, info, debug, trace)
  ```bash
  RUST_LOG=debug cargo run -- ...     # All debug logs
  RUST_LOG=veil::webrtc=trace ...    # Specific module
  ```

- `WAYLAND_DEBUG` - Wayland protocol debug output
  ```bash
  WAYLAND_DEBUG=1 cargo run -- ...
  ```

## Troubleshooting

### Build Errors

#### "VULKAN_SDK must be set"

```bash
# Solution: Source the Vulkan SDK setup script
source .vulkanSDK/setup-env.sh

# Or use the build script
./build.sh
```

#### "invalid semver: unexpected character '_'"

This is a southpaw (evdev library) build error with certain kernel versions.

```bash
# Solution 1: Use the build script (handles this automatically)
./build.sh

# Solution 2: Set KERNEL_VERSION manually
export KERNEL_VERSION=$(uname -r | sed 's/-.*//g')
cargo build
```

#### Missing system libraries

```bash
# Ubuntu/Debian
sudo apt install \
    build-essential \
    pkg-config \
    libwayland-dev \
    libxkbcommon-dev \
    libudev-dev \
    libinput-dev \
    libgbm-dev \
    libdrm-dev \
    libpulse-dev \
    libvulkan-dev \
    vulkan-tools \
    mesa-vulkan-drivers

# Fedora
sudo dnf install \
    gcc \
    pkg-config \
    wayland-devel \
    libxkbcommon-devel \
    systemd-devel \
    libinput-devel \
    libgbm-devel \
    libdrm-devel \
    pulseaudio-libs-devel \
    vulkan-loader-devel \
    vulkan-tools \
    mesa-vulkan-drivers
```

### Runtime Errors

#### "failed to find a suitable physical device"

Your GPU doesn't support Vulkan Video encoding.

```bash
# Check Vulkan support
vulkaninfo | grep -A 10 "VK_KHR_video_encode"

# List supported codecs
vulkaninfo | grep "VK_VIDEO_CODEC"
```

#### "Permission denied" for input devices

```bash
# Add user to input group
sudo usermod -a -G input $USER

# Log out and back in for changes to take effect
```

#### "failed to create Wayland socket"

Another compositor might be running.

```bash
# Check for existing Wayland displays
ls /run/user/$(id -u)/wayland-*

# Kill other compositors if needed
pkill -9 weston
pkill -9 sway
```

## Development Workflow

### Quick Iteration

```bash
# Check for errors without building
cargo check

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy

# Watch for changes and rebuild
cargo watch -x check -x test
```

### Building Web Client

```bash
cd web-client/sdk

# Install dependencies
npm install

# Build for development (with watch)
npm run dev

# Build for production
npm run build

# Run tests
npm test

# Lint
npm run lint
```

### Full System Test

```bash
# Terminal 1: Build and run server with WebRTC
source .vulkanSDK/setup-env.sh
export KERNEL_VERSION=$(uname -r | sed 's/-.*//g')
RUST_LOG=info cargo run -- --webrtc --size 1920x1080 --fps 60 -- steam

# Terminal 2: Build and serve web client
cd web-client/sdk
npm install && npm run build
cd ../demo
python3 -m http.server 3000

# Browser: Open http://localhost:3000
```

## Performance Tips

### For Best Performance

1. **Use release builds**:
   ```bash
   cargo build --release
   ./target/release/veil ...
   ```

2. **Enable GPU encoding**: Ensure your GPU supports Vulkan Video

3. **Use appropriate preset**:
   - Preset 0: Ultra-low latency (higher CPU)
   - Preset 1: Balanced
   - Preset 2: Efficiency (lower CPU)

4. **Adjust resolution/framerate**:
   ```bash
   # Lower for weak networks
   --size 1280x720 --fps 30
   
   # Higher for local streaming
   --size 2560x1440 --fps 120
   ```

### Profiling

```bash
# CPU profiling
cargo build --release
perf record -g ./target/release/veil -- --seconds 30 -- vkcube
perf report

# GPU profiling (with RenderDoc)
renderdoc-capture ./target/release/veil -- --seconds 10 -- vkcube
```

## Useful Commands

```bash
# Check Vulkan Video support
vulkaninfo | grep -A 5 "VK_KHR_video"

# List available Vulkan devices
vulkaninfo | grep "deviceName"

# Monitor GPU usage (NVIDIA)
watch -n 1 nvidia-smi

# Monitor GPU usage (AMD)
watch -n 1 radeontop

# Check PulseAudio
pactl list sources

# List input devices
ls -l /dev/input/

# Check kernel version
uname -r
```

## Additional Resources

- **Main README**: `README.md` - Project overview and features
- **Development Guide**: `DEVELOPMENT.md` - Architecture and integration details
- **Implementation Summary**: `IMPLEMENTATION_SUMMARY.md` - Current status and next steps
- **SDK Documentation**: `web-client/sdk/README.md` - TypeScript SDK usage

## Getting Help

1. Check the documentation files listed above
2. Run with `RUST_LOG=debug` for detailed logs
3. Check GitHub issues for known problems
4. Ensure all dependencies are installed
5. Verify Vulkan SDK is properly sourced

## Known Issues

1. **Kernel version parsing**: Fixed in build script via KERNEL_VERSION
2. **WebRTC integration**: Implementation complete, integration in progress
3. **Multi-client support**: Not yet implemented
4. **Audio streaming**: Infrastructure exists, needs integration
5. **Rust edition 2024**: Requires nightly Rust toolchain

---

Last updated: 2024