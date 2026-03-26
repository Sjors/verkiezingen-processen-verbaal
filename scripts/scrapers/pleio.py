#!/usr/bin/env python3
"""
Download PDFs from Pleio (gemeente.pleio.nl) websites.

Usage:
  ./pleio-fetch.py <municipality_name> [pleio_url]

Examples:
  ./pleio-fetch.py Zandvoort https://haarlem.pleio.nl/groups/view/.../files/...
  ./pleio-fetch.py Haarlem

If no URL provided, uses default Haarlem Pleio URL.
Downloads in the current folder.
"""

from urllib.parse import urlparse, urlunparse
from pathlib import Path
from datetime import datetime
from zoneinfo import ZoneInfo
from requests import Request, Session
import os
import sys
import json
import re


def filter_filename(filename):
    """Filters unsafe characters from a filename and preserves spaces and periods."""
    keepcharacters = (" ", ".", "_")
    filename = filename.replace("+", " ")
    result = "".join(c for c in filename if c.isalnum() or c in keepcharacters).rstrip()
    return result


def create_json_data(rootid, id):
    return {
        'operationName': 'FilesList',
        'variables': {
            'offset': 0,
            'limit': 100,
            'rootContainerGuid': rootid,
            'containerGuid': id,
            'filterBookmarks': False,
            'orderBy': 'filename',
            'orderDirection': 'asc',
        },
        'query': 'query FilesList($rootContainerGuid: String, $containerGuid: String!, $filterBookmarks: Boolean, $offset: Int = 0, $limit: Int = 50, $orderBy: String, $orderDirection: String) {\n  site {\n    guid\n    collabEditingEnabled\n    readAccessIds {\n      id\n      description\n      __typename\n    }\n    __typename\n  }\n  viewer {\n    guid\n    loggedIn\n    isAdmin\n    canWriteToContainer(containerGuid: $containerGuid, subtype: "file")\n    user {\n      guid\n      name\n      icon\n      url\n      __typename\n    }\n    __typename\n  }\n  containerEntity: entity(guid: $containerGuid) {\n    guid\n    ... on Folder {\n      title\n      canEdit\n      subtype\n      size\n      owner {\n        guid\n        name\n        __typename\n      }\n      hasChildren\n      timeUpdated\n      timePublished\n      accessId\n      writeAccessId\n      tags\n      tagCategories {\n        name\n        values\n        __typename\n      }\n      __typename\n    }\n    __typename\n  }\n  rootEntity: entity(guid: $rootContainerGuid) {\n    guid\n    ... on User {\n      canEdit\n      __typename\n    }\n    ... on Group {\n      canEdit\n      readAccessIds {\n        id\n        description\n        __typename\n      }\n      defaultTags\n      defaultTagCategories {\n        name\n        values\n        __typename\n      }\n      __typename\n    }\n    __typename\n  }\n  files(\n    containerGuid: $containerGuid\n    offset: $offset\n    limit: $limit\n    orderBy: $orderBy\n    orderDirection: $orderDirection\n    filterBookmarks: $filterBookmarks\n  ) {\n    total\n    edges {\n      guid\n      ... on File {\n        subtype\n        title\n        description\n        richDescription\n        url\n        size\n        isBookmarked\n        download\n        timeUpdated\n        timePublished\n        mimeType\n        canEdit\n        thumbnail\n        accessId\n        writeAccessId\n        tags\n        tagCategories {\n          name\n          values\n          __typename\n        }\n        owner {\n          guid\n          name\n          __typename\n        }\n        referenceCount\n        references {\n          guid\n          label\n          url\n          referenceType\n          entityType\n          __typename\n        }\n        __typename\n      }\n      ... on Folder {\n        subtype\n        title\n        description\n        richDescription\n        url\n        hasChildren\n        isBookmarked\n        timeUpdated\n        timePublished\n        mimeType\n        canEdit\n        accessId\n        writeAccessId\n        tags\n        tagCategories {\n          name\n          values\n          __typename\n        }\n        owner {\n          guid\n          name\n          __typename\n        }\n        referenceCount\n        references {\n          guid\n          label\n          url\n          referenceType\n          entityType\n          __typename\n        }\n        __typename\n      }\n      ... on Pad {\n        subtype\n        title\n        richDescription\n        url\n        isBookmarked\n        timeUpdated\n        timePublished\n        canEdit\n        accessId\n        writeAccessId\n        owner {\n          guid\n          name\n          __typename\n        }\n        tags\n        tagCategories {\n          name\n          values\n          __typename\n        }\n        group {\n          guid\n          url\n          __typename\n        }\n        referenceCount\n        references {\n          guid\n          label\n          url\n          referenceType\n          entityType\n          __typename\n        }\n        __typename\n      }\n      __typename\n    }\n    __typename\n  }\n}',
    }


def get_level_filter(level, filterarray):
    if level < len(filterarray):
        return filterarray[level]
    else:
        return None


def process_container(s, level, rootid, id, host, settings, api_url):
    req = Request("POST", api_url, json=create_json_data(rootid, id))
    prepped = s.prepare_request(req)
    resp = s.send(prepped)

    if resp:
        try:
            data = json.loads(resp.content).get("data")

            if not data is None:
                files = data["files"]
                edges = files["edges"]
                for edge in edges:
                    timepublished = datetime.strptime(edge["timePublished"], "%Y-%m-%dT%H:%M:%S.%f%z")

                    if timepublished >= settings["target_date"]:
                        guid = edge["guid"]
                        subtype = edge["subtype"]
                        title = edge["title"]

                        level_filter = get_level_filter(level, settings["level_filter"])
                        if level_filter is None or re.search(level_filter, title, re.IGNORECASE):
                            
                            if subtype == "folder":
                                process_container(s, level+1, rootid, guid, host, settings, api_url)

                            elif subtype == "file":
                                downloadurl = edge["download"]
                                
                                downloadpath = Path(downloadurl)
                                titlepath = Path(title)
                                if downloadpath.suffix.casefold() in settings["extensions"] or titlepath.suffix.casefold() in settings["extensions"]:
                                    safe_filename = filter_filename(title)
                                    if not safe_filename:
                                        safe_filename = filter_filename(downloadpath.name)

                                    if not os.path.exists(safe_filename) or settings["overwrite_files"]:
                                        fullurl = f"{host}{downloadurl}"
                                        print(fullurl)
                                        resp_pdf = s.get(fullurl)
                                        if resp_pdf:
                                            with open(safe_filename, "wb") as f:
                                                f.write(resp_pdf.content)
                                        else:
                                            print(f"Error requesting url {downloadurl}", file=sys.stderr)
                                            sys.exit(2)

        except Exception as e:
            print(f"Error loading JSON: {e}", file=sys.stderr)
            with open("response.json", "wb") as f:
                f.write(resp.content)
            sys.exit(1)
    else:
        print(f"Error requesting url {api_url}", file=sys.stderr)


# Main
if len(sys.argv) < 2:
    print("Usage: pleio-fetch.py <municipality> [pleio_url]")
    sys.exit(1)

municipality = sys.argv[1]

# Default to Haarlem Pleio main verkiezingen folder
starting_url = "https://haarlem.pleio.nl/groups/view/7b769524-7339-4ae6-ab49-c10b7c20abed/verkiezingen/files/ce13ac21-051b-4a33-bc64-0be1ae37ac94"

# If a second argument is provided, use it as the starting URL
if len(sys.argv) > 2:
    starting_url = sys.argv[2]
    print(f"Using provided URL: {starting_url}", file=sys.stderr)

settings = {
    "overwrite_files": False,
    "target_date": datetime(2026, 3, 1, tzinfo=ZoneInfo("Europe/Amsterdam")),
    "level_filter": [None],  # None = no filtering, get all files at all levels
    "extensions": [".pdf", ".csv"]
}

parsed_url = urlparse(starting_url)
api_url = urlunparse((parsed_url.scheme, parsed_url.netloc, "/graphql", "", "", ""))
host = f"{parsed_url.scheme}://{parsed_url.netloc}"

path_segments = parsed_url.path.split('/')
rootcontainerguid = path_segments[3]
containerguid = path_segments[-1]

request_headers = {
    'Authority': parsed_url.netloc,
    'Accept': '*/*',
    'Cache-Control': 'no-cache',
    'Content-Type': 'application/json',
    'Origin': host,
    'Pragma': 'no-cache',
    'Referer': starting_url,
    'Sec-Ch-Ua': '"Google Chrome";v="119", "Chromium";v="119", "Not?A_Brand";v="24"',
    'Sec-Ch-Ua-Mobile': '?0',
    'Sec-Ch-Ua-Platform': '"macOS"',
    'Sec-Fetch-Dest': 'empty',
    'Sec-Fetch-Mode': 'cors',
    'Sec-Fetch-Site': 'same-origin',
    'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36',
}

s = Session()
s.headers = request_headers

process_container(s, 0, rootcontainerguid, containerguid, host, settings, api_url)

s.close()
