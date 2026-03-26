#!/usr/bin/env python3
"""
Fetch Processen-Verbaal (vote counting documents) for a municipality.

Usage:
    ./scripts/fetch-pv.py <municipality_dir>

The municipality directory must contain a config.txt file with:
    URL=<page containing PDF links>
    REGEX=<regex pattern to match PDF links>
    PREFIX=<prefix to add to relative URLs>

Example config.txt:
    URL=https://www.amersfoort.nl/processen-verbaal
    REGEX=processen-verbaal.*\.pdf
    PREFIX=https://www.amersfoort.nl

The script will:
1. Generate a URL list file ({code} {name}.txt) if missing
2. Download PDFs from the list (skipping existing files)
3. Generate SHA256SUMS or verify existing checksums
"""

import os
import sys
import re
import time
import hashlib
import http.client
import urllib.request
import urllib.parse
import urllib.error
from pathlib import Path

from bs4 import BeautifulSoup


USER_AGENT = os.environ.get(
    'PV_USER_AGENT',
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/605.1.15 '
    '(KHTML, like Gecko) Version/17.0 Safari/605.1.15'
)


def read_config(config_path: Path) -> dict:
    """Read key=value config file."""
    config = {}
    with open(config_path) as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith('#') and '=' in line:
                key, value = line.split('=', 1)
                config[key.strip()] = value.strip()
    return config


def normalize_url(url: str) -> str:
    """Normalize URLs by encoding unsafe characters in the path/query."""
    parts = urllib.parse.urlsplit(url)
    path = urllib.parse.quote(parts.path, safe="/%:")
    query = urllib.parse.quote(parts.query, safe="=&%")
    return urllib.parse.urlunsplit((parts.scheme, parts.netloc, path, query, parts.fragment))


def fetch_urls(url: str, regex: str, prefix: str) -> list[str]:
    """Fetch PDF URLs from a webpage."""
    req = urllib.request.Request(
        url,
        data=None,
        headers={
            'User-Agent': USER_AGENT,
        }
    )
    try:
        response = urllib.request.urlopen(req, timeout=30)
        html = response.read().decode('utf-8', errors='ignore')
    except urllib.error.HTTPError as err:
        html = err.read().decode('utf-8', errors='ignore')
    soup = BeautifulSoup(html, "html.parser")

    links = soup.find_all('a', href=re.compile(r'(' + regex + ')'))
    hrefs = [el['href'] for el in links]
    if not hrefs:
        pattern = re.compile(regex)
        hrefs = [match.group(0) for match in pattern.finditer(html)]

    urls = []
    for href in hrefs:
        if href.startswith(prefix) or href.startswith('http'):
            url = href
        else:
            url = prefix + href
        urls.append(normalize_url(url))

    return sorted(set(urls))


def extract_filename(content_disp: str) -> str | None:
    """Extract filename from Content-Disposition header."""
    if not content_disp:
        return None

    parts = [part.strip() for part in content_disp.split(';')]
    for part in parts:
        lower = part.lower()
        if lower.startswith('filename*='):
            value = part.split('=', 1)[1].strip().strip('"\'')
            if "''" in value:
                value = value.split("''", 1)[1]
            return urllib.parse.unquote(value)
        if lower.startswith('filename='):
            value = part.split('=', 1)[1].strip()
            if value and value[0] in ('"', "'"):
                quote = value[0]
                if value.endswith(quote):
                    value = value[1:-1]
                else:
                    value = value[1:]
            return value

    return None


def normalize_download_filename(filename: str, content_type: str = '') -> str:
    """Add a .pdf suffix when the downloaded file is clearly a PDF."""
    filename = filename.strip()
    if not filename:
        return filename

    lower_name = filename.lower()
    lower_type = content_type.lower()
    if lower_name.endswith('.pdf'):
        return filename

    is_pdf_slug = lower_name.endswith('pdf')
    is_pdf_response = 'application/pdf' in lower_type
    if is_pdf_slug or is_pdf_response:
        return f"{filename}.pdf"

    return filename


def download_file(
    url: str,
    dest: Path,
    delay: float = 0.5,
    retries: int = 3,
    retry_delay: float = 2.0,
) -> bool:
    """Download a file, returning True if successful.

    For dsresource-type URLs, the real filename comes from Content-Disposition
    header, so we need to fetch before we can check if file exists.
    """
    is_dsresource = 'dsresource' in url or 'objectid=' in url

    # For normal URLs, check if file exists first
    if not is_dsresource and dest.exists() and dest.stat().st_size > 0:
        print(f"Skipping (exists): {dest.name}")
        return True

    delay = max(delay, 0.5)
    for attempt in range(1, retries + 1):
        time.sleep(delay)
        try:
            req = urllib.request.Request(
                url,
                data=None,
                headers={
                    'User-Agent': USER_AGENT,
                }
            )
            with urllib.request.urlopen(req, timeout=30) as response:
                # Check for Content-Disposition header for filename
                content_disp = response.headers.get('Content-Disposition', '')
                content_type = response.headers.get('Content-Type', '')
                if 'filename=' in content_disp:
                    new_name = extract_filename(content_disp)
                    if new_name:
                        dest = dest.parent / normalize_download_filename(new_name, content_type)
                else:
                    dest = dest.parent / normalize_download_filename(dest.name, content_type)

                # Now check if file exists (for dsresource URLs)
                if dest.exists() and dest.stat().st_size > 0:
                    print(f"Skipping (exists): {dest.name}")
                    return True

                print(f"Downloading: {dest.name}")
                with open(dest, 'wb') as f:
                    f.write(response.read())
            return True
        except http.client.IncompleteRead as e:
            print(f"  Error downloading {url}: {e}")
        except Exception as e:
            print(f"  Error downloading {url}: {e}")

        if dest.exists():
            dest.unlink()

        if attempt < retries:
            backoff = max(retry_delay, delay) * attempt
            print(f"  Retrying in {backoff:.1f}s (attempt {attempt + 1}/{retries})")
            time.sleep(backoff)
        else:
            return False

    return False


def sha256_file(path: Path) -> str:
    """Calculate SHA256 hash of a file."""
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(8192), b''):
            h.update(chunk)
    return h.hexdigest()


def generate_checksums(directory: Path, checksum_file: Path, blacklist_pattern: str | None = None) -> None:
    """Generate SHA256SUMS file for PDFs only (not config/metadata files).
    
    If blacklist_pattern is provided, files matching the pattern are excluded.
    """
    checksums = []

    # Find all relevant files (exclude metadata files)
    exclude = {
        'README.md',

        'config.txt',
        'SHA256SUMS',
        '.DS_Store',
        '.lock',
        '.todo',
    }
    
    blacklist_re = re.compile(blacklist_pattern, re.IGNORECASE) if blacklist_pattern else None
    
    for f in sorted(directory.iterdir()):
        if f.name in exclude or f.suffix == '.txt':
            continue
        if f.is_file():
            # Check against blacklist pattern
            if blacklist_re and blacklist_re.search(f.name):
                print(f"Skipping (blacklist): {f.name}")
                continue
            h = sha256_file(f)
            checksums.append(f"{h}  {f.name}")

    with open(checksum_file, 'w') as f:
        f.write('\n'.join(checksums) + '\n')

    print(f"Generated {checksum_file.name} with {len(checksums)} entries")


def count_pdf_files(directory: Path) -> int:
    """Count PDF files in directory (excluding metadata)."""
    exclude = {
        'README.md',

        'config.txt',
        'SHA256SUMS',
        '.DS_Store',
        '.lock',
        '.todo',
    }
    count = 0
    for f in directory.iterdir():
        if f.name in exclude or f.suffix == '.txt':
            continue
        if f.is_file():
            count += 1
    return count


def count_checksum_lines(checksum_file: Path) -> int:
    """Count non-empty lines in checksum file."""
    count = 0
    with open(checksum_file) as f:
        for line in f:
            if line.strip():
                count += 1
    return count


def verify_checksums(directory: Path, checksum_file: Path) -> bool:
    """Verify checksums, returns True if all match."""
    all_ok = True

    with open(checksum_file) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            expected_hash, filename = line.split('  ', 1)
            filepath = directory / filename

            if not filepath.exists():
                print(f"MISSING: {filename}")
                all_ok = False
                continue

            actual_hash = sha256_file(filepath)
            if actual_hash == expected_hash:
                print(f"OK: {filename}")
            else:
                print(f"MISMATCH: {filename}")
                all_ok = False

    return all_ok


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    muni_dir = Path(sys.argv[1]).resolve()
    if not muni_dir.is_dir():
        print(f"Error: {muni_dir} is not a directory")
        sys.exit(1)

    config_file = muni_dir / 'config.txt'
    if not config_file.exists():
        print(f"Error: {config_file} not found")
        print("Create a config.txt with URL, REGEX, and PREFIX settings")
        sys.exit(1)

    config = read_config(config_file)

    required = ['URL', 'REGEX', 'PREFIX']
    missing = [k for k in required if k not in config]
    if missing:
        print(f"Error: Missing config keys: {', '.join(missing)}")
        sys.exit(1)

    # Determine list file name from config NAME or directory name
    # Config can have NAME=Amersfoort, or directory can be "0307 Amersfoort"
    dir_name = muni_dir.name
    code = dir_name.split()[0] if ' ' in dir_name else dir_name

    if 'NAME' in config:
        list_name = f"{code} {config['NAME']}.txt"
    elif ' ' in dir_name:
        list_name = f"{dir_name}.txt"
    else:
        list_name = f"{code}.txt"

    list_file = muni_dir / list_name
    checksum_file = muni_dir / 'SHA256SUMS'

    # Step 1: Generate URL list if missing
    if not list_file.exists() or list_file.stat().st_size == 0:
        print(f"Fetching URLs from {config['URL']}")
        urls = fetch_urls(config['URL'], config['REGEX'], config['PREFIX'])

        if not urls:
            print("Warning: No URLs found matching pattern")
        else:
            with open(list_file, 'w') as f:
                f.write('\n'.join(urls) + '\n')
            print(f"Found {len(urls)} URLs, saved to {list_file.name}")

    # Step 2: Download from list
    if list_file.exists():
        with open(list_file) as f:
            urls = [normalize_url(line.strip()) for line in f if line.strip()]

        print(f"\nDownloading {len(urls)} files...")
        for url in urls:
            # Extract filename from URL
            parsed = urllib.parse.urlparse(url)
            filename = os.path.basename(parsed.path)
            if not filename:
                filename = url.split('/')[-1].split('?')[0]
            filename = normalize_download_filename(filename)

            dest = muni_dir / filename
            if not download_file(url, dest):
                print("Aborting after first download failure.")
                sys.exit(1)

    # Step 3: Generate or verify checksums
    print()
    blacklist_pattern = config.get('BLACKLIST_PATTERN')
    if blacklist_pattern:
        print(f"Using blacklist pattern: {blacklist_pattern}")
    
    pdf_count = count_pdf_files(muni_dir)
    if checksum_file.exists() and checksum_file.stat().st_size > 0:
        print("Verifying checksums...")
        if verify_checksums(muni_dir, checksum_file):
            checksum_count = count_checksum_lines(checksum_file)
            if checksum_count != pdf_count:
                print("\nChecksum count mismatch; regenerating checksums...")
                generate_checksums(muni_dir, checksum_file, blacklist_pattern)
            else:
                print("\nAll checksums OK!")
        else:
            print("\nChecksum verification FAILED!")
            sys.exit(1)
    elif pdf_count == 0:
        if checksum_file.exists():
            checksum_file.unlink()
        print("No downloaded files found; skipping SHA256SUMS generation.")
    else:
        print("Generating checksums...")
        generate_checksums(muni_dir, checksum_file, blacklist_pattern)

    print("\nDone!")


if __name__ == '__main__':
    main()
