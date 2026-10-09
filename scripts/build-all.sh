#!/usr/bin/env bash
# Builds every target the current machine can actually build.
#
#   scripts/build-all.sh              # everything available here
#   scripts/build-all.sh android web  # just these
#   scripts/build-all.sh --list       # show what this machine supports
#
# Targets that need a foreign toolchain (Windows needs MinGW or Windows, iOS
# and macOS need macOS) are reported as skipped rather than failing the run, so
# this is useful on every host.

set -euo pipefail

cd "$(dirname "$0")/.."

ALL_TARGETS=(linux android windows macos ios web)
REQUESTED=()

if [ $# -eq 0 ]; then
  REQUESTED=("${ALL_TARGETS[@]}")
else
  for arg in "$@"; do
    case "$arg" in
      --list) MODE=list ;;
      all)    REQUESTED=("${ALL_TARGETS[@]}") ;;
      -h|--help) sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
      *)
        if printf '%s\n' "${ALL_TARGETS[@]}" | grep -qx "$arg"; then
          REQUESTED+=("$arg")
        else
          echo "unknown target: $arg" >&2
          echo "usage: $0 [${ALL_TARGETS[*]}] [--list]" >&2
          exit 2
        fi
        ;;
    esac
  done
fi

host_is() { [ "$(uname -s)" = "$1" ]; }

declare -A SUPPORTED=(
  [linux]="yes"
  [android]="yes"
  [windows]="$(command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1 && echo yes || host_is MINGW* || echo no)"
  [macos]="$(host_is Darwin && echo yes || echo no)"
  [ios]="$(host_is Darwin && echo yes || echo no)"
  [web]="$(command -v trunk >/dev/null 2>&1 && echo yes || echo no)"
)

if [ "${MODE:-}" = "list" ]; then
  printf '%-9s %-9s %s\n' TARGET HERE REASON
  for t in "${ALL_TARGETS[@]}"; do
    case "$t" in
      windows) [ "${SUPPORTED[windows]}" = "yes" ] && why="MinGW-w64 present" || why="needs MinGW-w64 or a Windows host" ;;
      macos|ios) [ "${SUPPORTED[$t]}" = "yes" ] && why="macOS host" || why="needs a macOS host with Xcode" ;;
      android)   why="Android SDK + NDK" ;;
      web)       [ "${SUPPORTED[web]}" = "yes" ] && why="trunk present" || why="needs 'cargo install --locked trunk'" ;;
      *)         why="native toolchain" ;;
    esac
    printf '%-9s %-9s %s\n' "$t" "${SUPPORTED[$t]}" "$why"
  done
  exit 0
fi

declare -a BUILT=() SKIPPED=() FAILED=()

for target in "${REQUESTED[@]}"; do
  if [ "${SUPPORTED[$target]:-no}" != "yes" ]; then
    echo "== $target: skipped (not supported on this host)"
    SKIPPED+=("$target")
    continue
  fi

  echo
  echo "== $target"
  if scripts/build-$target.sh --release; then
    BUILT+=("$target")
  else
    echo "== $target: FAILED" >&2
    FAILED+=("$target")
  fi
done

echo
echo "built:   ${BUILT[*]:-none}"
echo "skipped: ${SKIPPED[*]:-none}"
if [ ${#FAILED[@]} -gt 0 ]; then
  echo "failed:  ${FAILED[*]}" >&2
  exit 1
fi
