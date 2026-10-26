# Architecture

Compás Tools is a desktop companion for Compás players, built with Tauri 2. It
runs on Linux (AppImage), macOS and Windows.

## Principles

- **The player owns its formats.** Anything stored on the card (library
  database, album art cache, plugin records) is either written by
  `compas-sdtool`, which is built from compas-player's own C code, or follows
  a format the player documents as stable. Never reverse-engineer a format the
  player can produce for us.
- **Thin UI, testable core.** All logic lives in the `compas-core` Rust crate,
  which has no Tauri dependency and is unit-tested against temporary folders
  laid out like a card. The Tauri crate only adapts core functions into
  commands and events.
- **Small, single-purpose files.** No file exceeds 500 lines
  (`scripts/check-file-length.sh` enforces it). Shared behaviour such as
  verified downloads and atomic writes has one implementation.
- **No speculative abstraction.** Add a module, crate, dependency or setting
  only when a feature needs it.

## Layout

```
crates/compas-core/        Pure Rust library: all behaviour
  src/card/                Card discovery, device.json, safe paths
  src/net/                 HTTP client and verified downloads
  src/fs/                  Atomic writes, staging, free space
  src/catalog/             Plugin catalog, installed record, enable/disable
  src/releases/            Player releases and card update placement
  src/music/               Transfer planning, verified copy, FAT rules, macOS clutter
  src/sdtool/              compas-sdtool sidecar: library database and album art
src-tauri/                 Tauri app: commands, events, app state
  src/commands/            One file per feature area, thin wrappers
src/                       React + TypeScript UI (Vite)
  api/                     Typed wrappers around Tauri commands and events
  i18n/                    translate() shared with compas-app
  screens/                 One folder per screen
  components/              Reusable UI pieces
sdtool/                    C source of compas-sdtool (see SDTOOL.md); builds against a compas-player checkout
docs/                      Architecture, roadmap, on-card formats
scripts/                   Developer and CI helpers
Containerfile              Linux build environment (Ubuntu 22.04 base)
```

## Data flow

1. The UI calls a typed function in `src/api/`, which invokes a Tauri command.
2. The command calls `compas-core` and returns a serialisable result or a
   typed error (`{ code, message }`).
3. Long operations (downloads, installs, scans) report progress through one
   Tauri event channel per operation: `{ phase, done, total }`.

## On-card layout used by phase 1

| Path on the card            | Owner  | Purpose                                            |
| --------------------------- | ------ | -------------------------------------------------- |
| `.compas/`                  | player | Marks a Compás card                                |
| `.compas/device.json`       | player | Model, player version, plugin API level            |
| `.compas/compas_player`     | tools  | Player-only update file                            |
| `<board>.upt` at the root   | tools  | Full firmware image (`r1`, `r3proii`, `r3ii_2025`) |
| `.plugins/`                 | shared | Plugins, layouts and themes                        |
| `.plugins/.store_installed` | shared | Store install record, same format as the player    |
| `.plugins/.disabled`        | shared | Disabled plugin file names, one per line           |

The plugin store semantics mirror `src/network/plugin_store.c` in compas-player.
They are written up in [formats/plugin-store.md](formats/plugin-store.md).

## External sources

| Source          | URL                                                                                   |
| --------------- | ------------------------------------------------------------------------------------- |
| Plugin catalog  | `https://github.com/Starnished66/compas-plugins/releases/latest/download/index.json`  |
| Catalog assets  | `https://github.com/Starnished66/compas-plugins/releases/download/<tag>/<asset>`      |
| Player releases | GitHub API, latest release of `Starnished66/compas-player` (assets plus `SHA256SUMS`) |

Every downloaded file is checked against its published size and SHA-256 before
it reaches the card.

## ADB (Device screen)

The Device screen installs and updates the player over USB and collects logs, using `adb`:
a system `adb` when there is one, otherwise Google's platform-tools downloaded on first use
(after asking) into the app's data folder: a pinned version (r37.0.1) fetched over HTTPS and
checked against its SHA-256 before it is unpacked. Every adb call goes through
`crates/compas-core/src/adb`.

Set `COMPAS_ADB` to the path of any program that behaves like `adb` to use it instead. The
tests do this with a fake adb (`adb/fake.rs`): a shell script serving a pretend player with
recorded outputs for stock and Compás firmware on each board. No test touches a real device.

The backup reads only an allowlist of `/usr/data` entries (settings, EQ, theme overrides, the
player's logs), one file at a time, and masks each file in memory before it is written. The SD
card (`/usr/data/mnt`), `.compas/plugins`, server passwords and the PIN files are never read.

## Repository and tests

| Path                 | Contents                                                                |
| -------------------- | ----------------------------------------------------------------------- |
| `crates/compas-core` | Rust library with all behaviour; no Tauri dependency                    |
| `src-tauri`          | Tauri app: thin commands and app state                                  |
| `src`                | React + TypeScript UI                                                   |
| `sdtool`             | C source of compas-sdtool                                               |
| `scripts`            | `dev.sh` (build container), `build-appimage.sh`, `check-file-length.sh` |

One integration test uses the real plugin catalog and installs a small plugin into a
temporary card. It is skipped by default:

```sh
scripts/dev.sh cargo test -p compas-core -- --ignored
```
