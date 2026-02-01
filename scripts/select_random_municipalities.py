#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from random import SystemRandom
from typing import Iterable, List, Set

TOP5_CODES = {
    "0363": "Amsterdam",
    "0599": "Rotterdam",
    "0518": "Den Haag",
    "0344": "Utrecht",
    "0772": "Eindhoven",
}

CODE_RE = re.compile(r"\d{4}")


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


def iter_candidates(election_dir: Path, todo_codes: Set[str]) -> Iterable[str]:
    for entry in sorted(election_dir.iterdir()):
        if not entry.is_dir():
            continue
        code = entry.name
        if code.startswith("."):
            continue
        if not (CODE_RE.fullmatch(code) or code.startswith("BES-")):
            continue
        if (entry / ".lock").exists():
            continue
        if (entry / "SHA256SUMS").exists():
            continue
        if code in todo_codes:
            continue
        yield code


def select_random(candidates: List[str], count: int) -> List[str]:
    rng = SystemRandom()
    rng.shuffle(candidates)
    if count < len(candidates):
        candidates = candidates[:count]
    for code in candidates:
        if code in TOP5_CODES:
            name = TOP5_CODES[code]
            print(
                f"Top-5 municipality present ({code} {name}); selecting only that one.",
                file=sys.stderr,
            )
            return [code]
    return candidates


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Select random municipalities, excluding .lock, SHA256SUMS, and TODO.md entries."
        )
    )
    parser.add_argument("election_dir", help="Election directory (e.g., 2025-TK)")
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

    todo_codes = load_todo_codes(election_dir / "TODO.md")
    candidates = list(iter_candidates(election_dir, todo_codes))
    if not candidates:
        print("No eligible municipalities found.", file=sys.stderr)
        return 1

    selection = select_random(candidates, args.count)
    for code in selection:
        print(code)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
