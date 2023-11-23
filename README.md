# Processen-verbaal verkiezingen

Lijst met URL's van processen-verbaal van verkiezingen, maar niet de documenten zelf.

Dit maakt het makkelijker ze te downloaden.

## Gemeente toevoegen

Meestal als volgt:

1. Google "[gemeente] processen-verbaal tweede kamerverkiezingen 2023"
2. Kopieer de URL van de pagina met de tabel van processen-verbaal
3. Check of hij te verwerken is: `./urls-from-html.py https://www.[gemeente].nl/verkiezingen/processen-verbaal-.../`: zie je een lijst met PDF's?
4. Zoek het .txt bestand voor de gemeente, bv. "0034 Utrecht.txt"
5. Sla op: `./urls-from-html.py https://utrecht.nl/... | uniq > 2023-TK/0034\ Utrecht.txt`
6. Download de documenten: `./download.sh`
7. Check één of meer PDF bestanden om te zien of de download gelukt is
8. Commit: zet de URL in de commit message
9. maak een pull request

## Eigenaardigheden per gemeente

* Venray: de bestanden hebben geen PDF extentie; voeg evt. `.pdf` toe om ze te bekijken
