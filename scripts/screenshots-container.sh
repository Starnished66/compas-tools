#!/usr/bin/env bash
# Runs inside the build container; see screenshots.sh.
set -euo pipefail

export DISPLAY=:99 NO_AT_BRIDGE=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 LIBGL_ALWAYS_SOFTWARE=1
export APPIMAGE_EXTRACT_AND_RUN=1 COMPAS_ADB=/work/scripts/fake-adb.sh FAKE_ADB_DIR=/fake
eval "$(dbus-launch --sh-syntax)"
Xvfb :99 -screen 0 1280x860x24 >/dev/null 2>&1 &
sleep 2

APP=/cache/target/debug/compas-tools
[ -e /app/Compas.AppImage ] && APP=/app/Compas.AppImage

# One run: $1 is "<scheme>-<language>", e.g. dark-de. Each run gets its own container, because
# a second instance of the app would only focus the first one (the app allows a single instance).
run=${1:?usage: screenshots-container.sh <scheme>-<language>}
scheme=${run%-*}
lang=${run#*-}
# The AppImage ignores GTK_THEME, so the preference is also written where GTK reads it.
mkdir -p "$HOME/.config/gtk-3.0"
if [ "$scheme" = dark ]; then
  export GTK_THEME=Adwaita:dark
  printf '[Settings]\ngtk-application-prefer-dark-theme=true\n' > "$HOME/.config/gtk-3.0/settings.ini"
else
  export GTK_THEME=Adwaita
  printf '[Settings]\ngtk-application-prefer-dark-theme=false\n' > "$HOME/.config/gtk-3.0/settings.ini"
fi
export LANG=C.UTF-8

"$APP" >/dev/null 2>&1 &
sleep 12
win=$(xdotool search --name "Compás Tools" | head -1)
xdotool windowmove "$win" 0 0 windowsize "$win" 1280 860
sleep 2

shot() { sleep "${2:-4}"; import -window root "/shots/$run-$1.png"; }
nav() { xdotool mousemove 90 "$((70 + 38 * $1))" click 1; }  # sidebar rows, top to bottom

# Switch the language with the app's own picker, as a user would (typing the first letters picks the entry).
if [ "$lang" = de ]; then
  xdotool mousemove 100 829 click 1; sleep 1
  xdotool type --delay 100 Deu; xdotool key Return; sleep 2
fi

shot 01-card
nav 1; shot 02-device 5
xdotool mousemove 700 500 click 5 click 5 click 5 click 5 click 5; shot 03-device-install 2
nav 2; shot 04-music 2
nav 3; shot 05-library 3
nav 4; shot 06-plugins 12
nav 5; shot 07-layouts 3
nav 6; shot 08-themes 3
nav 7; shot 09-updates 6
