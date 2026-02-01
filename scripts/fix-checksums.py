#!/usr/bin/env python3
"""Clean and regenerate SHA256SUMS files for an election directory."""

from __future__ import annotations

import argparse
import hashlib
import sys
from pathlib import Path


EXCLUDE = {
    "README.md",
    "fetch.sh",
    "config.txt",
    "SHA256SUMS",
    ".DS_Store",
    ".lock",
    ".todo",
}


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def list_eligible_files(directory: Path) -> list[Path]:
    return sorted(
        path
        for path in directory.iterdir()
        if path.is_file() and path.name not in EXCLUDE and path.suffix != ".txt"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Clean and regenerate SHA256SUMS for an election directory"
    )
    parser.add_argument(
        "--election",
        default="2025-TK",
        help="Relative path to the election directory (default: 2025-TK)",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path(__file__).resolve().parents[1]
    batch_dir = repo_root / args.election
    if not batch_dir.exists():
        print(f"{args.election} directory missing", file=sys.stderr)
        return 1

    removed_empty = 0
    regenerated = 0
    skipped = 0

    for muni_dir in sorted(path for path in batch_dir.iterdir() if path.is_dir()):
        checksum_file = muni_dir / "SHA256SUMS"
        if not checksum_file.exists():
            continue

        if checksum_file.stat().st_size == 0:
            checksum_file.unlink()
            removed_empty += 1
            continue

        eligible = list_eligible_files(muni_dir)
        if not eligible:
            checksum_file.unlink()
            removed_empty += 1
            continue

        lines = [f"{sha256_file(path)}  {path.name}" for path in eligible]
        checksum_file.write_text("\n".join(lines) + "\n")
        regenerated += 1

    print(f"Removed empty SHA256SUMS : {removed_empty}")
    print(f"Regenerated SHA256SUMS   : {regenerated}")
    print(f"Skipped (no SHA256SUMS)  : {skipped}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
