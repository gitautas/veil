#!/bin/bash
# Veil Build Helper Script
# This script sets up the required environment variables and builds the project

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Veil Build Script ===${NC}"
echo ""

# Check if Vulkan SDK directory exists
VULKAN_SDK_DIR=".vulkanSDK"
if [ ! -d "$VULKAN_SDK_DIR" ]; then
    echo -e "${YELLOW}Warning: Vulkan SDK directory not found at $VULKAN_SDK_DIR${NC}"
    echo "Please ensure the Vulkan SDK is installed and the directory exists."
    echo ""
fi

# Source Vulkan SDK if setup script exists
if [ -f "$VULKAN_SDK_DIR/setup-env.sh" ]; then
    echo -e "${GREEN}✓ Sourcing Vulkan SDK environment...${NC}"
    source "$VULKAN_SDK_DIR/setup-env.sh"
else
    echo -e "${YELLOW}⚠ Vulkan SDK setup script not found${NC}"
    echo "Attempting to use system Vulkan SDK..."

    # Try to find system Vulkan SDK
    if [ -n "$VULKAN_SDK" ]; then
        echo -e "${GREEN}✓ Using system VULKAN_SDK: $VULKAN_SDK${NC}"
    else
        echo -e "${RED}✗ VULKAN_SDK not set!${NC}"
        echo ""
        echo "Please set VULKAN_SDK environment variable or install the SDK to $VULKAN_SDK_DIR"
        echo ""
        echo "You can download the Vulkan SDK from:"
        echo "  https://vulkan.lunarg.com/sdk/home"
        echo ""
        echo "Or install via package manager:"
        echo "  Ubuntu/Debian: sudo apt install libvulkan-dev vulkan-tools"
        echo "  Fedora: sudo dnf install vulkan-loader-devel vulkan-tools"
        echo ""
        exit 1
    fi
fi

# Set KERNEL_VERSION to work around southpaw build issue
# This strips the problematic suffix from the kernel version
KERNEL_FULL=$(uname -r)
KERNEL_VERSION=$(echo "$KERNEL_FULL" | sed 's/-.*//g')
export KERNEL_VERSION
echo -e "${GREEN}✓ Set KERNEL_VERSION: $KERNEL_VERSION (from $KERNEL_FULL)${NC}"

# Check for required system dependencies
echo ""
echo -e "${GREEN}Checking system dependencies...${NC}"

check_lib() {
    local lib=$1
    local pkg=$2
    if ldconfig -p | grep -q "$lib"; then
        echo -e "${GREEN}  ✓ $lib${NC}"
        return 0
    else
        echo -e "${RED}  ✗ $lib (install $pkg)${NC}"
        return 1
    fi
}

MISSING_DEPS=0

check_lib "libwayland-server" "libwayland-dev" || MISSING_DEPS=1
check_lib "libxkbcommon" "libxkbcommon-dev" || MISSING_DEPS=1
check_lib "libvulkan" "libvulkan-dev" || MISSING_DEPS=1
check_lib "libpulse" "libpulse-dev" || MISSING_DEPS=1
check_lib "libinput" "libinput-dev" || MISSING_DEPS=1
check_lib "libgbm" "libgbm-dev" || MISSING_DEPS=1
check_lib "libdrm" "libdrm-dev" || MISSING_DEPS=1

if [ $MISSING_DEPS -eq 1 ]; then
    echo ""
    echo -e "${YELLOW}Some dependencies are missing. Install them with:${NC}"
    echo ""
    echo "  Ubuntu/Debian:"
    echo "    sudo apt install libwayland-dev libxkbcommon-dev libvulkan-dev \\"
    echo "                     libpulse-dev libinput-dev libgbm-dev libdrm-dev \\"
    echo "                     libudev-dev pkg-config build-essential"
    echo ""
    echo "  Fedora:"
    echo "    sudo dnf install wayland-devel libxkbcommon-devel vulkan-loader-devel \\"
    echo "                     pulseaudio-libs-devel libinput-devel libgbm-devel \\"
    echo "                     libdrm-devel systemd-devel pkg-config gcc"
    echo ""
fi

# Parse command line arguments
BUILD_TYPE="debug"
CARGO_ARGS=""
RUN_AFTER_BUILD=0
RUN_ARGS=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            BUILD_TYPE="release"
            CARGO_ARGS="--release"
            shift
            ;;
        --run)
            RUN_AFTER_BUILD=1
            shift
            # Collect remaining args for running
            RUN_ARGS="$@"
            break
            ;;
        --clean)
            echo -e "${GREEN}Cleaning build artifacts...${NC}"
            cargo clean
            exit 0
            ;;
        --check)
            echo -e "${GREEN}Running cargo check...${NC}"
            cargo check
            exit 0
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --release         Build in release mode (optimized)"
            echo "  --run [ARGS...]   Run after building (pass args after --)"
            echo "  --check           Only run cargo check"
            echo "  --clean           Clean build artifacts"
            echo "  --help, -h        Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                           # Build in debug mode"
            echo "  $0 --release                 # Build in release mode"
            echo "  $0 --run -- --seconds 5 -- vkcube"
            echo "  $0 --release --run -- --xwayland -- steam"
            echo ""
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Print build configuration
echo ""
echo -e "${GREEN}=== Build Configuration ===${NC}"
echo "  Build type: $BUILD_TYPE"
echo "  Vulkan SDK: ${VULKAN_SDK:-system}"
echo "  Kernel version: $KERNEL_VERSION"
echo ""

# Build the project
echo -e "${GREEN}Building veil ($BUILD_TYPE)...${NC}"
echo ""

if cargo build $CARGO_ARGS; then
    echo ""
    echo -e "${GREEN}✓ Build successful!${NC}"

    if [ $RUN_AFTER_BUILD -eq 1 ]; then
        echo ""
        echo -e "${GREEN}Running veil...${NC}"
        echo ""

        if [ "$BUILD_TYPE" = "release" ]; then
            RUST_LOG="${RUST_LOG:-info}" ./target/release/veil $RUN_ARGS
        else
            RUST_LOG="${RUST_LOG:-info}" ./target/debug/veil $RUN_ARGS
        fi
    else
        echo ""
        echo "Binary location:"
        if [ "$BUILD_TYPE" = "release" ]; then
            echo "  ./target/release/veil"
        else
            echo "  ./target/debug/veil"
        fi
        echo ""
        echo "Run with:"
        echo "  RUST_LOG=info ./target/$BUILD_TYPE/veil --help"
    fi
else
    echo ""
    echo -e "${RED}✗ Build failed!${NC}"
    echo ""
    echo "Common issues:"
    echo "  1. Vulkan SDK not found: source .vulkanSDK/setup-env.sh"
    echo "  2. Missing dependencies: install system packages (see above)"
    echo "  3. Kernel version issue: set KERNEL_VERSION manually"
    echo ""
    exit 1
fi
