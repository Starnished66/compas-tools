# Releasing

Nothing here has been run yet: the repository has no remote, and no signing key exists.

## What the CI builds

`.github/workflows/build.yml` makes unsigned installers: an AppImage (Ubuntu 22.04), an Apple Silicon
`.dmg` (macOS, arm64 only, 11.0 or newer) and an NSIS `.exe` plus `.msi` (Windows). Each one bundles `compas-sdtool`,
built from the player's sources (`PLAYER_REPO` at `PLAYER_REF`):

- Linux: `make sdtool`.
- Windows: cross-built with MinGW-w64 on a Linux runner (the way the player documents it).
- macOS: built natively on an arm64 runner (`macos-14`); Intel Macs are not supported.

Before the first run, move `PLAYER_REF` to a tag or commit of compas-player that contains
`compas-sdtool` (the Makefile clones the vendored libraries itself). Check the workflow with
`actionlint` after any change.

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
   "Build the installer" step to it when the repository exists. The app reads
   `https://github.com/<owner>/compas-tools/releases/latest/download/latest.json`.

Lose the private key and installed copies can no longer be updated: users would have to install
a new build by hand, so back it up.

## Version numbers

Change them together: `package.json`, `[workspace.package]` in `Cargo.toml`, and `version` in
`src-tauri/tauri.conf.json`.

## Still to be checked on real hardware

The CI cannot test these: recovery mode through `bootmode.sh`, the names of the player's real
Wi-Fi and PIN files, ADB behaviour of each board, and the Windows driver step.
