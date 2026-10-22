#!/bin/bash
# Veil — run everything at once:
#   1. Build + watch the TypeScript demo (esbuild --watch)
#   2. Serve the demo over HTTP (http-server)
#   3. Build (if needed) and run the Rust backend with --transport
#
# Usage:
#   ./run.sh [-- <app-command> [args...]]
#
# Examples:
#   ./run.sh -- vkcube
#   ./run.sh -- steam -gamepadui
#   ./run.sh --release -- glxgears
#
# Options:
#   --release        Build Rust in release mode
#   --size WxH       Compositor resolution (default: 1920x1080)
#   --fps N          Framerate (default: 60)
#   --codec h264|h265
#   --port N         QUIC/WebTransport port (default: 7801; cert-hash HTTP on N+1)
#   --web-port N     HTTP demo server port (default: 7800)
#   --no-build       Skip cargo build (use existing binary)

set -euo pipefail

# ── Colours ──────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; NC='\033[0m'

log()  { echo -e "${CYAN}[veil]${NC} $*"; }
ok()   { echo -e "${GREEN}[veil]${NC} $*"; }
warn() { echo -e "${YELLOW}[veil]${NC} $*"; }
die()  { echo -e "${RED}[veil] ERROR:${NC} $*" >&2; exit 1; }

# ── Defaults ─────────────────────────────────────────────────────────────────
RELEASE=0
SIZE="1920x1080"
FPS=60
CODEC="h264"
QUIC_PORT=7801
WEB_PORT=7800
NO_BUILD=0
APP_CMD=()

# ── Argument parsing ──────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case $1 in
    --release)   RELEASE=1;         shift ;;
    --size)      SIZE=$2;           shift 2 ;;
    --fps)       FPS=$2;            shift 2 ;;
    --codec)     CODEC=$2;          shift 2 ;;
    --port)      QUIC_PORT=$2;      shift 2 ;;
    --web-port)  WEB_PORT=$2;       shift 2 ;;
    --no-build)  NO_BUILD=1;        shift ;;
    --)          shift; APP_CMD=("$@"); break ;;
    -h|--help)
      sed -n '/^# Usage:/,/^[^#]/p' "$0" | grep '^#' | sed 's/^# \?//'
      exit 0 ;;
    *) die "Unknown option: $1  (use -- to separate the app command)" ;;
  esac
done

[[ ${#APP_CMD[@]} -eq 0 ]] && die "No app command given. Example: ./run.sh -- vkcube"

# ── Node / nvm ───────────────────────────────────────────────────────────────
NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
if [[ -s "$NVM_DIR/nvm.sh" ]]; then
  source "$NVM_DIR/nvm.sh"
  nvm use 24 --silent || warn "nvm use 24 failed; continuing with current node"
fi
command -v npm &>/dev/null || die "npm not found. Install Node 24 via nvm: nvm install 24"

# ── Vulkan SDK ────────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK_ENV="$SCRIPT_DIR/.vulkanSDK/setup-env.sh"

if [[ -f "$SDK_ENV" ]]; then
  source "$SDK_ENV"
  ok "Vulkan SDK sourced → VULKAN_SDK=$VULKAN_SDK"
elif [[ -z "${VULKAN_SDK:-}" ]]; then
  die "Could not find .vulkanSDK/setup-env.sh and VULKAN_SDK is not set."
fi

export LD_LIBRARY_PATH="${VULKAN_SDK}/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

# ── Directories ───────────────────────────────────────────────────────────────
DEMO_DIR="$SCRIPT_DIR/web-client/demo"
[[ -d "$DEMO_DIR" ]] || die "Demo directory not found: $DEMO_DIR"
[[ -d "$DEMO_DIR/node_modules" ]] || { log "Installing demo npm deps…"; npm install --prefix "$DEMO_DIR"; }

# ── Rust binary path ──────────────────────────────────────────────────────────
if [[ $RELEASE -eq 1 ]]; then
  CARGO_FLAGS="--release"
  BINARY="$SCRIPT_DIR/target/release/veil"
else
  CARGO_FLAGS=""
  BINARY="$SCRIPT_DIR/target/debug/veil"
fi

HTTP_PORT=$((QUIC_PORT + 1))

# ── PID tracking + cleanup ────────────────────────────────────────────────────
PIDS=()

cleanup() {
  echo ""
  log "Shutting down…"
  for pid in "${PIDS[@]}"; do
    kill "$pid" 2>/dev/null || true
  done
  # Kill any Steam processes launched by this session so the daemon doesn't
  # survive and leave stale pid files for the next run.
  pkill -f "[Ss]team\.sh\|[Ss]team$\|steam_linux\|steamwebhelper" 2>/dev/null || true
  wait 2>/dev/null || true
  ok "Done."
}
trap cleanup EXIT INT TERM

# ── Step 1: Build Rust ────────────────────────────────────────────────────────
if [[ $NO_BUILD -eq 0 ]]; then
  log "Building Rust backend ($([[ $RELEASE -eq 1 ]] && echo release || echo debug))…"
  cargo build $CARGO_FLAGS 2>&1 | grep -v "^   Compiling\|^   Checking\|^    Finished\|^     Locking" || true
  cargo build $CARGO_FLAGS
  ok "Rust build complete → $BINARY"
else
  [[ -x "$BINARY" ]] || die "--no-build specified but binary not found: $BINARY"
  ok "Skipping build, using existing binary."
fi

# ── Step 2: esbuild --watch ───────────────────────────────────────────────────
log "Starting esbuild watcher…"
(cd "$DEMO_DIR" && npm run dev) &
PIDS+=($!)
sleep 1  # give esbuild a moment to emit app.js

# ── Step 3: HTTP demo server ──────────────────────────────────────────────────
log "Starting HTTP demo server on port $WEB_PORT…"
(cd "$DEMO_DIR" && npx http-server . -p "$WEB_PORT" --cors -c-1 -s) &
PIDS+=($!)

# ── Step 4: Shut down any existing Steam instance ────────────────────────────
# Steam is single-instance: if it's already running it will exit immediately
# when launched again (status 0), which looks like a crash to the reactor.
# We check both live processes AND stale pid files (which Steam reads on startup).
STEAM_PID_FILE="${HOME}/.steam/steam.pid"
STEAM_LOCK_FILE="${HOME}/.local/share/Steam/steam.lock"

_stale_pid() {
  local f=$1
  [[ -f "$f" ]] || return 1
  local pid; pid=$(<"$f") 2>/dev/null
  [[ -n "$pid" ]] && ! kill -0 "$pid" 2>/dev/null
}

if pgrep -f "[Ss]team\.sh\|[Ss]team$\|steam_linux\|steamwebhelper" &>/dev/null || _stale_pid "$STEAM_PID_FILE" || _stale_pid "$STEAM_LOCK_FILE"; then
  if pgrep -f "[Ss]team\.sh\|[Ss]team$\|steam_linux\|steamwebhelper" &>/dev/null; then
    warn "Steam is already running — shutting it down first…"
    steam -shutdown 2>/dev/null || true
    # Give it up to 10 s to exit cleanly, then force-kill.
    for i in $(seq 1 20); do
      pgrep -f "[Ss]team\.sh\|[Ss]team$\|steam_linux\|steamwebhelper" &>/dev/null || break
      sleep 0.5
    done
    if pgrep -f "[Ss]team\.sh\|[Ss]team$\|steam_linux\|steamwebhelper" &>/dev/null; then
      warn "Steam did not exit cleanly — killing it."
      pkill -9 -f "[Ss]team\.sh\|[Ss]team$\|steam_linux\|steamwebhelper" 2>/dev/null || true
      sleep 1
    fi
    ok "Steam stopped."
  fi
  # Remove stale pid/lock files so Steam doesn't think it's already running.
  if [[ -f "$STEAM_PID_FILE" ]]; then
    warn "Removing stale Steam pid file: $STEAM_PID_FILE"
    rm -f "$STEAM_PID_FILE"
  fi
  if [[ -f "$STEAM_LOCK_FILE" ]]; then
    warn "Removing stale Steam lock file: $STEAM_LOCK_FILE"
    rm -f "$STEAM_LOCK_FILE"
  fi
fi

# ── Step 5: Rust backend ──────────────────────────────────────────────────────
log "Starting Veil backend on QUIC :${QUIC_PORT} / HTTP :${HTTP_PORT}…"
RUST_LOG="${RUST_LOG:-info,veil::session::reactor=debug}" \
  RUST_BACKTRACE="${RUST_BACKTRACE:-1}" \
  "$BINARY" \
    --transport \
    --transport-addr "0.0.0.0:${QUIC_PORT}" \
    --size "$SIZE" \
    --fps "$FPS" \
    --codec "$CODEC" \
    -- "${APP_CMD[@]}" &
PIDS+=($!)

# ── Ready ─────────────────────────────────────────────────────────────────────
HOST_IP=$(hostname -I 2>/dev/null | awk '{print $1}' || echo "localhost")
echo ""
echo -e "${BOLD}${GREEN}┌─ Veil is running ──────────────────────────────────────┐${NC}"
echo -e "${BOLD}${GREEN}│${NC}  Demo UI   →  ${BOLD}http://${HOST_IP}:${WEB_PORT}${NC}"
echo -e "${BOLD}${GREEN}│${NC}  (open in Chrome — Safari/Firefox lack WebTransport)  ${BOLD}${GREEN}│${NC}"
echo -e "${BOLD}${GREEN}│${NC}  QUIC      →  https://${HOST_IP}:${QUIC_PORT}           ${BOLD}${GREEN}│${NC}"
echo -e "${BOLD}${GREEN}│${NC}  Cert hash →  http://${HOST_IP}:${HTTP_PORT}/cert-hash  ${BOLD}${GREEN}│${NC}"
echo -e "${BOLD}${GREEN}│${NC}                                                        ${BOLD}${GREEN}│${NC}"
echo -e "${BOLD}${GREEN}│${NC}  Press Ctrl+C to stop all services                    ${BOLD}${GREEN}│${NC}"
echo -e "${BOLD}${GREEN}└────────────────────────────────────────────────────────┘${NC}"
echo ""

# ── Wait for the Rust backend (it drives the session lifetime) ───────────────
RUST_PID=${PIDS[-1]}
wait "$RUST_PID" 2>/dev/null || true
