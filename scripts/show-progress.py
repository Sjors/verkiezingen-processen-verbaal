#!/usr/bin/env python3
"""Report the status of the chosen election batch."""

from pathlib import Path
import argparse
import sys


def count_todo_entries(todo_path: Path) -> int:
    if not todo_path.exists():
        return 0
    entries = 0
    with todo_path.open() as handle:
        for line in handle:
            if line.startswith("## "):
                entries += 1
    return entries


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Show progress for a gemeente election batch")
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
    municipalities = [path for path in batch_dir.iterdir() if path.is_dir()]
    finished = sum(1 for path in municipalities if (path / "SHA256SUMS").exists())
    locked = sum(1 for path in municipalities if (path / ".lock").exists())
    todo_entries = count_todo_entries(batch_dir / "TODO.md")
    remaining = max(len(municipalities) - finished, 0)

    rows = [
        (f"{args.election} municipalities", len(municipalities)),
        ("Finished (SHA256SUMS)", finished),
        ("In-progress (.lock)", locked),
        ("TODO issues (TODO.md entries)", todo_entries),
        ("Remaining without SHA256", remaining),
    ]

    label_width = max(len(label) for label, _ in rows)
    for label, value in rows:
        print(f"{label:<{label_width}} : {value:>5}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
