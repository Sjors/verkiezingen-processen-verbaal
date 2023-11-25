#!/usr/bin/env python3
# Based on: https://stackoverflow.com/a/42510443/313633
#
# pip install bs4
#
# Usage:
# ./urls-from-html.py https://www.voorneaanzee.nl/processen-verbaal-tweede-kamerverkiezing-2023
# ./urls-from-html.py https://www.venray.nl/uitslag-tweede-kamerverkiezingen-2023 "/file/proces" https://www.venray.nl

from urllib.request import urlopen
from bs4 import BeautifulSoup
import re
import sys

response = urlopen(sys.argv[1]).read()
soup= BeautifulSoup(response, "html.parser")

regex = sys.argv[2] if len(sys.argv) > 2 else ".pdf"
prefix = sys.argv[3] if len(sys.argv) > 3 else ""
links = soup.find_all('a', href=re.compile(r'(' + regex + ')'))

for el in links:
    print(el['href']) if el['href'].startswith(prefix) else print(prefix + el['href'])
