#!/usr/bin/env bash
# Builds the native Linux desktop binary.
#
#   scripts/build-linux.sh            # debug
#   scripts/build-linux.sh --release
#
# Output: target/<profile>/example-app

set -euo pipefail

cd "$(dirname "$0")/.."

PROFILE="debug"
case "${1:-}" in
  --release) PROFILE="release" ;;
  "")        PROFILE="debug" ;;
  *) echo "usage: $0 [--release]" >&2; exit 2 ;;
esac

FLAGS=()
RUN_FLAGS=""
[ "$PROFILE" = "release" ] && { FLAGS=(--release); RUN_FLAGS="--release"; }

echo "Building example-app for Linux${RUN_FLAGS:+ (release)}"
cargo build --package example-app "${FLAGS[@]}"

echo
echo "Run it with:  cargo run --package example-app $RUN_FLAGS"
