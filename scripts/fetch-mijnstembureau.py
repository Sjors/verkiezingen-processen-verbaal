#!/usr/bin/env python3
"""
Download processen-verbaal from Mijn Stembureau API.

Usage:
    ./scripts/fetch-mijnstembureau.py <base_url> <output_dir> [--election <name>]

Examples:
    ./scripts/fetch-mijnstembureau.py https://mijnstembureau-losser.nl 2025-TK/0168
    ./scripts/fetch-mijnstembureau.py https://mijnstembureau-losser.nl 2025-TK/0168 --election "Tweede Kamerverkiezing 2025"

The script will:
1. Fetch all available elections from the API
2. Select the most recent election (or one matching --election)
3. Download all proces-verbaal PDFs to the output directory
4. Generate SHA256SUMS file
"""

import argparse
import hashlib
import json
import os
import sys
from pathlib import Path
from urllib.parse import quote, urlparse

import requests


def filter_filename(filename: str) -> str:
    """Sanitize filename by removing unsafe characters."""
    keepcharacters = (" ", ".", "_", "-")
    result = "".join(c for c in filename if c.isalnum() or c in keepcharacters).rstrip()
    return result


def get_api_url(base_url: str) -> str:
    """Construct the API URL from the base URL."""
    parsed = urlparse(base_url.rstrip("/"))
    return f"{parsed.scheme}://{parsed.netloc}/api/prime/uitslagen"


def fetch_elections(api_url: str, referer: str) -> list:
    """Fetch all elections from the API."""
    headers = {
        'Accept': 'application/json',
        'X-Requested-With': 'XMLHttpRequest',
        'Referer': referer,
        'User-Agent': 'Mozilla/5.0'
    }

    session = requests.Session()
    session.headers.update(headers)

    resp = session.get(api_url)
    resp.raise_for_status()
    return resp.json(), session


def select_election(elections: list, election_filter: str = None) -> dict:
    """Select an election from the list."""
    # Filter elections that have pvKeys
    with_pvs = [e for e in elections if e.get('pvKeys') or e.get('pv_keys')]

    if not with_pvs:
        print("No elections with processen-verbaal found")
        sys.exit(1)

    if election_filter:
        # Find election matching filter
        matches = [e for e in with_pvs if election_filter.lower() in e['verkiezingNaam'].lower()]
        if not matches:
            print(f"No election matching '{election_filter}' found")
            print("Available elections:")
            for e in with_pvs:
                print(f"  - {e['verkiezingNaam']}")
            sys.exit(1)
        return matches[0]

    # Return the first one (usually most recent)
    # Sort by date to be sure
    with_pvs.sort(key=lambda e: e.get('electionDate', ''), reverse=True)
    return with_pvs[0]


def download_pvs(election: dict, api_url: str, session: requests.Session, output_dir: Path) -> list:
    """Download all proces-verbaal PDFs for an election."""
    uitslag_id = election['uitslagId']
    pv_keys = election.get('pvKeys') or election.get('pv_keys', [])

    print(f"Found {len(pv_keys)} PV files for {election['verkiezingNaam']}")

    downloaded = []
    for item in pv_keys:
        _id = item['_id']
        # Handle both old and new API formats
        aws_key = item.get('awsKey') or item.get('pvAwsKey', '')

        if not aws_key:
            print(f"  Warning: No path for item {_id}")
            continue

        # Sanitize filename
        filename = filter_filename(Path(aws_key).name)
        filepath = output_dir / filename

        if filepath.exists():
            print(f'Skipping (exists): {filename}')
            downloaded.append(filename)
            continue

        url = f'{api_url}/view-pv/{quote(uitslag_id)}/{quote(_id)}'
        print(f'Downloading: {filename}')

        resp = session.get(url)
        if resp.ok:
            filepath.write_bytes(resp.content)
            downloaded.append(filename)
        else:
            print(f'  Error: {resp.status_code}')

    return downloaded


def generate_checksums(output_dir: Path, files: list):
    """Generate SHA256SUMS file."""
    checksums_file = output_dir / 'SHA256SUMS'

    lines = []
    for filename in sorted(files):
        filepath = output_dir / filename
        if filepath.exists():
            sha256 = hashlib.sha256(filepath.read_bytes()).hexdigest()
            lines.append(f"{sha256}  {filename}")

    checksums_file.write_text('\n'.join(lines) + '\n')
    print(f"Generated SHA256SUMS with {len(lines)} entries")


def main():
    parser = argparse.ArgumentParser(description="Download processen-verbaal from Mijn Stembureau API")
    parser.add_argument('base_url', help="Base URL of the mijnstembureau site (e.g., https://mijnstembureau-losser.nl)")
    parser.add_argument('output_dir', help="Directory to save PDFs to (e.g., 2025-TK/0168)")
    parser.add_argument('--election', help="Filter by election name (e.g., '2025' or 'Tweede Kamer')")

    args = parser.parse_args()

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    api_url = get_api_url(args.base_url)
    print(f"Fetching from {api_url}")

    elections, session = fetch_elections(api_url, args.base_url)
    election = select_election(elections, args.election)

    downloaded = download_pvs(election, api_url, session, output_dir)

    if downloaded:
        generate_checksums(output_dir, downloaded)

    print("Done!")


if __name__ == '__main__':
    main()
