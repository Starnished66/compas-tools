#!/usr/bin/env python3
"""Build the Tauri updater manifest from the signed release artifacts."""

import argparse
import datetime as dt
import json
import re
import sys
from pathlib import Path
from urllib.parse import quote


def exactly_one(root: Path, pattern: str, description: str) -> Path:
    matches = sorted(root.glob(pattern))
    if len(matches) != 1 or not matches[0].is_file():
        raise ValueError(f"expected exactly one {description}; found {len(matches)}")
    return matches[0]


def read_pair(root: Path, pattern: str, description: str) -> tuple[Path, str]:
    artifact = exactly_one(root, pattern, description)
    signature_path = Path(f"{artifact}.sig")
    if not signature_path.is_file():
        raise ValueError(f"missing signature for {artifact.name}")
    signature = signature_path.read_text(encoding="utf-8").strip()
    if not signature:
        raise ValueError(f"empty signature for {artifact.name}")
    return artifact, signature


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    config = json.loads(Path("src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
    package = json.loads(Path("package.json").read_text(encoding="utf-8"))
    version = config.get("version")
    if not isinstance(version, str) or not re.fullmatch(r"\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?", version):
        raise ValueError("src-tauri/tauri.conf.json must contain a valid semantic version")
    if args.tag != f"v{version}":
        raise ValueError(f"release tag {args.tag!r} does not match configured version v{version}")
    if package.get("version") != version:
        raise ValueError("package.json and src-tauri/tauri.conf.json versions must match")

    root = args.artifacts
    linux, linux_sig = read_pair(root, "**/*.AppImage", "Linux AppImage")
    mac_app, mac_sig = read_pair(root, "**/*.app.tar.gz", "macOS app archive")
    windows_exe, windows_sig = read_pair(root, "**/*.exe", "Windows NSIS installer")
    # Both Windows installer formats are shipped. The updater uses NSIS; validate and ship the
    # MSI signature too, so every downloadable installer has its matching updater signature.
    read_pair(root, "**/*.msi", "Windows MSI installer")
    exactly_one(root, "**/*.dmg", "macOS DMG installer")

    base = f"https://github.com/{args.repository}/releases/download/{quote(args.tag, safe='')}/"

    def entry(path: Path, signature: str) -> dict[str, str]:
        return {"signature": signature, "url": base + quote(path.name, safe="")}

    manifest = {
        "version": version,
        "notes": f"Compás Tools {version}",
        "pub_date": dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z"),
        "platforms": {
            "linux-x86_64": entry(linux, linux_sig),
            "darwin-aarch64": entry(mac_app, mac_sig),
            "windows-x86_64": entry(windows_exe, windows_sig),
        },
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote updater manifest for {version} with all required platform artifacts.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"updater manifest error: {error}", file=sys.stderr)
        raise SystemExit(1)
