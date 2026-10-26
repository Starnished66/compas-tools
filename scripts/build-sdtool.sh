#!/usr/bin/env bash
# Builds compas-sdtool (sdtool/ in this repository) against a compas-player checkout
# (default: ../compas-player) and puts it where Tauri's externalBin expects it
# (src-tauri/binaries/). Objects go to build/ here; the player's sources are not changed,
# but its Makefile fetches its vendored libraries into the checkout when they are missing
# (use a `git worktree` of the player to keep your own checkout untouched).
#
#   scripts/build-sdtool.sh                       Linux, inside the build container
#   SDTOOL_TARGET=windows scripts/build-sdtool.sh Windows .exe (MinGW, in the container)
#   SDTOOL_SRC=/path/to/compas-player scripts/build-sdtool.sh
#   SDTOOL_TEST=1 scripts/build-sdtool.sh         also run the end-to-end test (Linux)
#
# On macOS it builds natively; the Windows and macOS binaries are untested here.
set -euo pipefail

cd "$(dirname "$0")/.."
SRC=$(cd "${SDTOOL_SRC:-$PWD/../compas-player}" && pwd)
TARGET="${SDTOOL_TARGET:-native}"
JOBS="${SDTOOL_JOBS:-4}"
OUT=src-tauri/binaries
MK=sdtool/sdtool.mk

[ -f "$SRC/Makefile" ] && [ -f "$SRC/src/library/tagcache/core/tagcache.h" ] || {
  echo "$SRC is not a compas-player checkout (set SDTOOL_SRC)" >&2
  exit 1
}
mkdir -p "$OUT"

# The build id `info` reports: both revisions, marked when they have changes. Computed here
# because the container cannot read a worktree's git metadata.
rev() {
  local id
  id=$(git -C "$1" rev-parse --short HEAD)
  git -C "$1" diff --quiet HEAD -- "${2:-.}" || id="$id-dirty"
  echo "$id"
}
BUILD_ID="tools-$(rev . sdtool)+player-$(rev "$SRC")"

if [ "$(uname -s)" = Darwin ] && [ "$TARGET" = native ]; then
  triple=$(rustc -vV | sed -n 's/^host: //p')
  make -C "$SRC" -j"$(sysctl -n hw.ncpu)" -f Makefile -f "$PWD/$MK" sdtool SDTOOL_BUILD_ID="$BUILD_ID"
  cp build/sdtool/compas-sdtool "$OUT/compas-sdtool-$triple"
  SDTOOL_EXPECT_BUILD="$BUILD_ID" scripts/verify-sdtool.sh "$SRC" "$OUT/compas-sdtool-$triple"
  exit 0
fi

if [ "$TARGET" = windows ]; then
  triple=x86_64-pc-windows-msvc
  built=build/sdtool-windows/compas-sdtool.exe
  make_args="SDTOOL_TARGET=windows SDTOOL_CMAKE_EXTRA=-DCMAKE_CXX_COMPILER=x86_64-w64-mingw32-g++"
  suffix=.exe
else
  triple=x86_64-unknown-linux-gnu
  built=build/sdtool/compas-sdtool
  make_args=""
  suffix=
fi

goals=sdtool
[ -n "${SDTOOL_TEST:-}" ] && [ "$TARGET" != windows ] && goals="sdtool test-sdtool"
DEV_PODMAN_ARGS="--security-opt label=disable -v $SRC:/player" scripts/dev.sh bash -c "
  make -C /player -j$JOBS -f Makefile -f /work/$MK $goals SDTOOL_BUILD_ID=$BUILD_ID $make_args"
cp "$built" "$OUT/compas-sdtool-$triple$suffix"

# The Windows binary cannot run here.
if [ "$TARGET" != windows ]; then
  SDTOOL_EXPECT_BUILD="$BUILD_ID" scripts/verify-sdtool.sh "$SRC" "$OUT/compas-sdtool-$triple"
fi
echo "Built $OUT/compas-sdtool-$triple$suffix (build $BUILD_ID)"
