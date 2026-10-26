# Releasing

The repository is private on GitHub and the CI builds run, but no signing key exists yet and the
releases have not been tried on real hardware.

## What the CI builds

`.github/workflows/build.yml` makes unsigned installers: an AppImage (Ubuntu 22.04), an Apple Silicon
`.dmg` (macOS, arm64 only, 11.0 or newer) and an NSIS `.exe` plus `.msi` (Windows). Each one bundles `compas-sdtool`,
built from `sdtool/` in this repository against the player's sources (`PLAYER_REPO` at
`PLAYER_REF`). The CI checks out both, then runs `make -C player -f Makefile -f
"$PWD/sdtool/sdtool.mk" sdtool`: the player's Makefile supplies the vendored libraries and flags,
`sdtool/sdtool.mk` the tool, and the output goes to `build/`, never into the player. The build id
that `info` prints names both revisions (`tools-<hash>+player-<hash>`).

- Linux: native, and `make test-sdtool` runs the ported end-to-end tests.
- Windows: cross-built with MinGW-w64 on a Linux runner (`SDTOOL_TARGET=windows`).
- macOS: built natively on an arm64 runner (`macos-14`); Intel Macs are not supported.

`scripts/verify-sdtool.sh` then runs `info` on each runnable binary and checks the build id and
that `database_format` equals the player's `TAGCACHE_DATABASE_FORMAT`. `scripts/build-sdtool.sh`
does the same flow locally (Linux and Windows in the build container).

`PLAYER_REF` pins the compas-player commit (the Makefile clones the vendored libraries itself).
Check the workflow with `actionlint` after any change.

**The sdtool and the player must match.** `compas-sdtool` writes one database format
(`compas-sdtool info` prints it as `database_format`), and the app builds a card's library only when
the card's `device.json` reports that same format. A player release that predates the format, or
writes no `device.json` (v1.2.0 and older), is left to build its own library. So a Compás Tools
release must bundle the sdtool of the player release it targets: before tagging, pin `PLAYER_REF`
to that player's release tag and check the new build's `info` against that player's
`database_format`.

## Publishing a release

Push a tag `vX.Y.Z` (after the version numbers below match it). The `release` job in
`build.yml` runs once all three installers built, and creates a **draft** GitHub release holding
them with generated notes. It never publishes: open the draft on GitHub, check the files and the
notes, and press Publish yourself.

Unsigned builds work but warn: macOS Gatekeeper asks the user to open the app from the context
menu, and Windows SmartScreen shows "unknown publisher". Code signing (an Apple Developer ID with
notarization, a Windows Authenticode certificate) is a separate step that needs the owner's
accounts; Tauri reads the usual `APPLE_*` and `WINDOWS_CERTIFICATE*` secrets.

## Updates of the app itself (off until a key exists)

The updater is compiled in only with `--features updater` and configured by
`src-tauri/tauri.updater.conf.json`, so ordinary builds never look for updates. The Updates screen
shows its panel only in builds that have the feature. To switch it on, the owner must:

1. Generate a signing key pair, keep the private key safe, and never commit it:
   `npx tauri signer generate -w ~/.tauri/compas-tools.key`
2. Put the **public** key (the content of `compas-tools.key.pub`) into `pubkey` in
   `src-tauri/tauri.updater.conf.json`, and check that `endpoints` names the real repository.
3. Add two repository secrets: `TAURI_SIGNING_PRIVATE_KEY` (the private key's content) and
   `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
4. Build the releases with the updater on:
   `npx tauri build --features updater --config src-tauri/tauri.updater.conf.json`
   This also writes an update bundle and a `.sig` file next to each installer.
5. Publish a GitHub release (tag `vX.Y.Z`) holding the installers, the update bundles and `.sig`
   files, and a `latest.json` listing each platform's URL and signature. The
   `tauri-apps/tauri-action` action can build, sign and publish all of this; switch the
   "Build the installer" step to it. The app reads
   `https://github.com/<owner>/compas-tools/releases/latest/download/latest.json`.

A private repository cannot serve `latest.json` or the update bundles to users without a login, so
the updater only works once the releases are public, or once `endpoints` points at another host
that serves them openly.

Lose the private key and installed copies can no longer be updated: users would have to install
a new build by hand, so back it up.

## Version numbers

Change them together: `package.json`, `[workspace.package]` in `Cargo.toml`, and `version` in
`src-tauri/tauri.conf.json`.

## Still to be checked on real hardware

The CI cannot test these: recovery mode through `bootmode.sh`, the names of the player's real
Wi-Fi and PIN files, ADB behaviour of each board, and the Windows driver step.
