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
src-tauri/                 Tauri app: commands, events, app state
  src/commands/            One file per feature area, thin wrappers
src/                       React + TypeScript UI (Vite)
  api/                     Typed wrappers around Tauri commands and events
  i18n/                    translate() shared with compas-app
  screens/                 One folder per screen
  components/              Reusable UI pieces
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

| Path on the card | Owner | Purpose |
| --- | --- | --- |
| `.compas/` | player | Marks a Compás card |
| `.compas/device.json` | player | Model, player version, plugin API level |
| `.compas/compas_player` | tools | Player-only update file |
| `<board>.upt` at the root | tools | Full firmware image (`r1`, `r3proii`, `r3ii_2025`) |
| `.plugins/` | shared | Plugins, layouts and themes |
| `.plugins/.store_installed` | shared | Store install record, same format as the player |
| `.plugins/.disabled` | shared | Disabled plugin file names, one per line |

The plugin store semantics mirror `src/network/plugin_store.c` in compas-player.
They are written up in [formats/plugin-store.md](formats/plugin-store.md).

## External sources

| Source | URL |
| --- | --- |
| Plugin catalog | `https://github.com/Starnished66/compas-plugins/releases/latest/download/index.json` |
| Catalog assets | `https://github.com/Starnished66/compas-plugins/releases/download/<tag>/<asset>` |
| Player releases | GitHub API, latest release of `Starnished66/compas-player` (assets plus `SHA256SUMS`) |

Every downloaded file is checked against its published size and SHA-256 before
it reaches the card.
