#!/usr/bin/env python3
#
# pip install requests
#
# Usage example:
#  mkdir 0168
#  cd 0168
#  ./mijnstembureau.py https://mijnstembureau-losser.nl/uitslagen/ep/totaal

from urllib.parse import urlparse, urlunparse, quote
from requests import Request, Session
from pathlib import Path
from datetime import datetime
import os
import sys
import json


def filter_filename(filename):
    """
    Filters unsafe characters from a filename and preserves spaces and periods.

    Args:
         filename (str): The original filename.

    Returns:
         str: The filtered file name.
    """

    keepcharacters = (" ", ".", "_")
    result = "".join(c for c in filename if c.isalnum() or c in keepcharacters).rstrip()
    return result


######### main

overwrite_files = False
target_date = datetime(2024, 6, 1)


try:
    base_url = sys.argv[1]
    base_url = base_url.rstrip("/")
except IndexError as e:
    print("Provide as first argument the 'mijnstembureau' url ending with /ep/totaal or /ep/download")
    sys.exit(1)

parsed_url = urlparse(base_url)
host = parsed_url.netloc
api_url = urlunparse((parsed_url.scheme, parsed_url.netloc, "/api/prime/uitslagen", "", "", ""))



request_headers = {
  "Accept": "application/json, text/plain, */*",
  "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
  "Sec-Fetch-Dest": "empty",
  "Sec-Fetch-Mode": "cors",
  "Sec-Fetch-Site": "same-origin",
  "X-Requested-With": "XMLHttpRequest",
  "Host": host,
  "Referer": base_url
}


s = Session()
s.headers = request_headers

req = Request("GET", api_url)
prepped = s.prepare_request(req)
resp = s.send(prepped)

if resp:
    try:
        data = json.loads(resp.content)

        for election in data:
            electiondate = datetime.strptime(election["electionDate"], "%Y-%m-%dT%H:%M:%S.%fZ")

            if electiondate >= target_date:
                uitslagid = election["uitslagId"]
                pvkeys = election["pvKeys"]

                for item in pvkeys:
                    id = item["_id"]
                    path = item["pvAwsKey"]
                    filepath = Path(path)

                    if filepath.suffix.casefold() == ".pdf":
                        safe_filename = filter_filename(filepath.name)

                        url_viewpv = f"{api_url}/view-pv/{quote(uitslagid)}/{quote(id)}"

                        if not os.path.exists(safe_filename) or overwrite_files:
                            print(path)
                            resp_pdf = s.get(url_viewpv)
                            if resp_pdf:
                                with open(safe_filename, "wb") as f:
                                    f.write(resp_pdf.content)
                            else:
                                print(f"Error requesting url {url_viewpv}")
                                sys.exit(2)

    except Exception as e:
        print(f"Error loading JSON: {e}")
        sys.exit(1)


else:
    print(f"Error requesting url {api_url}")
