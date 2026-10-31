## macOS installation (Apple Silicon)

This community build is not signed with an Apple Developer ID or notarized by Apple. macOS may show **“Compás Tools.app is damaged and can’t be opened”** when you launch it. Right-clicking and choosing **Open** may not resolve this warning.

For the official download from this repository:

1. Download the Apple Silicon `.dmg` and open it.
2. Drag **Compás Tools.app** into **Applications**.
3. Open **Terminal** and run:

   ```bash
   xattr -dr com.apple.quarantine "/Applications/Compás Tools.app"
   ```

4. Open Compás Tools from Applications again.

The command removes the download quarantine flag only from this app and its contents. Use the exact application name if you renamed it.

This workaround is for the app-launch warning. If the `.dmg` itself will not open, download it again and compare its SHA-256 checksum with the release’s `SHA256SUMS`. This build supports Apple Silicon Macs, not Intel Macs.
