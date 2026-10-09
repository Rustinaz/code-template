#!/usr/bin/env bash
# Builds the macOS desktop app, for both Apple silicon and Intel.
#
#   scripts/build-macos.sh            # debug
#   scripts/build-macos.sh --release
#
# Must run on macOS with the Xcode command line tools installed
# (`xcode-select --install`). The `.app` bundle is assembled by hand because the
# only thing in it is one binary and one Info.plist.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ "$(uname -s)" != "Darwin" ]; then
  echo "error: macOS apps can only be built on macOS (found $(uname -s))." >&2
  echo "Cross-compiling a macOS .app from Linux needs osxcross and is not supported here." >&2
  exit 1
fi

PROFILE="debug"
case "${1:-}" in
  --release) PROFILE="release" ;;
  "")        PROFILE="debug" ;;
  *) echo "usage: $0 [--release]" >&2; exit 2 ;;
esac

FLAGS=()
[ "$PROFILE" = "release" ] && FLAGS=(--release)

TARGETS="aarch64-apple-darwin x86_64-apple-darwin"
rustup target add $TARGETS

for target in $TARGETS; do
  echo "Building example-app for $target"
  cargo build --package example-app --target "$target" "${FLAGS[@]}"

  app="target/$target/$PROFILE/example-app.app"
  rm -rf "$app"
  mkdir -p "$app/Contents/MacOS"
  cp "target/$target/$PROFILE/example-app" "$app/Contents/MacOS/example-app"

  cat > "$app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>          <string>example-app</string>
  <key>CFBundleIdentifier</key>          <string>dev.rustcrossplatform.example</string>
  <key>CFBundleName</key>                <string>Example App</string>
  <key>CFBundlePackageType</key>         <string>APPL</string>
  <key>CFBundleShortVersionString</key>  <string>0.1.0</string>
  <key>CFBundleVersion</key>             <string>1</string>
  <key>LSMinimumSystemVersion</key>      <string>11.0</string>
  <key>NSHighResolutionCapable</key>     <true/>
</dict>
</plist>
PLIST

  echo "  -> $app"
done

echo
echo "macOS bundles built. Universal (both architectures in one bundle):"
echo "  lipo -create -output App target/aarch64-apple-darwin/$PROFILE/example-app.app/Contents/MacOS/example-app \\"
echo "                     target/x86_64-apple-darwin/$PROFILE/example-app.app/Contents/MacOS/example-app"
