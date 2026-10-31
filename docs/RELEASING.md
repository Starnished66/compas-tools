# Releasing Compás Tools

## Tauri updater signing

The in-app updater verifies a signature made by Tauri's updater signing key. This is separate
from operating-system code signing and notarization: it does not identify the publisher to
Windows, macOS Gatekeeper, or Linux package managers.

Create the updater key pair on a trusted maintainer machine, and keep the private key and its
password in the team's credential manager:

```sh
npx tauri signer generate -w "$HOME/.tauri/compas-tools.key"
```

The command creates the private key and a `.key.pub` public key file. Add the public key's
contents to `plugins.updater.pubkey` in `src-tauri/tauri.conf.json` (the public key is
safe to commit). Do not commit or upload the private key. Configure these repository Actions
secrets for tagged release builds:

| Secret                               | Value                                        |
| ------------------------------------ | -------------------------------------------- |
| `TAURI_SIGNING_PRIVATE_KEY`          | Complete contents of the private `.key` file |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password used when creating that key         |

You can set the private key without printing it in a terminal transcript with
`gh secret set TAURI_SIGNING_PRIVATE_KEY < "$HOME/.tauri/compas-tools.key"`. Set the password
with `gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD`; enter it at the hidden prompt. Keep
backups of both values in the team's credential manager. Losing them prevents compatible
updater artifacts from being produced for the existing public key.

Pull requests and branch builds do not receive signing secrets. They override
`bundle.createUpdaterArtifacts` to build unsigned installers for CI. A `v*` tag build requires
both secrets and creates the signed updater artifacts. Do not rotate the updater key unless
the published public key and migration strategy are updated deliberately; existing installs
trust the public key embedded in their build.

Updater artifacts must include their signed version. `requireSignedVersion` is enabled in the
Tauri config so a release manifest cannot pair a valid signature from an older artifact with a
different advertised version. Tauri CLI adds the app version while signing a build. Releases
without that signed version are rejected by the updater.

## Update channels

Stable is the default channel and reads the latest published stable GitHub release. Users can
opt into beta in the update panel. Beta versions use SemVer prerelease tags such as
`v0.2.0-beta.1`; the workflow first creates a draft. After a maintainer publishes that
prerelease, `beta-feed.yml` validates its signed artifacts and updates the fixed prerelease
feed at `updates-beta`. The feed manifest links to the immutable assets on the original tagged
release. Stable releases are never published automatically and do not update the beta feed.

## Release checklist

1. Update `version` in `package.json` and `src-tauri/tauri.conf.json` to the same release
   version. Confirm the updater endpoints and public key in `src-tauri/tauri.conf.json`.
2. Run the repository checks and build locally as appropriate for the change.
3. Push a tag matching the configured version, such as `v0.2.0`. For beta testing, use a
   prerelease version and tag such as `0.2.0-beta.1` and `v0.2.0-beta.1`.
4. GitHub Actions builds Linux x86_64 AppImage, macOS Apple Silicon app archive and DMG, and
   Windows NSIS and MSI installers. It requires a nonempty `.sig` alongside each updater
   installer (including both Windows installer formats), then generates `latest.json` from
   those files and checks the tag/version match and all required platforms are present.
5. Review the generated draft release, its `latest.json`, installers, `.sig` files, and
   `SHA256SUMS`. Publish the draft manually only after review. Until publication, the updater's
   `/releases/latest/download/latest.json` endpoint does not expose that release.

The workflow never publishes automatically. A failed signing or artifact validation step
blocks the draft release. PR and branch workflows build unsigned installers and do not need
the signing secrets.

## macOS signing and notarization

Tauri updater signatures provide artifact integrity and authenticity to the app's updater.
They are not platform code signatures. The macOS bundle defaults to an ad-hoc code signature,
which protects the app bundle from unsealed changes but does not identify the publisher to
Gatekeeper. With no Apple credentials, users may need to approve opening the downloaded app
manually in macOS privacy and security settings. The macOS CI job validates the DMG, app bundle,
Mach-O signatures, the main executable and `compas-sdtool` arm64 architectures, and that the
updater archive contains the same app signature. When Developer ID notarization is configured,
CI also validates the stapled ticket and Gatekeeper assessment.

For Developer ID signing and notarization on tagged releases, configure all six repository
Actions secrets below, or none. Tauri imports the certificate into a temporary keychain and
uses it to sign and notarize the app bundle before Tauri packages it in the updater archive and
DMG. `APPLE_SIGNING_IDENTITY` overrides the default ad-hoc identity in `tauri.conf.json`.

| Secret                       | Value                                              |
| ---------------------------- | -------------------------------------------------- |
| `APPLE_CERTIFICATE`          | Base64 encoded Developer ID `.p12` certificate     |
| `APPLE_CERTIFICATE_PASSWORD` | Password used to export the `.p12` certificate     |
| `APPLE_SIGNING_IDENTITY`     | Full `Developer ID Application: ...` identity name |
| `APPLE_ID`                   | Apple ID used for notarization                     |
| `APPLE_PASSWORD`             | App-specific password for that Apple ID            |
| `APPLE_TEAM_ID`              | Apple Developer team ID                            |

Configure these in the repository's Actions secrets. The workflow never prints their values.
Incomplete configuration fails a tagged build. With all six absent, it builds with an ad-hoc
signature and prints a notice that Gatekeeper may require manual approval.

## Other platform signing

- **Windows:** Authenticode signing certificate and timestamping for the NSIS and MSI
  installers.
- **Linux:** platform packages or repository metadata may need their own signing process if
  those distribution channels are introduced.

Windows and Linux platform credentials are not required by the current workflow. None of these
platform credentials substitute for `TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

The GitHub repository and its release feeds supply the updater's release metadata. Tauri does
not sign `latest.json`; it uses the embedded public key to verify each downloaded artifact and
its signed version before installation.
