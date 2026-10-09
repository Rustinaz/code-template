#!/usr/bin/env bash
# Builds the web (WASM) version and serves it locally.
#
#   scripts/build-web.sh              # build into apps/example/dist
#   scripts/build-web.sh --serve      # build, then serve on http://localhost:8080
#   scripts/build-web.sh --release    # optimised build
#
# Uses Trunk, which is the supported toolchain for an eframe app: it runs
# `cargo build --target wasm32-unknown-unknown` and then `wasm-bindgen` on the
# result, wiring the glue into `index.html`. `wasm-pack` cannot do this because
# eframe's WebGL backend is not a plain `wasm-bindgen` export.
#
# Install once with:
#
#     cargo install --locked trunk

set -euo pipefail

cd "$(dirname "$0")/.."

MODE="build"
PROFILE="--release"

for arg in "$@"; do
  case "$arg" in
    --serve)   MODE="serve" ;;
    --release) PROFILE="--release" ;;
    --debug)   PROFILE="" ;;
    -h|--help) sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown option: $arg" >&2; exit 2 ;;
  esac
done

if ! command -v trunk >/dev/null 2>&1; then
  cat >&2 <<'EOF'
error: trunk is not installed.

    cargo install --locked trunk

Then re-run this script.
EOF
  exit 1
fi

rustup target add wasm32-unknown-unknown

# `--config` points Trunk at the Trunk.toml next to index.html. Trunk resolves
# relative paths in that file against its own directory, so `dist` refers to
# apps/example/dist and not the workspace's.
#
# index.html is passed positionally rather than with `--package`: Trunk has no
# `--package` flag. Note the *relative* path -- Trunk resolves the target against
# the directory of the config file, so `apps/example/index.html` here would be
# joined into `apps/example/apps/example/index.html` and fail.
TRUNK_ARGS=(--config apps/example/Trunk.toml index.html)
[ -n "$PROFILE" ] && TRUNK_ARGS+=("$PROFILE")

case "$MODE" in
  build)
    echo "Building for the web (WASM)"
    trunk build "${TRUNK_ARGS[@]}"
    echo
    echo "Output: apps/example/dist"
    echo "Serve it with any static file server, e.g.:"
    echo "  python3 -m http.server -d apps/example/dist 8080"
    ;;
  serve)
    echo "Building and serving on http://localhost:8080 (Ctrl-C to stop)"
    trunk serve "${TRUNK_ARGS[@]}"
    ;;
esac
