#!/usr/bin/env python3
"""Select random municipalities that still need processing.

Reads the full municipality list from progress/{election}-municipalities.txt
(produced by scrape_kiesraad_municipalities.py).  Excludes municipalities that
already have a directory with SHA256SUMS, a .lock, or an entry in TODO.md.

Usage:
    ./scripts/select_random_municipalities.py 2026-GR 6
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path
from random import SystemRandom
from typing import List, Set, Tuple

ROOT = Path(__file__).resolve().parents[1]


def load_todo_codes(todo_path: Path) -> Set[str]:
    if not todo_path.exists():
        return set()
    codes: Set[str] = set()
    for line in todo_path.read_text(encoding="utf-8").splitlines():
        if line.startswith("## "):
            parts = line[3:].strip().split(maxsplit=1)
            if parts:
                codes.add(parts[0])
    return codes


def load_municipalities(list_path: Path) -> List[Tuple[str, str, str]]:
    """Return list of (code, name, url) from the municipalities file."""
    entries: List[Tuple[str, str, str]] = []
    for line in list_path.read_text(encoding="utf-8").splitlines():
        parts = line.split("\t", 2)
        if len(parts) == 3:
            entries.append((parts[0], parts[1], parts[2]))
    return entries


def iter_candidates(
    election_dir: Path,
    municipalities: List[Tuple[str, str, str]],
    todo_codes: Set[str],
) -> List[Tuple[str, str, str]]:
    """Filter to municipalities not yet processed, locked, or in TODO."""
    result: List[Tuple[str, str, str]] = []
    for code, name, url in municipalities:
        mdir = election_dir / code
        if (mdir / ".lock").exists():
            continue
        if (mdir / "SHA256SUMS").exists():
            continue
        if code in todo_codes:
            continue
        result.append((code, name, url))
    return result


def select_random(candidates: List[Tuple[str, str, str]], count: int) -> List[Tuple[str, str, str]]:
    rng = SystemRandom()
    rng.shuffle(candidates)
    if count < len(candidates):
        candidates = candidates[:count]
    return candidates


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Select random municipalities from the Kiesraad list, "
            "excluding finished, locked, and TODO entries."
        )
    )
    parser.add_argument("election_dir", help="Election directory (e.g., 2026-GR)")
    parser.add_argument("count", type=int, help="Number of municipalities to select")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    election_dir = Path(args.election_dir)
    if not election_dir.is_dir():
        print(f"Not a directory: {election_dir}", file=sys.stderr)
        return 1
    if args.count <= 0:
        print("Count must be a positive integer", file=sys.stderr)
        return 1

    list_path = ROOT / "progress" / f"{election_dir.name}-municipalities.txt"
    if not list_path.exists():
        print(
            f"Municipality list not found: {list_path}\n"
            f"Run: ./scripts/scrape_kiesraad_municipalities.py {election_dir.name} <kiesraad_url>",
            file=sys.stderr,
        )
        return 1

    municipalities = load_municipalities(list_path)
    todo_codes = load_todo_codes(election_dir / "TODO.md")
    candidates = iter_candidates(election_dir, municipalities, todo_codes)

    if not candidates:
        print("No eligible municipalities found.", file=sys.stderr)
        return 1

    selection = select_random(candidates, args.count)
    for code, name, url in selection:
        print(f"{code}\t{name}\t{url}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
