#!/usr/bin/env bash
# Takes a screenshot of every screen of the built app, in the light and the dark scheme.
# The app runs on a virtual display inside the build container, against a throwaway
# card folder. Build it first: npm run build && scripts/dev.sh npx tauri build --debug --no-bundle
#
#   scripts/screenshots.sh [output-dir]      default: docs/screenshots
set -euo pipefail

cd "$(dirname "$0")/.."
OUT="$(realpath -m "${1:-docs/screenshots}")"
CARD="$HOME/.cache/compas-tools/screenshot-card"
MUSIC="$HOME/.cache/compas-tools/screenshot-music"

rm -rf "$CARD" "$MUSIC"
mkdir -p "$CARD/.compas" "$CARD/.plugins" "$CARD/Artist/Album" "$MUSIC/Album" "$OUT"
echo '{"schema":1,"model":"r1","player_version":"1.1.0","plugin_api":17,"database_format":5,"art_cache":"v4"}' \
  > "$CARD/.compas/device.json"
echo "-- a plugin the store did not install" > "$CARD/.plugins/ABRepeat.lua"
head -c 300000 /dev/urandom > "$MUSIC/Album/01 Opening.flac"
head -c 200000 /dev/urandom > "$MUSIC/Album/02 AC:DC?.flac"
head -c 100000 /dev/urandom > "$MUSIC/Album/._01 Opening.flac"

DEV_PODMAN_ARGS="-v $CARD:/media/compas:z -v $MUSIC:/music:z -v $OUT:/shots:z" \
  scripts/dev.sh bash scripts/screenshots-container.sh
echo "Screenshots in $OUT"
