#!/usr/bin/env python3

from __future__ import annotations

import argparse
from pathlib import Path


REMOVABLES = {
    "lock": ".lock",
    "todo": ".todo",
    "config": "config.txt",
    "sha256": "SHA256SUMS",
    "readme": "README.md",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Remove generated municipality files without using prompt-heavy shell globs."
    )
    parser.add_argument(
        "municipality_dir",
        help="Municipality directory, e.g. 2026-GR/1904",
    )
    parser.add_argument(
        "--remove",
        action="append",
        choices=["lock", "todo", "config", "url-list", "pdfs", "sha256", "readme"],
        default=[],
        help="Specific artifact to remove. May be passed multiple times.",
    )
    parser.add_argument(
        "--reset-generated",
        action="store_true",
        help="Remove generated fetch artifacts: .todo, config.txt, URL list, PDFs, SHA256SUMS.",
    )
    return parser.parse_args()


def remove_file(path: Path) -> int:
    if not path.exists():
        print(f"Missing: {path}")
        return 0
    path.unlink()
    print(f"Removed: {path}")
    return 1


def main() -> int:
    args = parse_args()
    municipality_dir = Path(args.municipality_dir)
    if not municipality_dir.is_dir():
        raise SystemExit(f"Not a directory: {municipality_dir}")

    requested = set(args.remove)
    if args.reset_generated:
        requested.update({"todo", "config", "url-list", "pdfs", "sha256"})

    if not requested:
        raise SystemExit("Nothing to do. Use --remove and/or --reset-generated.")

    removed = 0

    for key in sorted(requested):
        if key in REMOVABLES:
            removed += remove_file(municipality_dir / REMOVABLES[key])
        elif key == "url-list":
            txt_files = sorted(municipality_dir.glob("*.txt"))
            if not txt_files:
                print(f"Missing: {municipality_dir}/*.txt")
            for file_path in txt_files:
                removed += remove_file(file_path)
        elif key == "pdfs":
            pdf_files = sorted(municipality_dir.glob("*.pdf"))
            if not pdf_files:
                print(f"Missing: {municipality_dir}/*.pdf")
            for file_path in pdf_files:
                removed += remove_file(file_path)

    print(f"Done. Removed {removed} file(s).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())