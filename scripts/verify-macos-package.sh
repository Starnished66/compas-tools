#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: $0 <disk-image.dmg> [updater-app.tar.gz]" >&2
  exit 2
}

[[ $# -ge 1 && $# -le 2 ]] || usage
dmg=$1
[[ -f "$dmg" ]] || { echo "DMG does not exist: $dmg" >&2; exit 1; }
archive=
if [[ $# -eq 2 ]]; then
  archive=$2
  [[ -n "$archive" ]] || { echo "Updater app archive path is empty" >&2; exit 1; }
  [[ -f "$archive" ]] || { echo "Updater app archive does not exist: $archive" >&2; exit 1; }
fi

work=$(mktemp -d)
mount="$work/mount"
mkdir "$mount"
mounted=0
cleanup() {
  if [[ "$mounted" == 1 ]]; then hdiutil detach "$mount" -quiet || true; fi
  rm -rf "$work"
}
trap cleanup EXIT

verify_app() {
  local app=$1 sidecar main main_name main_arches sidecar_arches hash info card binary
  [[ -d "$app/Contents" ]] || { echo "Not a macOS app bundle: $app" >&2; return 1; }
  [[ -f "$app/Contents/_CodeSignature/CodeResources" ]] || {
    echo "App signature manifest is missing: $app/Contents/_CodeSignature/CodeResources" >&2
    return 1
  }
  codesign --verify --deep --strict --verbose=2 "$app" || {
    echo "App bundle signature verification failed: $app" >&2
    return 1
  }

  sidecar="$app/Contents/MacOS/compas-sdtool"
  [[ -f "$sidecar" ]] || { echo "Bundled compas-sdtool sidecar is missing: $sidecar" >&2; return 1; }
  main_name=$(plutil -extract CFBundleExecutable raw -o - "$app/Contents/Info.plist") || {
    echo "Unable to read CFBundleExecutable from $app/Contents/Info.plist" >&2
    return 1
  }
  main="$app/Contents/MacOS/$main_name"
  [[ -f "$main" ]] || { echo "Main app executable is missing: $main" >&2; return 1; }

  local macho_count=0
  while IFS= read -r -d '' binary; do
    if file -b "$binary" | grep -q 'Mach-O'; then
      macho_count=$((macho_count + 1))
      codesign --verify --strict --verbose=2 "$binary" || {
        echo "Mach-O signature verification failed: $binary" >&2
        return 1
      }
    fi
  done < <(find "$app" -type f -print0)
  [[ "$macho_count" -gt 0 ]] || { echo "No Mach-O executables found in $app" >&2; return 1; }

  sidecar_arches=$(lipo -archs "$sidecar") || { echo "Unable to read sidecar architecture: $sidecar" >&2; return 1; }
  [[ " $sidecar_arches " == *" arm64 "* ]] || {
    echo "compas-sdtool must contain arm64; found: $sidecar_arches" >&2
    return 1
  }
  main_arches=$(lipo -archs "$main") || { echo "Unable to read main executable architecture: $main" >&2; return 1; }
  [[ " $main_arches " == *" arm64 "* ]] || {
    echo "Main app executable must contain arm64; found: $main_arches" >&2
    return 1
  }

  card=$(mktemp -d "$work/card.XXXXXX")
  mkdir "$card/.compas"
  info=$("$sidecar" info "$card") || { echo "Bundled compas-sdtool info command failed" >&2; return 1; }
  [[ "$info" == *'"build":'* && "$info" == *'"database_format":'* ]] || {
    echo "Bundled compas-sdtool returned incomplete info: $info" >&2
    return 1
  }
  rm -rf "$card"

  if [[ "${EXPECT_NOTARIZED:-0}" == 1 ]]; then
    xcrun stapler validate "$app" >&2 || { echo "App notarization ticket is invalid: $app" >&2; return 1; }
    spctl --assess --type execute --verbose "$app" >&2 || { echo "Gatekeeper rejected: $app" >&2; return 1; }
  fi

  hash=$(codesign -dv --verbose=4 "$app" 2>&1 | sed -n 's/^CDHash=//p' | head -1)
  [[ -n "$hash" ]] || { echo "Unable to read the app signature CDHash: $app" >&2; return 1; }
  printf '%s\n' "$hash"
}

hdiutil verify "$dmg"
hdiutil attach "$dmg" -mountpoint "$mount" -nobrowse -readonly -quiet
mounted=1
app=$(find "$mount" -maxdepth 3 -type d -name '*.app' -print -quit)
[[ -n "$app" ]] || { echo "No .app bundle found in DMG: $dmg" >&2; exit 1; }
dmg_hash=$(verify_app "$app")
hdiutil detach "$mount" -quiet
mounted=0

if [[ -n "$archive" ]]; then
  mkdir "$work/archive"
  tar -xzf "$archive" -C "$work/archive"
  archive_app=$(find "$work/archive" -type d -name '*.app' -print -quit)
  [[ -n "$archive_app" ]] || { echo "No .app bundle found in updater archive: $archive" >&2; exit 1; }
  archive_hash=$(verify_app "$archive_app")
  [[ "$dmg_hash" == "$archive_hash" ]] || {
    echo "DMG and updater archive contain different app signatures ($dmg_hash != $archive_hash)" >&2
    exit 1
  }
  echo "DMG and updater app archive are valid and contain the same signed arm64 app ($dmg_hash)."
else
  echo "DMG is valid and contains a signed arm64 app ($dmg_hash)."
fi
