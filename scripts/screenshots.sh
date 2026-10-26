#!/usr/bin/env bash
# Takes a screenshot of every screen of the built app, in the light and the dark scheme and in
# German. The app runs on a virtual display inside the build container, against a throwaway
# card folder and the fake adb (scripts/fake-adb.sh), so no real card or device is involved.
#
#   scripts/screenshots.sh [output-dir]                 uses the debug build
#   APP=/path/to/Compas.AppImage scripts/screenshots.sh   uses an AppImage
#
# Debug build: npm run build && scripts/dev.sh npx tauri build --debug --no-bundle
set -euo pipefail

cd "$(dirname "$0")/.."
OUT="$(realpath -m "${1:-docs/screenshots}")"
WORK="$HOME/.cache/compas-tools/screenshot-data"
CARD="$WORK/card"
MUSIC="$WORK/music"
FAKE="$WORK/fake-player"

rm -rf "$WORK"
mkdir -p "$CARD/.compas" "$CARD/.plugins" "$CARD/Artist/Album" "$MUSIC/Album" "$OUT" "$FAKE/remote"
echo '{"schema":1,"model":"r1","player_version":"1.1.0","plugin_api":17,"database_format":5,"art_cache":"v4"}' \
  > "$CARD/.compas/device.json"
echo "-- a plugin the store did not install" > "$CARD/.plugins/ABRepeat.lua"
head -c 300000 /dev/urandom > "$MUSIC/Album/01 Opening.flac"
head -c 200000 /dev/urandom > "$MUSIC/Album/02 AC:DC?.flac"

# A stock R1 on the other end of a pretend cable.
mkdir -p "$FAKE/remote/usr/data/.compas" "$FAKE/remote/data/mnt/sd_0"
printf 'List of devices attached\ningenic_1a2b3c        device usb:1-1 product:mako model:Nexus_4 device:mako transport_id:1\n\n' > "$FAKE/devices.txt"
echo "480,800" > "$FAKE/fb"
printf '  PID USER     COMMAND\n    1 root     /sbin/init\n  214 root     /usr/bin/hiby_player\n  230 root     /sbin/adbd\n' > "$FAKE/ps"
printf '[    0.0] Linux version 3.10.14\n[   12.5] wlan0: associated with "Casa Garita"\n' > "$FAKE/dmesg"
printf 'volume=12\nremote_control_pin=424242\n' > "$FAKE/remote/usr/data/.compas/settings.txt"
printf 'crash at 12:00\nlastfm token=abc123\n' > "$FAKE/remote/usr/data/player_crash.log"

APP_MOUNT=()
# Copied to a path without spaces, which the container options cannot carry.
if [ -n "${APP:-}" ]; then
  cp "$APP" "$WORK/Compas.AppImage"
  APP_MOUNT=(-v "$WORK/Compas.AppImage":/app/Compas.AppImage:ro,z)
fi

for run in ${RUNS:-light-en dark-en dark-de}; do
  DEV_PODMAN_ARGS="-v $CARD:/media/compas:z -v $MUSIC:/music:z -v $OUT:/shots:z -v $FAKE:/fake:z ${APP_MOUNT[*]:-}" \
    scripts/dev.sh bash scripts/screenshots-container.sh "$run"
done
echo "Screenshots in $OUT"
