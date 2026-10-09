#!/usr/bin/env bash
# Exports everything Cargo needs to cross-compile for Android.
#
# Source this, do not run it:
#
#     source scripts/android-env.sh
#     cargo build --package example-app --target aarch64-linux-android
#
# NDK r23 and newer removed the bare `<triple>-clang` symlinks, so the C
# toolchain has to be named explicitly for cc-rs. The Gradle `cargoBuild` task
# in android-app/app/build.gradle.kts sets exactly the same variables, so you
# only need this script when driving Cargo by hand.
set -euo pipefail

: "${ANDROID_HOME:=${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"

if [ -z "${ANDROID_NDK_HOME:-}" ]; then
  # Highest version wins, so an explicit ANDROID_NDK_HOME always takes priority.
  ANDROID_NDK_HOME="$(find "$ANDROID_HOME/ndk" -mindepth 1 -maxdepth 1 -type d 2>/dev/null |
    sort -V | tail -1)"
fi

if [ -z "${ANDROID_NDK_HOME:-}" ] || [ ! -d "$ANDROID_NDK_HOME" ]; then
  echo "android-env: no NDK found. Set ANDROID_NDK_HOME to an NDK directory." >&2
  return 1 2>/dev/null || exit 1
fi

NDK_BIN="$(ls -d "$ANDROID_NDK_HOME"/toolchains/llvm/prebuilt/*/bin 2>/dev/null | head -1)"
if [ -z "$NDK_BIN" ]; then
  echo "android-env: $ANDROID_NDK_HOME has no llvm/prebuilt toolchain." >&2
  return 1 2>/dev/null || exit 1
fi

# API level to compile against. minSdk 21 in android-app/app/build.gradle.kts.
ANDROID_API_LEVEL="${ANDROID_API_LEVEL:-21}"

export ANDROID_HOME ANDROID_NDK_HOME
export PATH="$NDK_BIN:$PATH"

# Rust target triple -> NDK arch prefix.
android_toolchain() {
  case "$1" in
    aarch64-linux-android)  echo aarch64-linux-android ;;
    armv7-linux-androideabi) echo armv7a-linux-androideabi ;;
    i686-linux-android)      echo i686-linux-android ;;
    x86_64-linux-android)    echo x86_64-linux-android ;;
    *) return 1 ;;
  esac
}

for triple in \
  aarch64-linux-android \
  armv7-linux-androideabi \
  i686-linux-android \
  x86_64-linux-android
do
  prefix="$(android_toolchain "$triple")" || continue
  upper="$(echo "$triple" | tr 'a-z-' 'A-Z_')"
  clang="$NDK_BIN/${prefix}${ANDROID_API_LEVEL}-clang"
  if [ ! -x "$clang" ]; then
    echo "android-env: $clang is missing" >&2
    continue
  fi
  export "CARGO_TARGET_${upper}_LINKER=$clang"
  export "CC_${triple//-/_}=$clang"
  export "CXX_${triple//-/_}=$NDK_BIN/${prefix}${ANDROID_API_LEVEL}-clang++"
  export "AR_${triple//-/_}=$NDK_BIN/llvm-ar"
  export "RANLIB_${triple//-/_}=$NDK_BIN/llvm-ranlib"
done

if [ -n "${VERBOSE:-}" ]; then
  echo "ANDROID_NDK_HOME=$ANDROID_NDK_HOME"
  echo "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"
fi
