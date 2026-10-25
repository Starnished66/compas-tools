# Roadmap

## Phase 0: player-side groundwork (compas-player)

- `compas-sdtool`: library database and album art cache built from the
  player's own code, with JSON-lines progress.
- `.compas/device.json` written by the player.
- A database built in another time zone must not trigger a full rescan.
- Stable documentation for `.store_installed` and `.disabled`.

## Phase 1: app shell and card management

- App shell: navigation, theme (light and dark), error and progress patterns.
- Card detection: removable volumes that contain `.compas/`, plus a manual
  folder picker. Shows model, player version, space and file system.
- Plugins, layouts and themes: browse the catalog with previews, install,
  update, update all, uninstall, enable and disable, with the same on-card
  records as the player's own store.
- Updates on the card: compare with the latest player release, place the
  player-only update or the full `.upt`, and explain how to finish on the
  player.

## Phase 2: music and library

- Music transfer with verification, FAT checks and macOS clutter cleanup.
- Library database build and update through `compas-sdtool`.
- Album art cache build and refresh through `compas-sdtool`.

## Phase 3: ADB

- Guided ADB setup on stock firmware (About tapped 10 times), Windows driver.
- Device detection, first install and updates over ADB, with `/usr/data`
  backup.
- Log collection with redaction.

## Phase 4: release

- Translations (en, es, de, fr, it, ru).
- CI builds: AppImage, macOS `.dmg`, Windows installer (unsigned for now).
- Auto-update and QA on all three systems.
