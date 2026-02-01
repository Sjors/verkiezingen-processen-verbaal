#!/usr/bin/env python3
"""Generate TODO.md headings with Kiesraad result URLs.

Reads GEMEENTES.md for codes/names and matches them to the
Kiesraad 'uitslagen per gemeente' page. Writes {election}/TODO.md
and supports skipping specific municipality codes.
"""
from __future__ import annotations

import argparse
from html import unescape
from html.parser import HTMLParser
from pathlib import Path
from typing import Dict, Iterable, List, Tuple
from urllib.request import Request, urlopen

class KiesraadLinkParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.links: Dict[str, str] = {}
        self._in_a = False
        self._href: str | None = None
        self._is_muni = False
        self._text_parts: List[str] = []

    def handle_starttag(self, tag: str, attrs: List[Tuple[str, str | None]]) -> None:
        if tag != "a":
            return
        attr = dict(attrs)
        self._href = attr.get("href")
        aria = attr.get("aria-label", "") or ""
        self._is_muni = aria.endswith("(opent externe website)")
        self._in_a = True
        self._text_parts = []

    def handle_data(self, data: str) -> None:
        if self._in_a:
            self._text_parts.append(data)

    def handle_endtag(self, tag: str) -> None:
        if tag != "a" or not self._in_a:
            return
        name = unescape("".join(self._text_parts)).strip()
        if self._is_muni and name and self._href:
            self.links.setdefault(name, self._href)
        self._in_a = False
        self._href = None
        self._is_muni = False
        self._text_parts = []


def fetch_kiesraad_links(url: str) -> Dict[str, str]:
    req = Request(url, headers={"User-Agent": "Mozilla/5.0"})
    with urlopen(req) as resp:
        html = resp.read().decode("utf-8")
    parser = KiesraadLinkParser()
    parser.feed(html)
    return parser.links


def parse_gemeentes(path: Path) -> List[Tuple[str, str]]:
    entries: List[Tuple[str, str]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("## Aliases"):
            break
        if not line.startswith("|"):
            continue
        parts = [p.strip() for p in line.strip().strip("|").split("|")]
        if len(parts) < 2:
            continue
        code, name = parts[0], parts[1]
        if code.lower() == "code" or set(code) == {"-"}:
            continue
        entries.append((code, name))
    return entries


def normalize_name(name: str) -> str:
    overrides = {
        "'s-Gravenhage": "Den Haag",
        "Bergen (Limburg)": "Bergen (L)",
        "Bergen (Noord-Holland)": "Bergen (NH)",
        "Nuenen, Gerwen en Nederwetten": "Nuenen",
    }
    if name in overrides:
        return overrides[name]
    if " (" in name:
        return name.split(" (", 1)[0]
    return name


def build_todo(
    entries: Iterable[Tuple[str, str]],
    links: Dict[str, str],
    skip_codes: Iterable[str],
) -> Tuple[str, List[str]]:
    skip = set(skip_codes)
    missing: List[str] = []
    lines: List[str] = []

    for code, name in entries:
        if code in skip:
            continue
        key = normalize_name(name)
        url = links.get(key, "")
        if not url:
            missing.append(f"{code} {name} -> {key}")
            url = "MISSING"
        lines.append(f"## {code} {name}")
        lines.append(f"- URL: {url}")
        lines.append("")

    content = "\n".join(lines).rstrip() + "\n"
    return content, missing


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate TODO.md with Kiesraad URLs.")
    parser.add_argument("--kiesraad-url", required=True)
    parser.add_argument("--gemeentes", default="GEMEENTES.md")
    parser.add_argument("--election", required=True, help="Election folder (e.g. 2025-TK)")
    parser.add_argument("--out", help="Output path (defaults to {election}/TODO.md)")
    parser.add_argument(
        "--skip",
        action="append",
        default=[],
        help="Codes to skip (can be provided multiple times)",
    )
    args = parser.parse_args()

    out_path = args.out or f"{args.election}/TODO.md"

    links = fetch_kiesraad_links(args.kiesraad_url)
    entries = parse_gemeentes(Path(args.gemeentes))
    content, missing = build_todo(entries, links, args.skip)

    Path(out_path).write_text(content, encoding="utf-8")

    if missing:
        print("Missing URLs:")
        for item in missing:
            print(f"- {item}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
