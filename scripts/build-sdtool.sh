#!/usr/bin/env bash
# Builds compas-sdtool from the player's feature/compas-sdtool worktree and
# puts it where Tauri's externalBin expects it (src-tauri/binaries/).
#
#   scripts/build-sdtool.sh                       Linux, inside the build container
#   SDTOOL_TARGET=windows scripts/build-sdtool.sh Windows .exe (MinGW, in the container)
#   SDTOOL_SRC=/path/to/worktree scripts/build-sdtool.sh
#
# On macOS it builds natively; the Windows and macOS binaries are untested here.
set -euo pipefail

cd "$(dirname "$0")/.."
SRC="${SDTOOL_SRC:-$HOME/.cache/compas-tools/player-sdtool}"
TARGET="${SDTOOL_TARGET:-native}"
OUT=src-tauri/binaries

[ -f "$SRC/src/tools/sdtool/sdtool.mk" ] || {
  echo "No compas-sdtool sources in $SRC (set SDTOOL_SRC to the feature/compas-sdtool worktree)" >&2
  exit 1
}
mkdir -p "$OUT"

# The id the tool must report as `build`: the worktree's HEAD, marked when it has changes.
# Computed here because the container cannot read the worktree's git metadata.
BUILD_ID=$(git -C "$SRC" rev-parse --short HEAD)
git -C "$SRC" diff --quiet HEAD || BUILD_ID="$BUILD_ID-dirty"

# Fails unless the binary's `info` reports the expected build.
verify() {
  local reported
  reported=$("$@" | sed -n 's/.*"build":"\([^"]*\)".*/\1/p')
  [ "$reported" = "$BUILD_ID" ] || {
    echo "compas-sdtool reports build '$reported', expected '$BUILD_ID'" >&2
    exit 1
  }
}

if [ "$(uname -s)" = Darwin ] && [ "$TARGET" = native ]; then
  triple=$(rustc -vV | sed -n 's/^host: //p')
  make -C "$SRC" -j"$(sysctl -n hw.ncpu)" sdtool SDTOOL_BUILD_ID="$BUILD_ID"
  cp "$SRC/build_sdtool/compas-sdtool" "$OUT/compas-sdtool-$triple"
  card=$(mktemp -d); mkdir "$card/.compas"
  verify "$OUT/compas-sdtool-$triple" info "$card"
  exit 0
fi

if [ "$TARGET" = windows ]; then
  triple=x86_64-pc-windows-msvc
  build_dir=/cache/sdtool-windows
  binary=compas-sdtool.exe
  make_args=(SDTOOL_TARGET=windows SDTOOL_CMAKE_EXTRA=-DCMAKE_CXX_COMPILER=x86_64-w64-mingw32-g++)
  suffix=.exe
else
  triple=x86_64-unknown-linux-gnu
  build_dir=/cache/sdtool-linux
  binary=compas-sdtool
  make_args=()
  suffix=
fi


# Objects go to the cache volume, so the worktree is only read.
DEV_PODMAN_ARGS="--security-opt label=disable -v $SRC:/sdtool-src:ro" scripts/dev.sh bash -c "
  make -C /sdtool-src -j\$(nproc) sdtool SDTOOL_DIR=$build_dir SDTOOL_BUILD_ID=$BUILD_ID ${make_args[*]:-} &&
  cp $build_dir/$binary /work/$OUT/compas-sdtool-$triple$suffix"

if [ "$TARGET" != windows ]; then
  card=$(mktemp -d); mkdir "$card/.compas"
  DEV_PODMAN_ARGS="-v $card:/card:z" scripts/dev.sh bash -c \
    "/work/$OUT/compas-sdtool-$triple info /card" > "$card/info.json"
  verify cat "$card/info.json"
  rm -rf "$card"
fi
echo "Built $OUT/compas-sdtool-$triple$suffix (build $BUILD_ID)"
