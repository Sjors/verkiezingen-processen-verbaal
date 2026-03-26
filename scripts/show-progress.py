#!/usr/bin/env python3
"""Report the status of the chosen election batch."""

from pathlib import Path
import argparse
import subprocess
import sys


def todo_municipalities(todo_path: Path) -> set[str]:
    if not todo_path.exists():
        return set()
    municipalities = set()
    with todo_path.open() as handle:
        for line in handle:
            if line.startswith("## "):
                parts = line.strip().split(maxsplit=2)
                if len(parts) >= 2:
                    municipalities.add(parts[1])
    return municipalities


def tracked_files(repo_root: Path, scope: str) -> set[str]:
    result = subprocess.run(
        ["git", "ls-files", "--", scope],
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        return set()
    return {line.strip() for line in result.stdout.splitlines() if line.strip()}


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

    # Load full municipality list from progress/ if available
    list_path = repo_root / "progress" / f"{args.election}-municipalities.txt"
    if list_path.exists():
        all_codes = set()
        for line in list_path.read_text(encoding="utf-8").splitlines():
            parts = line.split("\t", 1)
            if parts and parts[0] != "????":
                all_codes.add(parts[0])
        total = len(all_codes)
    else:
        all_codes = None
        total = sum(1 for p in batch_dir.iterdir() if p.is_dir())

    municipalities = [path for path in batch_dir.iterdir() if path.is_dir()]
    tracked = tracked_files(repo_root, args.election)
    todo_codes = todo_municipalities(batch_dir / "TODO.md")
    finished = 0
    not_counted_finished = 0
    for path in municipalities:
        sha256 = path / "SHA256SUMS"
        if not sha256.exists():
            continue
        tracked_sha256 = f"{args.election}/{path.name}/SHA256SUMS"
        if path.name in todo_codes:
            not_counted_finished += 1
        elif tracked_sha256 in tracked:
            finished += 1
        else:
            not_counted_finished += 1
    locked = sum(1 for path in municipalities if (path / ".lock").exists())
    todo_entries = len(todo_codes)
    remaining = max(total - finished, 0)

    rows = [
        (f"{args.election} municipalities", total),
        ("Finished (tracked SHA256SUMS)", finished),
        ("Not counted as finished (local-only/TODO)", not_counted_finished),
        ("In-progress (.lock)", locked),
        ("TODO issues (TODO.md entries)", todo_entries),
        ("Remaining without tracked SHA256", remaining),
    ]

    label_width = max(len(label) for label, _ in rows)
    for label, value in rows:
        print(f"{label:<{label_width}} : {value:>5}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
