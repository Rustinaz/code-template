#!/usr/bin/env bash
# Builds the Android `.so` files, and optionally the APK around them.
#
#   scripts/build-android.sh                 # all four ABIs, debug
#   scripts/build-android.sh --release       # all four ABIs, release
#   scripts/build-android.sh --abi arm64-v8a # a single ABI
#   scripts/build-android.sh --apk           # also build the APK via Gradle
#
# Rust is cross-compiled by Cargo with the NDK's clang; the APK is assembled by
# the Gradle project in `android-app/`, which runs the same Cargo command itself.
# Both paths need an NDK, discovered by `scripts/android-env.sh`.

set -euo pipefail

cd "$(dirname "$0")/.."

ABIS="arm64-v8a,armeabi-v7a,x86_64,x86"
PROFILE="debug"
MAKE_APK="no"

while [ $# -gt 0 ]; do
  case "$1" in
    --release) PROFILE="release"; shift ;;
    --debug)   PROFILE="debug";   shift ;;
    --abi)     ABIS="$2";         shift 2 ;;
    --apk)     MAKE_APK="yes";    shift ;;
    -h|--help) sed -n '2,11p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
done

abi_to_triple() {
  case "$1" in
    arm64-v8a)   echo aarch64-linux-android ;;
    armeabi-v7a) echo armv7-linux-androideabi ;;
    x86_64)      echo x86_64-linux-android ;;
    x86)         echo i686-linux-android ;;
    *) echo "unknown ABI: $1" >&2; exit 2 ;;
  esac
}

# shellcheck source=scripts/android-env.sh
source scripts/android-env.sh

# Cargo puts a cross-compiled artifact in target/<triple>/<profile>/ -- the
# triple comes *before* the profile, unlike a host build where there is no
# triple at all.
case "$PROFILE" in
  debug)   CARGO_PROFILE_FLAG=() ; OUT_SUBDIR=debug ;;
  release) CARGO_PROFILE_FLAG=(--release); OUT_SUBDIR=release ;;
esac

echo "Building for Android ABIs: $ABIS ($PROFILE)"

IFS=',' read -r -a abi_list <<< "$ABIS"
triples=()
for abi in "${abi_list[@]}"; do
  triples+=("$(abi_to_triple "$abi")")
done

# The Rust std libraries for these targets have to be installed once per machine.
rustup target add "${triples[@]}"

for i in "${!abi_list[@]}"; do
  abi="${abi_list[$i]}"
  triple="${triples[$i]}"
  echo "  -> $abi ($triple)"
  cargo build --package example-app --target "$triple" "${CARGO_PROFILE_FLAG[@]}"

  so="target/$triple/$OUT_SUBDIR/libexample_app.so"
  if [ ! -f "$so" ]; then
    echo "error: expected $so to exist after a successful build" >&2
    exit 1
  fi
  printf '     %s\n' "$so ($(du -h "$so" | cut -f1))"
done

echo
echo "Android shared libraries built."

if [ "$MAKE_APK" = "yes" ]; then
  echo
  echo "Assembling the APK with Gradle..."
  (
    cd android-app
    ./gradlew assembleDebug -PandroidAbi="$ABIS"
  )
  echo
  echo "APK: android-app/app/build/outputs/apk/debug/app-debug.apk"
fi
