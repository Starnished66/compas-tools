# Compás Tools

A desktop app for managing a player running [Compás](https://github.com/Starnished66/compas-player). Connect the player to your computer and Compás Tools takes care of:

- installing Compás and keeping it up to date;
- adding music and updating the library database;
- installing and updating plugins, layouts and themes.

It runs on Linux (AppImage), macOS on Apple Silicon and Windows. Download it from [Releases](https://github.com/Starnished66/compas-tools/releases). The [user guide](https://compasplayer.org/guide.html) explains the player itself.

## Building it yourself

You need Node 22. On Linux the Rust side builds in a Podman container (`Containerfile`), so you don't need Rust installed. On macOS and Windows, install Rust and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) and run the same commands directly.

The app bundles `compas-sdtool`, the player's own library code as a command-line tool. Its sources are in `sdtool/` and build against a `compas-player` checkout at the commit pinned as `PLAYER_REF` in `.github/workflows/build.yml` (its sources are not changed and the output goes to `build/`, but its Makefile fetches its libraries into the checkout, so use a separate `git worktree` of the player):

```sh
npm install
SDTOOL_SRC=/path/to/compas-player scripts/build-sdtool.sh   # default: ../compas-player
scripts/build-appimage.sh        # writes artifacts/*.AppImage
```

For development, `npm run dev` runs the interface in a browser without the backend. `cargo tauri dev` runs the full app on Linux if the Tauri prerequisites are installed. `npm run typecheck` and `npm run lint` check the interface code.

## Licence

GPL-3.0. See [LICENSE](LICENSE), and [THIRD_PARTY.md](THIRD_PARTY.md) for the icons and font.
