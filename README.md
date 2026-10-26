# Compás Tools

Desktop companion for [Compás](https://github.com/Starnished66/compas-player) players,
built with Tauri 2 (Linux AppImage, macOS on Apple Silicon, Windows). It manages the SD card of a
player: browse and install plugins, layouts and themes from the catalog, and place
player updates on the card.

Design notes live in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and the plugin store contract in
[docs/formats/plugin-store.md](docs/formats/plugin-store.md).

## Layout

| Path                 | Contents                                                                |
| -------------------- | ----------------------------------------------------------------------- |
| `crates/compas-core` | Rust library with all behaviour; no Tauri dependency                    |
| `src-tauri`          | Tauri app: thin commands and app state                                  |
| `src`                | React + TypeScript UI                                                   |
| `scripts`            | `dev.sh` (build container), `build-appimage.sh`, `check-file-length.sh` |

## Requirements

- Node 22 and npm on the host (frontend).
- Podman for everything Rust and Tauri on Linux: the `Containerfile` provides
  Ubuntu 22.04 with the Tauri 2 prerequisites, Rust stable (clippy, rustfmt) and Node 22.
  The image is built on first use. Cargo, npm and target caches live in the
  `compas-tools-cache` volume.

On macOS and Windows install Rust and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
and run the same `cargo` / `npm` commands directly.

## Library tool (compas-sdtool)

The library database and album-art cache are built by `compas-sdtool`, the player's own
library code as a command-line program. It is bundled as a Tauri `externalBin`, so the
binary for the build target must exist in `src-tauri/binaries/` before any `cargo` or
`tauri` command that builds `src-tauri` (it is not committed):

```sh
scripts/build-sdtool.sh                         # Linux, built in the container
SDTOOL_TARGET=windows scripts/build-sdtool.sh   # Windows .exe (MinGW, in the container)
SDTOOL_SRC=/path/to/worktree scripts/build-sdtool.sh
```

The sources default to `~/.cache/compas-tools/player-sdtool` (compas-player, branch
`feature/compas-sdtool`; the CLI is described in its `docs/SDTOOL.md`). On macOS the script
builds natively. Output names follow Tauri's target-triple convention.

## ADB (Device screen)

The Device screen installs and updates the player over USB and collects logs, using `adb`:
a system `adb` when there is one, otherwise Google's platform-tools downloaded on first use
(after asking) into the app's data folder: a pinned version (r37.0.1) fetched over HTTPS and checked against its SHA-256 before it is unpacked. Every adb call goes through `crates/compas-core/src/adb`.

Set `COMPAS_ADB` to the path of any program that behaves like `adb` to use it instead. The
tests do this with a fake adb (`adb/fake.rs`): a shell script serving a pretend player with
recorded outputs for stock and Compás firmware on each board. No test touches a real device.

The backup reads only an allowlist of `/usr/data` entries (settings, EQ, theme overrides, the
player's logs), one file at a time, and masks each file in memory before it is written. The SD
card (`/usr/data/mnt`), `.compas/plugins`, server passwords and the PIN files are never read.

## Run

```sh
npm install
scripts/dev.sh npx tauri dev     # needs a display; see below
npm run dev                      # frontend only, in a browser (Tauri commands fail)
```

`tauri dev` opens a window, which the container cannot do by default. On Linux either run
`cargo tauri dev` natively with the Tauri prerequisites installed, or test the packaged
AppImage.

## Build

```sh
scripts/build-appimage.sh        # writes artifacts/*.AppImage
```

The design follows KDE's Breeze: see [THIRD_PARTY.md](THIRD_PARTY.md) for the icons, font and
colour values it uses.

## Test and lint

```sh
npm run typecheck && npm run lint && npm test

scripts/dev.sh cargo fmt --all -- --check
scripts/dev.sh cargo clippy --workspace --all-targets -- -D warnings
scripts/dev.sh cargo test --workspace

scripts/check-file-length.sh     # no file over 500 lines
```

One integration test uses the real plugin catalog and installs a small plugin into a
temporary card. It is skipped by default:

```sh
scripts/dev.sh cargo test -p compas-core -- --ignored
```

## Licence

GPL-3.0, see [LICENSE](LICENSE).
