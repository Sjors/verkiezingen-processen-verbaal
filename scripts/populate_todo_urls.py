#!/usr/bin/env python3
from __future__ import annotations

import html
import re
import unicodedata
from html.parser import HTMLParser
from pathlib import Path
from typing import Dict, List, Optional
from urllib.request import urlopen

KIESRAAD_URL = (
    "https://www.kiesraad.nl/verkiezingen/tweede-kamer/uitslagen/"
    "uitslagen-per-gemeente-tweede-kamer"
)

ROOT = Path(__file__).resolve().parents[1]
ELECTION_DIR = ROOT / "2025-TK"
KIESRAAD_HTML = ROOT / "tmp" / "kiesraad-tk2025.html"
GEMEENTES_MD = ROOT / "GEMEENTES.md"
TODO_MD = ELECTION_DIR / "TODO.md"

CODE_OVERRIDES = {
    "0518": "Den Haag",
    "0373": "Bergen (NH)",
    "0893": "Bergen (L)",
    "0820": "Nuenen",
}

BES_OVERRIDES = {
    "BES-Bonaire": "Bonaire",
    "BES-Saba": "Saba",
    "BES-SintEustatius": "Sint Eustatius",
}


class LinkParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.links: List[tuple[str, str]] = []
        self._collect = False
        self._text: List[str] = []
        self._href: Optional[str] = None

    def handle_starttag(self, tag: str, attrs: List[tuple[str, Optional[str]]]) -> None:
        if tag != "a":
            return
        attr_map = {k: (v or "") for k, v in attrs}
        class_attr = attr_map.get("class", "")
        if "external" not in class_attr.split():
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


def normalize_name(name: str, *, remove_parens: bool) -> str:
    name = html.unescape(name).lower()
    name = unicodedata.normalize("NFKD", name)
    name = "".join(ch for ch in name if not unicodedata.combining(ch))
    name = name.replace("&", "en")
    if remove_parens:
        name = re.sub(r"\([^)]*\)", "", name)
    name = re.sub(r"[^a-z0-9]+", "", name)
    return name


def load_kiesraad_links() -> tuple[Dict[str, List[str]], Dict[str, List[str]]]:
    if not KIESRAAD_HTML.exists():
        KIESRAAD_HTML.parent.mkdir(parents=True, exist_ok=True)
        with urlopen(KIESRAAD_URL) as response:
            KIESRAAD_HTML.write_bytes(response.read())

    parser = LinkParser()
    parser.feed(KIESRAAD_HTML.read_text(encoding="utf-8"))

    by_full: Dict[str, List[str]] = {}
    by_base: Dict[str, List[str]] = {}

    for name, url in parser.links:
        key_full = normalize_name(name, remove_parens=False)
        key_base = normalize_name(name, remove_parens=True)
        by_full.setdefault(key_full, [])
        if url not in by_full[key_full]:
            by_full[key_full].append(url)
        by_base.setdefault(key_base, [])
        if url not in by_base[key_base]:
            by_base[key_base].append(url)

    return by_full, by_base


def load_gemeentes() -> Dict[str, str]:
    code_to_name: Dict[str, str] = {}
    pattern = re.compile(r"^\|\s*(\d{4})\s*\|\s*([^|]+?)\s*\|\s*(https?://[^|]+)\s*\|\s*$")
    for line in GEMEENTES_MD.read_text(encoding="utf-8").splitlines():
        match = pattern.match(line)
        if match:
            code, name, _url = match.groups()
            code_to_name[code] = name.strip()
    return code_to_name


def load_todo_codes() -> set[str]:
    if not TODO_MD.exists():
        return set()
    todo_codes: set[str] = set()
    for line in TODO_MD.read_text(encoding="utf-8").splitlines():
        if line.startswith("## "):
            parts = line[3:].strip().split(maxsplit=1)
            if parts:
                todo_codes.add(parts[0])
    return todo_codes


def read_name_from_config(path: Path) -> Optional[str]:
    config_path = path / "config.txt"
    if not config_path.exists():
        return None
    for line in config_path.read_text(encoding="utf-8").splitlines():
        if line.startswith("NAME="):
            return line.split("=", 1)[1].strip()
    return None


def read_name_from_readme(path: Path) -> Optional[str]:
    readme_path = path / "README.md"
    if not readme_path.exists():
        return None
    for line in readme_path.read_text(encoding="utf-8").splitlines():
        if line.startswith("# "):
            return line[2:].strip()
    return None


def find_url(name: str, by_full: Dict[str, List[str]], by_base: Dict[str, List[str]]) -> Optional[str]:
    if not name:
        return None
    key_full = normalize_name(name, remove_parens=False)
    key_base = normalize_name(name, remove_parens=True)

    if key_full in by_full and len(by_full[key_full]) == 1:
        return by_full[key_full][0]
    if key_base in by_base and len(by_base[key_base]) == 1:
        return by_base[key_base][0]
    return None


def ensure_url_file(path: Path, url: str) -> bool:
    current = ""
    if path.exists():
        current = path.read_text(encoding="utf-8").strip()
    if current == url:
        return False
    path.write_text(url + "\n", encoding="utf-8")
    return True


def main() -> int:
    if not ELECTION_DIR.exists():
        print(f"Missing election dir: {ELECTION_DIR}")
        return 1

    by_full, by_base = load_kiesraad_links()
    code_to_name = load_gemeentes()
    todo_codes = load_todo_codes()

    updated = 0
    missing_url: List[str] = []
    missing_todo: List[str] = []
    todo_with_sha: List[str] = []

    for entry in sorted(ELECTION_DIR.iterdir()):
        if not entry.is_dir():
            continue
        code = entry.name
        if code.startswith("."):
            continue
        if not (re.fullmatch(r"\d{4}", code) or code.startswith("BES-")):
            continue

        sha_exists = (entry / "SHA256SUMS").exists()
        todo_path = entry / ".todo"

        if code in BES_OVERRIDES:
            name = BES_OVERRIDES[code]
            lookup_name = name
        else:
            name = code_to_name.get(code) or read_name_from_config(entry) or read_name_from_readme(entry)
            lookup_name = CODE_OVERRIDES.get(code, name)

        url = None
        if lookup_name:
            url = find_url(lookup_name, by_full, by_base)
        if not url and name and lookup_name != name:
            url = find_url(name, by_full, by_base)

        needs_todo = not sha_exists

        if url:
            if needs_todo or todo_path.exists():
                if ensure_url_file(todo_path, url):
                    updated += 1
        else:
            missing_url.append(code)

        if needs_todo and not todo_path.exists():
            missing_todo.append(code)

        if code in todo_codes and sha_exists:
            todo_with_sha.append(code)

    if missing_todo:
        print("Missing .todo (no SHA256SUMS):")
        print("  " + ", ".join(missing_todo))

    if missing_url:
        print("Missing URL from Kiesraad list:")
        print("  " + ", ".join(missing_url))

    if todo_with_sha:
        print("TODO.md entries that already have SHA256SUMS:")
        print("  " + ", ".join(todo_with_sha))

    print(f"Updated .todo files: {updated}")
    print("Sanity check complete.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
