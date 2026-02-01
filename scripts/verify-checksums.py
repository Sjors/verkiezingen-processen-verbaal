#!/usr/bin/env python3
"""Verify SHA256SUMS for all municipalities in an election directory."""

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


def list_expected_files(directory: Path) -> set[str]:
    return {
        path.name
        for path in directory.iterdir()
        if path.is_file() and path.name not in EXCLUDE and path.suffix != ".txt"
    }


def parse_checksums(checksum_file: Path) -> list[tuple[str, str]]:
    entries: list[tuple[str, str]] = []
    with checksum_file.open() as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            if "  " not in line:
                print(f"INVALID: {checksum_file} -> {line}")
                continue
            expected, filename = line.split("  ", 1)
            if filename.startswith("./"):
                filename = filename[2:]
            entries.append((expected, filename))
    return entries


def verify_checksums(directory: Path, checksum_file: Path) -> bool:
    ok = True
    entries = parse_checksums(checksum_file)
    if not entries:
        print(f"EMPTY: {checksum_file}")
        return False

    checksum_files: set[str] = set()
    for expected, filename in entries:
        checksum_files.add(filename)

        if filename in {".lock", ".todo"}:
            print(f"INVALID: {directory.name} SHA256SUMS includes {filename}")
            ok = False
            continue

        filepath = directory / filename
        if not filepath.exists():
            print(f"MISSING: {directory.name} -> {filename}")
            ok = False
            continue
        if not filepath.is_file():
            print(f"NOT_FILE: {directory.name} -> {filename}")
            ok = False
            continue

        actual = sha256_file(filepath)
        if actual != expected:
            print(f"MISMATCH: {directory.name} -> {filename}")
            ok = False

    expected_files = list_expected_files(directory)
    missing = expected_files - checksum_files
    extra = checksum_files - expected_files

    if missing:
        print(f"MISSING_IN_CHECKSUM: {directory.name} -> {', '.join(sorted(missing))}")
        ok = False
    if extra:
        print(f"EXTRA_IN_CHECKSUM: {directory.name} -> {', '.join(sorted(extra))}")
        ok = False

    return ok


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Verify SHA256SUMS for all municipalities in an election directory"
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

    municipalities = sorted(path for path in batch_dir.iterdir() if path.is_dir())
    with_checksums = 0
    ok_count = 0
    failed_count = 0

    for muni_dir in municipalities:
        checksum_file = muni_dir / "SHA256SUMS"
        if not checksum_file.exists():
            continue
        with_checksums += 1
        if verify_checksums(muni_dir, checksum_file):
            ok_count += 1
        else:
            failed_count += 1

    print()
    print(f"Checked SHA256SUMS : {with_checksums}")
    print(f"OK                : {ok_count}")
    print(f"FAILED            : {failed_count}")

    return 0 if failed_count == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
