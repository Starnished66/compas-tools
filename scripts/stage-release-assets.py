#!/usr/bin/env python3
"""Preflight and flatten Tauri release artifacts into GitHub-safe asset names."""

import argparse
import re
import shutil
import sys
from pathlib import Path

ARTIFACT_SUFFIXES = (".AppImage", ".app.tar.gz", ".exe", ".msi", ".dmg")
REQUIRED_ARTIFACTS = (".AppImage", ".app.tar.gz", ".exe", ".msi", ".dmg")
SAFE_BASENAME = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]*\Z")


def artifact_suffix(name: str) -> str | None:
    return next((suffix for suffix in ARTIFACT_SUFFIXES if name.endswith(suffix)), None)


def stage(source: Path, destination: Path) -> list[tuple[Path, str]]:
    source = source.resolve(strict=True)
    destination = destination.resolve()
    if not source.is_dir():
        raise ValueError(f"input is not a directory: {source}")
    if destination == source or source in destination.parents:
        raise ValueError("output directory must be outside the recursive input directory")
    if destination.exists() and (not destination.is_dir() or any(destination.iterdir())):
        raise ValueError(f"output directory must be absent or empty: {destination}")

    files: list[tuple[Path, str]] = []
    names: dict[str, Path] = {}
    counts = {suffix: 0 for suffix in REQUIRED_ARTIFACTS}
    for path in sorted(source.rglob("*")):
        if path.is_symlink():
            raise ValueError(f"symlinks are not allowed: {path.relative_to(source)}")
        if not path.is_file():
            continue
        name = path.name
        if name.startswith("Compás Tools"):
            name = "Compas.Tools" + name[len("Compás Tools") :]
        if not SAFE_BASENAME.fullmatch(name):
            raise ValueError(f"asset basename is not safe ASCII: {path.name!r}")
        suffix = artifact_suffix(name)
        if suffix:
            counts[suffix] += 1
        elif name.endswith(".sig"):
            signed_name = name[:-4]
            if not any(signed_name.endswith(allowed) for allowed in ARTIFACT_SUFFIXES if allowed != ".dmg"):
                raise ValueError(f"signature does not match a supported installer: {path.name}")
        else:
            raise ValueError(f"unexpected release asset type: {path.name}")
        previous = names.get(name)
        if previous is not None:
            raise ValueError(
                f"flattened filename collision: {previous.relative_to(source)} and {path.relative_to(source)} -> {name}"
            )
        names[name] = path
        files.append((path, name))

    for suffix, count in counts.items():
        if count != 1:
            raise ValueError(f"expected exactly one {suffix} installer; found {count}")
    for path, name in files:
        if name.endswith(".sig") and name[:-4] not in names:
            raise ValueError(f"orphan signature: {path.relative_to(source)}")
    for path, name in files:
        if artifact_suffix(name) and not name.endswith(".dmg") and f"{name}.sig" not in names:
            raise ValueError(f"missing signature for installer: {path.relative_to(source)}")

    # All input files, names, required installers, signature pairs, and collisions are checked
    # before creating the destination or copying any bytes.
    destination.mkdir(parents=True, exist_ok=True)
    for path, name in files:
        shutil.copyfile(path, destination / name)
    return files


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True, help="recursive Actions artifact directory")
    parser.add_argument("--output", type=Path, required=True, help="flat output directory")
    args = parser.parse_args()
    try:
        files = stage(args.input, args.output)
    except (OSError, ValueError) as error:
        print(f"release asset staging failed: {error}", file=sys.stderr)
        return 1
    print(f"Staged {len(files)} release assets into {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
