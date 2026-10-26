#!/usr/bin/env bash
# Runs `<sdtool> info` on an empty card and checks it against a compas-player checkout:
# the build id names that checkout's revision (and equals $SDTOOL_EXPECT_BUILD when set), and
# database_format equals the player's TAGCACHE_DATABASE_FORMAT.
#
#   scripts/verify-sdtool.sh /path/to/compas-player path/to/compas-sdtool
set -euo pipefail

player=$1; shift
card=$(mktemp -d)
trap 'rm -rf "$card"' EXIT
mkdir "$card/.compas"
info=$("$@" info "$card")
echo "$info"

build=$(sed -n 's/.*"build":"\([^"]*\)".*/\1/p' <<< "$info")
format=$(sed -n 's/.*"database_format":\([0-9]*\).*/\1/p' <<< "$info")
want_format=$(sed -n 's/^#define TAGCACHE_DATABASE_FORMAT \([0-9][0-9]*\).*/\1/p' \
  "$player/src/library/tagcache/core/tagcache.h")
player_rev=$(git -C "$player" rev-parse --short HEAD)

case "$build" in
  tools-*+player-"$player_rev"*) ;;
  *) echo "build '$build' does not name player $player_rev" >&2; exit 1 ;;
esac
if [ -n "${SDTOOL_EXPECT_BUILD:-}" ] && [ "$build" != "$SDTOOL_EXPECT_BUILD" ]; then
  echo "build '$build', expected '$SDTOOL_EXPECT_BUILD'" >&2; exit 1
fi
[ -n "$want_format" ] && [ "$format" = "$want_format" ] || {
  echo "database_format '$format', the player writes '$want_format'" >&2; exit 1
}
