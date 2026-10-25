#!/usr/bin/env bash
# Builds the Linux AppImage in the container and copies it to artifacts/.
set -euo pipefail

cd "$(dirname "$0")/.."
npm ci
[ -e src-tauri/binaries/compas-sdtool-x86_64-unknown-linux-gnu ] || scripts/build-sdtool.sh
scripts/dev.sh bash -c '
  rm -rf /cache/target/release/bundle &&
  npx tauri build --bundles appimage &&
  mkdir -p artifacts &&
  cp /cache/target/release/bundle/appimage/*.AppImage artifacts/'
