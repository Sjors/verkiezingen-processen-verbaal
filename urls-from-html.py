#!/usr/bin/env python3
# Based on: https://stackoverflow.com/a/42510443/313633
#
# pip install bs4
#
# Usage: ./urls-from-html.py https://www.voorneaanzee.nl/processen-verbaal-tweede-kamerverkiezing-2023

from urllib.request import urlopen
from bs4 import BeautifulSoup
import re
import sys

response = urlopen(sys.argv[1]).read()
soup= BeautifulSoup(response, "html.parser")
links = soup.find_all('a', href=re.compile(r'(\.pdf)'))

for el in links:
    print(el['href'])