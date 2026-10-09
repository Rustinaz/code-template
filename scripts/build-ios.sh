#!/usr/bin/env bash
# Builds the iOS static library that an Xcode app links against.
#
#   scripts/build-ios.sh                  # device (arm64)
#   scripts/build-ios.sh --simulator      # x86_64 simulator
#
# Must run on macOS with Xcode installed. The output is a bare `.a`; Xcode
# builds are driven from `ios-app/` (or your own Xcode project) with the library
# added to "Link Binary With Libraries".
#
# Note on toolchain versions: this template uses eframe/winit, which need a
# reasonably recent Rust std for the iOS target and an iOS SDK no older than
# 12. Xcode 10.1 (the last version installable on macOS High Sierra) ships the
# iOS 12 SDK, which is the floor and works. Newer Xcode versions are fine too.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ "$(uname -s)" != "Darwin" ]; then
  echo "error: iOS artifacts can only be built on macOS (found $(uname -s))." >&2
  echo "There is no supported way to link an iOS binary from Linux or Windows." >&2
  exit 1
fi

if ! command -v xcrun >/dev/null 2>&1; then
  echo "error: xcrun not found. Install the Xcode command line tools:" >&2
  echo "         xcode-select --install" >&2
  exit 1
fi

PROFILE="release"
TARGET="aarch64-apple-ios"

case "${1:-}" in
  --release)   PROFILE="release"; TARGET="aarch64-apple-ios" ;;
  --debug)     PROFILE="debug";   TARGET="aarch64-apple-ios" ;;
  --simulator) PROFILE="release"; TARGET="x86_64-apple-ios" ;;
  "")          ;;
  *) echo "usage: $0 [--release|--debug|--simulator]" >&2; exit 2 ;;
esac

FLAGS=()
[ "$PROFILE" = "release" ] && FLAGS=(--release)

rustup target add "$TARGET"

SDK_VERSION="$(xcrun --sdk iphoneos --show-sdk-version 2>/dev/null || true)"
if [ -n "$SDK_VERSION" ]; then
  echo "iOS SDK: $SDK_VERSION"
fi

echo "Building example-app for iOS ($TARGET)"
cargo build --package example-app --target "$TARGET" "${FLAGS[@]}"

LIB="target/$TARGET/$PROFILE/libexample_app.a"
if [ ! -f "$LIB" ]; then
  echo "error: expected $LIB to exist after a successful build" >&2
  echo "Check that [lib] crate-type in apps/example/Cargo.toml is staticlib." >&2
  exit 1
fi

echo
echo "Output: $LIB ($(du -h "$LIB" | cut -f1))"
echo
echo "Next: add the library to an Xcode target under"
echo "  Target > Build Phases > Link Binary With Libraries,"
echo "and set the deployment target to iOS 12.0 or newer."
