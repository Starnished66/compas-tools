#!/usr/bin/env bash
# Builds the Linux AppImage in the container and copies it to artifacts/.
set -euo pipefail

cd "$(dirname "$0")/.."
npm ci
# A finished bundle folder cannot be overwritten by the next run (linuxdeploy
# leaves unreadable folders in it), so move it aside instead of deleting it.
scripts/dev.sh bash -c '
  bundle=/cache/target/release/bundle
  [ ! -e "$bundle" ] || mv "$bundle" "/cache/old-bundle-$(date +%s)"
  npx tauri build --bundles appimage &&
  mkdir -p artifacts &&
  cp /cache/target/release/bundle/appimage/*.AppImage artifacts/'
