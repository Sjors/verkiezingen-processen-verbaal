#!/usr/bin/env python3
"""Scrape the Kiesraad uitslagen-per-gemeente page for an election and produce
a municipalities list in progress/{election}-municipalities.txt.

Each line: {code}\t{name}\t{kiesraad_url}

The CBS code is looked up from GEMEENTES.md.  Municipalities that appear on the
Kiesraad page but are missing from GEMEENTES.md are printed as warnings and
written with code "????" so the user can add them.

Usage:
    ./scripts/scrape_kiesraad_municipalities.py 2026-GR <kiesraad_url>
"""
from __future__ import annotations

import html as html_mod
import re
import sys
import unicodedata
from html.parser import HTMLParser
from pathlib import Path
from typing import Dict, List, Optional
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
GEMEENTES_MD = ROOT / "GEMEENTES.md"

# Kiesraad wraps each municipality link in <a class="... external ...">
class LinkParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.links: list[tuple[str, str]] = []
        self._collect = False
        self._text: list[str] = []
        self._href: Optional[str] = None

    def handle_starttag(self, tag: str, attrs: list[tuple[str, Optional[str]]]) -> None:
        if tag != "a":
            return
        attr_map = {k: (v or "") for k, v in attrs}
        rel_attr = attr_map.get("rel", "")
        if "external" not in rel_attr.split():
            return
        href = attr_map.get("href")
        if not href:
            return
        self._collect = True
        self._text = []
        self._href = href

    def handle_data(self, data: str) -> None:
        if self._collect:
            self._text.append(data)

    def handle_endtag(self, tag: str) -> None:
        if tag != "a" or not self._collect:
            return
        text = "".join(self._text).strip()
        href = self._href
        if text and href:
            self.links.append((text, href))
        self._collect = False
        self._text = []
        self._href = None


def normalize(name: str) -> str:
    """Lowercase, strip accents, strip parenthetical, only alnum."""
    name = html_mod.unescape(name).lower()
    name = unicodedata.normalize("NFKD", name)
    name = "".join(ch for ch in name if not unicodedata.combining(ch))
    name = name.replace("&", "en")
    name = re.sub(r"\([^)]*\)", "", name)
    name = re.sub(r"[^a-z0-9]+", "", name)
    return name


NAME_OVERRIDES = {
    "Den Haag": "'s-Gravenhage",
    "Den Bosch": "'s-Hertogenbosch",
    "Nuenen": "Nuenen, Gerwen en Nederwetten",
}


def load_gemeentes() -> tuple[Dict[str, str], Dict[str, str]]:
    """Return (code_to_name, norm_to_code) from GEMEENTES.md."""
    code_to_name: Dict[str, str] = {}
    norm_to_code: Dict[str, str] = {}
    pattern = re.compile(
        r"^\|\s*(\d{4})\s*\|\s*([^|]+?)\s*\|\s*https?://[^|]+\s*\|\s*$"
    )
    for line in GEMEENTES_MD.read_text(encoding="utf-8").splitlines():
        m = pattern.match(line)
        if m:
            code, name = m.group(1), m.group(2).strip()
            code_to_name[code] = name
            norm_to_code[normalize(name)] = code
    return code_to_name, norm_to_code


def main() -> int:
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <election> <kiesraad_url>")
        return 1

    election = sys.argv[1]
    kiesraad_url = sys.argv[2]
    election_dir = ROOT / election
    if not election_dir.is_dir():
        print(f"Election dir not found: {election_dir}")
        return 1

    out_dir = ROOT / "progress"
    out_dir.mkdir(exist_ok=True)
    out_file = out_dir / f"{election}-municipalities.txt"

    # Download Kiesraad page
    cache = ROOT / "tmp" / f"kiesraad-{election.lower()}.html"
    cache.parent.mkdir(parents=True, exist_ok=True)
    if not cache.exists():
        print(f"Downloading {kiesraad_url} ...")
        req = Request(kiesraad_url, headers={
            "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) "
                          "AppleWebKit/605.1.15 (KHTML, like Gecko) "
                          "Version/17.0 Safari/605.1.15"
        })
        with urlopen(req) as resp:
            cache.write_bytes(resp.read())
    else:
        print(f"Using cached {cache}")

    # Parse links
    parser = LinkParser()
    parser.feed(cache.read_text(encoding="utf-8"))

    # Load GEMEENTES.md
    _code_to_name, norm_to_code = load_gemeentes()

    # Build list
    entries: list[tuple[str, str, str]] = []
    missing: list[tuple[str, str]] = []
    seen_codes: set[str] = set()

    for name, url in parser.links:
        norm = normalize(name)
        code = norm_to_code.get(norm)
        if not code:
            # Try with name overrides
            for alias, official in NAME_OVERRIDES.items():
                if normalize(alias) == norm:
                    code = norm_to_code.get(normalize(official))
                    break
        if code:
            if code not in seen_codes:
                entries.append((code, name.strip(), url.strip()))
                seen_codes.add(code)
        else:
            missing.append((name.strip(), url.strip()))

    # Sort by code
    entries.sort(key=lambda e: e[0])

    # Write output
    with out_file.open("w", encoding="utf-8") as f:
        for code, name, url in entries:
            f.write(f"{code}\t{name}\t{url}\n")
        for name, url in missing:
            f.write(f"????\t{name}\t{url}\n")

    print(f"Wrote {len(entries)} municipalities to {out_file}")
    if missing:
        print(f"WARNING: {len(missing)} municipalities not found in GEMEENTES.md:")
        for name, url in missing:
            print(f"  {name}: {url}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
