#!/usr/bin/env python3
"""Build an election-level manifest from municipality SHA256SUMS files."""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build an election-level manifest from municipality SHA256SUMS files"
    )
    parser.add_argument(
        "election",
        help="Relative path to the election directory, for example 2026-GR",
    )
    parser.add_argument(
        "--output",
        help="Write to this path instead of <election>/<election>",
    )
    return parser.parse_args()


def load_entries(election_dir: Path) -> list[str]:
    entries: list[str] = []
    checksum_files = sorted(election_dir.glob("*/SHA256SUMS"))

    for checksum_file in checksum_files:
        code = checksum_file.parent.name
        with checksum_file.open(encoding="utf-8") as handle:
            for raw_line in handle:
                line = raw_line.rstrip("\n")
                if not line:
                    continue
                parts = line.split(None, 1)
                if len(parts) != 2:
                    raise ValueError(f"Unexpected checksum line in {checksum_file}: {line!r}")
                digest, filename = parts
                entries.append(f"{digest}  {code}/{filename}")

    return entries


def sort_entries(entries: list[str]) -> list[str]:
    result = subprocess.run(
        ["sort", "-k", "2", "--version-sort"],
        input="".join(f"{entry}\n" for entry in entries),
        capture_output=True,
        text=True,
        check=True,
        env={**os.environ, "LC_ALL": "C"},
    )
    return [line for line in result.stdout.splitlines() if line]


def main() -> int:
    args = parse_args()
    repo_root = Path(__file__).resolve().parents[1]
    election_dir = repo_root / args.election

    if not election_dir.exists():
        print(f"{args.election} directory missing", file=sys.stderr)
        return 1

    output_path = repo_root / args.output if args.output else election_dir / election_dir.name
    entries = sort_entries(load_entries(election_dir))

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text("\n".join(entries) + ("\n" if entries else ""), encoding="utf-8")

    print(f"Wrote {len(entries)} manifest entries to {output_path.relative_to(repo_root)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
