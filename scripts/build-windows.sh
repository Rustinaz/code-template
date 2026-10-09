#!/usr/bin/env bash
# Builds the Windows desktop binary by cross-compiling from Linux or macOS.
#
#   scripts/build-windows.sh            # debug
#   scripts/build-windows.sh --release
#
# Needs a MinGW-w64 cross toolchain and the Rust target:
#
#   # Debian/Ubuntu
#   sudo apt-get install gcc-mingw-w64-x86-64
#   rustup target add x86_64-pc-windows-gnu
#
# On a real Windows host, install the MSVC toolchain and the `*-pc-windows-msvc`
# target instead, and this script will use the native compiler.

set -euo pipefail

cd "$(dirname "$0")/.."

PROFILE="debug"
case "${1:-}" in
  --release) PROFILE="release" ;;
  "")        PROFILE="debug" ;;
  *) echo "usage: $0 [--release]" >&2; exit 2 ;;
esac

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    TARGET="x86_64-pc-windows-msvc"
    HOST_NATIVE="yes"
    ;;
  *)
    TARGET="x86_64-pc-windows-gnu"
    HOST_NATIVE="no"
    ;;
esac

FLAGS=()
[ "$PROFILE" = "release" ] && FLAGS=(--release)

if [ "$HOST_NATIVE" = "no" ]; then
  if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
    cat >&2 <<'EOF'
error: x86_64-w64-mingw32-gcc not found.

Install a MinGW-w64 cross toolchain, for example:

    sudo apt-get install gcc-mingw-w64-x86-64     # Debian / Ubuntu
    sudo dnf install mingw64-gcc                 # Fedora

Or build on Windows itself, where the MSVC toolchain is used instead.
EOF
    exit 1
  fi
  rustup target add "$TARGET"
fi

echo "Building example-app for Windows ($TARGET)"
cargo build --package example-app --target "$TARGET" "${FLAGS[@]}"

BIN="target/$TARGET/$PROFILE/example-app.exe"
echo
echo "Output: $BIN"
