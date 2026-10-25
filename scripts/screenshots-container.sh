#!/usr/bin/env bash
# Runs inside the build container; see screenshots.sh.
set -euo pipefail

export DISPLAY=:99 NO_AT_BRIDGE=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 LIBGL_ALWAYS_SOFTWARE=1
eval "$(dbus-launch --sh-syntax)"
Xvfb :99 -screen 0 1280x860x24 >/dev/null 2>&1 &
sleep 2

for scheme in light dark; do
  if [ "$scheme" = dark ]; then export GTK_THEME=Adwaita:dark; else unset GTK_THEME; fi
  /cache/target/debug/compas-tools >/dev/null 2>&1 &
  app=$!
  sleep 10
  win=$(xdotool search --name "Compás Tools" | head -1)
  xdotool windowmove "$win" 0 0 windowsize "$win" 1280 860
  sleep 2

  shot() { sleep "${2:-4}"; import -window root "/shots/$scheme-$1.png"; }
  nav() { xdotool mousemove 90 "$1" click 1; }  # sidebar rows, top to bottom

  shot 1-card
  nav 120; shot 2-music-empty 2
  # The file dialog is GTK's: type each path into its location bar.
  for file in "01 Opening.flac" "02 AC:DC?.flac"; do
    xdotool mousemove 307 131 click 1; sleep 3
    xdotool key ctrl+l; sleep 1
    xdotool type --delay 40 "/music/Album/$file"; sleep 1
    xdotool key Return; sleep 3
  done
  shot 2b-music-plan 3
  xdotool mousemove 287 627 click 1; sleep 5
  shot 2c-music-done 1
  nav 158; shot 3-library 5
  xdotool mousemove 310 156 click 1; sleep 8
  shot 3b-library-scanned 1
  nav 196; shot 4-plugins 12
  xdotool mousemove 348 230 click 1; sleep 3
  shot 4b-plugins-confirm 1
  xdotool key Escape; sleep 1
  xdotool mousemove 870 129 click 1; shot 4c-plugin-details 2
  nav 234; shot 5-layouts 3
  nav 272; shot 6-themes 3
  nav 310; shot 7-updates 6

  kill "$app" 2>/dev/null || true
  sleep 2
done
