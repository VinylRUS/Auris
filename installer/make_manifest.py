#!/usr/bin/env python3
"""Генерирует latest.json — манифест автообновления Auris.

Запускается в CI (job publish) после сборки всех артефактов:

    python3 installer/make_manifest.py <тег> <примечание>

Читает версию из Cargo.toml (workspace.package.version), считает SHA-256
артефактов в текущем каталоге и пишет latest.json рядом с ними.
"""

import glob
import hashlib
import json
import os
import sys

import tomllib

REPO = "VinylRUS/Auris"


def sha256(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main() -> None:
    if len(sys.argv) < 2:
        raise SystemExit("usage: make_manifest.py <tag> [notes]")
    tag = sys.argv[1]
    notes = sys.argv[2] if len(sys.argv) > 2 else ""

    with open("Cargo.toml", "rb") as f:
        version = tomllib.load(f)["workspace"]["package"]["version"]

    base = f"https://github.com/{REPO}/releases/download/{tag}"

    platforms: dict[str, dict] = {}

    installer = sorted(glob.glob("Auris-Setup-*.exe"))
    if installer:
        name = os.path.basename(installer[-1])
        platforms["windows-x86_64"] = {
            "url": f"{base}/{name}",
            "sha256": sha256(name),
        }

    for fname, key in [
        ("auris-linux-x86_64.tar.gz", "linux-x86_64"),
        ("auris-macos-aarch64.tar.gz", "macos-aarch64"),
    ]:
        if os.path.exists(fname):
            platforms[key] = {
                "url": f"{base}/{fname}",
                "sha256": sha256(fname),
            }

    manifest = {
        "version": version,
        "tag": tag,
        "notes": notes,
        "platforms": platforms,
    }
    with open("latest.json", "w", encoding="utf-8") as f:
        json.dump(manifest, f, ensure_ascii=False, indent=2)
        f.write("\n")

    print(json.dumps(manifest, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
