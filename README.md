# Compás Tools

Desktop companion for [Compás](https://github.com/Starnished66/compas-player) players,
built with Tauri 2 (Linux AppImage, macOS, Windows). It manages the SD card of a
player: browse and install plugins, layouts and themes from the catalog, and place
player updates on the card.

Design notes live in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), the plan in
[docs/ROADMAP.md](docs/ROADMAP.md), and the plugin store contract in
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
