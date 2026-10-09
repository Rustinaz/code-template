#!/usr/bin/env bash
# Runs the bundled backend and the desktop client together.
#
#   scripts/dev.sh                          # backend + client
#   scripts/dev.sh --release
#   scripts/dev.sh --bind 0.0.0.0:8080      # also accept LAN / emulator clients
#   scripts/dev.sh --server-only            # just the backend
#   scripts/dev.sh --client-only --server http://127.0.0.1:8080
#
# Android: run `adb reverse tcp:8080 tcp:8080` first. The app's default backend
# URL is http://127.0.0.1:8080, which then reaches this machine.

set -euo pipefail

cd "$(dirname "$0")/.."

PROFILE="debug"
BIND="127.0.0.1:8080"
SERVER_URL=""
MODE="both"

usage() { sed -n '2,11p' "$0" | sed 's/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
  case "$1" in
    --release)     PROFILE="release"; shift ;;
    --bind)        BIND="${2:?--bind needs an address}"; shift 2 ;;
    --server)      SERVER_URL="${2:?--server needs a URL}"; shift 2 ;;
    --server-only) MODE="server"; shift ;;
    --client-only) MODE="client"; shift ;;
    -h|--help)     usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

FLAGS=()
[ "$PROFILE" = "release" ] && FLAGS=(--release)

BIN_DIR="target/$PROFILE"
SERVER_URL="${SERVER_URL:-http://$BIND}"

if [ "$MODE" = "server" ]; then
  cargo build --package server "${FLAGS[@]}"
  echo "Starting backend on $BIND (Ctrl-C to stop)"
  exec "$BIN_DIR/server" --bind "$BIND"
fi

echo "Building ($PROFILE)…"
cargo build --package example-app "${FLAGS[@]}"
[ "$MODE" = "both" ] && cargo build --package server "${FLAGS[@]}"

cleanup() {
  if [ -n "${SERVER_PID:-}" ]; then
    kill "$SERVER_PID" 2>/dev/null || true
    SERVER_PID=""
  fi
}

SERVER_PID=""
if [ "$MODE" = "both" ]; then
  "$BIN_DIR/server" --bind "$BIND" &
  SERVER_PID=$!
  # Stop the backend however this script exits, Ctrl-C included. A bare `trap`
  # on TERM/INT would run the handler and then keep going, so those exit.
  trap cleanup EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM

  HOST="${BIND%:*}"
  PORT="${BIND##*:}"
  for _ in $(seq 1 100); do
    if (exec 3<>"/dev/tcp/$HOST/$PORT") 2>/dev/null; then
      exec 3>&- 3<&-
      break
    fi
    sleep 0.1
  done
fi

echo "Starting client against $SERVER_URL"
"$BIN_DIR/example-app" --server "$SERVER_URL"
