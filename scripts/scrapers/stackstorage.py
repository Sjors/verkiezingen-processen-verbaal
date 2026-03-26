#!/usr/bin/env python3
"""
Download files from StackStorage public-share URLs.

Usage:
    ./scripts/fetch-stackstorage.py <share_url>... <output_dir>

Example:
    ./scripts/fetch-stackstorage.py \
      https://technischbeheerassen.stackstorage.com/s/TrAgEUIPvzmCfCwp \
      https://technischbeheerassen.stackstorage.com/s/I32vZL8Bs2a2JdNM \
      2026-GR/0106

The script:
1. Logs into each public share via the StackStorage API
2. Lists the files in the share (or treats the share itself as a file)
3. Downloads all files to the output directory
4. Writes a sorted SHA256SUMS file

Notes:
- Duplicate filenames across shares are downloaded only once, keeping the first
  occurrence.
- A URL list file is not generated automatically because the municipality code
  and name are not implied by the StackStorage share URLs alone.
"""

import argparse
import hashlib
import json
import os
import sys
import urllib.parse
import urllib.request
from pathlib import Path


USER_AGENT = os.environ.get(
    "PV_USER_AGENT",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/605.1.15 "
    "(KHTML, like Gecko) Version/17.0 Safari/605.1.15",
)


def parse_share_url(url: str) -> tuple[str, str]:
    """Return (base_url, share_id) for a StackStorage public-share URL."""
    parsed = urllib.parse.urlsplit(url.rstrip("/"))
    parts = [part for part in parsed.path.split("/") if part]
    if len(parts) != 2 or parts[0] != "s":
        raise ValueError(f"Unsupported StackStorage share URL: {url}")
    base_url = f"{parsed.scheme}://{parsed.netloc}"
    return base_url, parts[1]


def api_request(url: str, headers: dict[str, str], method: str = "GET") -> tuple[bytes, dict[str, str]]:
    """Perform an API request and return (body, response_headers)."""
    req = urllib.request.Request(url, headers=headers, method=method)
    with urllib.request.urlopen(req, timeout=60) as response:
        return response.read(), dict(response.headers.items())


def login_share(base_url: str, share_id: str) -> tuple[str, str]:
    """Authenticate against a public share and return (share_token, csrf_token)."""
    _, headers = api_request(
        f"{base_url}/api/v2/share/{share_id}",
        headers={"User-Agent": USER_AGENT},
        method="POST",
    )
    share_token = headers.get("x-sharetoken", "")
    csrf_token = headers.get("x-csrf-token", "")
    if not share_token or not csrf_token:
        raise RuntimeError(f"Missing StackStorage tokens for share {share_id}")
    return share_token, csrf_token


def share_headers(share_token: str, csrf_token: str) -> dict[str, str]:
    """Build headers for authenticated StackStorage public-share API calls."""
    return {
        "User-Agent": USER_AGENT,
        "x-sharetoken": share_token,
        "x-csrf-token": csrf_token,
    }


def get_share_meta(base_url: str, share_id: str, headers: dict[str, str]) -> dict:
    """Fetch public-share metadata."""
    body, _ = api_request(f"{base_url}/api/v2/share/{share_id}", headers=headers)
    return json.loads(body)


def list_share_nodes(base_url: str, share_id: str, root_node_id: int, headers: dict[str, str]) -> list[dict]:
    """List files in a public-share directory."""
    params = urllib.parse.urlencode({"parentID": root_node_id, "pageNumber": 1})
    body, _ = api_request(f"{base_url}/api/v2/share/{share_id}/nodes?{params}", headers=headers)
    return json.loads(body)["nodes"]


def download_node(base_url: str, share_id: str, node: dict, csrf_token: str, headers: dict[str, str], output_dir: Path) -> str:
    """Download a single node and return the filename written."""
    filename = node["name"]
    encoded_name = urllib.parse.quote(filename, safe="")
    query = urllib.parse.urlencode({"noContentDisposition": 1, "CSRF-Token": csrf_token})
    url = f"{base_url}/api/v2/share/{share_id}/files/{node['id']}/download/{encoded_name}?{query}"
    destination = output_dir / filename

    if destination.exists() and destination.stat().st_size > 0:
        print(f"Skipping (exists): {filename}")
        return filename

    print(f"Downloading: {filename}")
    body, _ = api_request(url, headers=headers)
    destination.write_bytes(body)
    return filename


def sha256_file(path: Path) -> str:
    """Calculate the SHA256 for a file."""
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def generate_checksums(output_dir: Path):
    """Generate SHA256SUMS for downloaded municipality files."""
    ignored = {".todo", ".lock", "config.txt", "README.md", "SHA256SUMS"}
    files = sorted(
        [path for path in output_dir.iterdir() if path.is_file() and path.name not in ignored],
        key=lambda path: path.name,
    )

    lines = [f"{sha256_file(path)}  {path.name}" for path in files]
    (output_dir / "SHA256SUMS").write_text("\n".join(lines) + ("\n" if lines else ""))
    print(f"Generated SHA256SUMS with {len(lines)} entries")


def main():
    parser = argparse.ArgumentParser(description="Download files from StackStorage public-share URLs")
    parser.add_argument("args", nargs="+", help="One or more share URLs followed by the output directory")
    parsed = parser.parse_args()

    if len(parsed.args) < 2:
        parser.error("Provide at least one share URL and an output directory")

    share_urls = parsed.args[:-1]
    output_dir = Path(parsed.args[-1])
    output_dir.mkdir(parents=True, exist_ok=True)

    seen_names: set[str] = set()
    downloaded = 0

    for share_url in share_urls:
        base_url, share_id = parse_share_url(share_url)
        share_token, csrf_token = login_share(base_url, share_id)
        headers = share_headers(share_token, csrf_token)
        meta = get_share_meta(base_url, share_id, headers)

        if meta.get("dir"):
            nodes = list_share_nodes(base_url, share_id, meta["id"], headers)
        else:
            nodes = [meta]

        print(f"Share {share_id}: {meta['name']} ({len(nodes)} item(s))")
        for node in nodes:
            if node["name"] in seen_names:
                print(f"Skipping (duplicate name): {node['name']}")
                continue
            seen_names.add(node["name"])
            download_node(base_url, share_id, node, csrf_token, headers, output_dir)
            downloaded += 1

    generate_checksums(output_dir)
    print(f"Done! Downloaded {downloaded} unique file(s)")


if __name__ == "__main__":
    main()
