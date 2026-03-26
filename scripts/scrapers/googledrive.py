#!/usr/bin/env python3
"""
Download files from public Google Drive folders.

Usage:
    ./scripts/fetch-googledrive-folder.py <folder_url_or_id> <output_dir>
    ./scripts/fetch-googledrive-folder.py <label>=<folder_url_or_id>... <output_dir>

Examples:
    ./scripts/fetch-googledrive-folder.py \
      https://drive.google.com/drive/folders/1_G0uhO3ssfArx4XU16gvbq6BK3IH0GH3 \
      tmp/drive-folder

    ./scripts/fetch-googledrive-folder.py \
      per-kandidaat=https://drive.google.com/drive/folders/1_G0uhO3ssfArx4XU16gvbq6BK3IH0GH3 \
      op-partijniveau=https://drive.google.com/drive/folders/1GG81l9a1O3nes35mSGnaGRls4BjeEeU_ \
      hertellingen=https://drive.google.com/drive/folders/1sXceJd5KREdM8pWR17bGjFsos1giwnK4 \
      2026-GR/0957

The script:
1. Fetches the folder's public embedded listing
2. Extracts file IDs and names from the HTML
3. Downloads each file via Google's direct download endpoint
4. Writes a sorted SHA256SUMS file for the downloaded tree

Notes:
- Public folders only; authentication is not supported.
- Duplicate relative paths are skipped.
"""

import argparse
import hashlib
import html
import os
import re
import sys
import urllib.parse
import urllib.request
from pathlib import Path


USER_AGENT = os.environ.get(
    "PV_USER_AGENT",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/605.1.15 "
    "(KHTML, like Gecko) Version/17.0 Safari/605.1.15",
)

ENTRY_RE = re.compile(
    r'href="https://drive\.google\.com/file/d/([^/]+)/view\?usp=drive_web"'
    r'.*?<div class="flip-entry-title">([^<]+)</div>',
    re.DOTALL,
)


def parse_folder_id(value: str) -> str:
    """Accept a folder URL or bare folder ID."""
    value = value.strip()
    if "://" not in value:
        return value

    parsed = urllib.parse.urlsplit(value)
    parts = [part for part in parsed.path.split("/") if part]
    if len(parts) >= 3 and parts[0] == "drive" and parts[1] == "folders":
        return parts[2]
    raise ValueError(f"Unsupported Google Drive folder URL: {value}")


def fetch_bytes(url: str) -> bytes:
    """Download bytes with a browser-like user agent."""
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=60) as response:
        return response.read()


def list_public_folder(folder_id: str) -> list[tuple[str, str]]:
    """Return [(file_id, filename), ...] from a public Google Drive folder."""
    url = f"https://drive.google.com/embeddedfolderview?id={folder_id}#list"
    body = fetch_bytes(url).decode("utf-8", "replace")
    entries = [(file_id, html.unescape(name)) for file_id, name in ENTRY_RE.findall(body)]
    if not entries:
        raise RuntimeError(f"No downloadable entries found in folder {folder_id}")
    return entries


def sha256_file(path: Path) -> str:
    """Calculate a file SHA256."""
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(8192), b""):
            digest.update(chunk)
    return digest.hexdigest()


def generate_checksums(output_dir: Path):
    """Generate SHA256SUMS for the full downloaded tree."""
    ignored = {".todo", ".lock", "config.txt", "README.md", "SHA256SUMS"}
    files = sorted(
        [
            path
            for path in output_dir.rglob("*")
            if path.is_file() and path.name not in ignored
        ],
        key=lambda path: path.relative_to(output_dir).as_posix(),
    )
    lines = [
        f"{sha256_file(path)}  {path.relative_to(output_dir).as_posix()}"
        for path in files
    ]
    (output_dir / "SHA256SUMS").write_text("\n".join(lines) + ("\n" if lines else ""))
    print(f"Generated SHA256SUMS with {len(lines)} entries")


def parse_source(value: str) -> tuple[str | None, str]:
    """Parse either folder or label=folder."""
    if "=" in value:
        label, folder = value.split("=", 1)
        label = label.strip()
        if not label:
            raise ValueError(f"Invalid labeled folder argument: {value}")
        return label, parse_folder_id(folder)
    return None, parse_folder_id(value)


def main():
    parser = argparse.ArgumentParser(description="Download files from public Google Drive folders")
    parser.add_argument("args", nargs="+", help="One or more folder IDs/URLs followed by the output directory")
    parsed = parser.parse_args()

    if len(parsed.args) < 2:
        parser.error("Provide at least one folder ID/URL and an output directory")

    raw_sources = parsed.args[:-1]
    output_dir = Path(parsed.args[-1])
    output_dir.mkdir(parents=True, exist_ok=True)

    labeled = any("=" in arg for arg in raw_sources)
    if labeled and not all("=" in arg for arg in raw_sources):
        parser.error("Either label all Google Drive folder arguments or none of them")

    seen_relpaths: set[str] = set()
    downloaded = 0

    for raw_source in raw_sources:
        label, folder_id = parse_source(raw_source)
        target_dir = output_dir / label if label else output_dir
        target_dir.mkdir(parents=True, exist_ok=True)

        entries = list_public_folder(folder_id)
        print(f"Folder {folder_id}: {len(entries)} item(s)")
        for file_id, filename in entries:
            relpath = (Path(label) / filename).as_posix() if label else filename
            if relpath in seen_relpaths:
                print(f"Skipping (duplicate path): {relpath}")
                continue
            seen_relpaths.add(relpath)

            destination = target_dir / filename
            if destination.exists() and destination.stat().st_size > 0:
                print(f"Skipping (exists): {relpath}")
                continue

            download_url = "https://drive.google.com/uc?export=download&id=" + urllib.parse.quote(file_id)
            print(f"Downloading: {relpath}")
            destination.write_bytes(fetch_bytes(download_url))
            downloaded += 1

    generate_checksums(output_dir)
    print(f"Done! Downloaded {downloaded} file(s)")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(130)
