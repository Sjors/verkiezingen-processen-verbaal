# Processen-verbaal verkiezingen

Lijst met URL's van processen-verbaal van verkiezingen, maar niet de documenten zelf.

Dit maakt het makkelijker ze te downloaden.

## Gebruik

Installeer [jq](https://jqlang.github.io/jq/).

Download alle processesen-verbaal voor de gemeentes die aan dit project zijn toegevoegd:

```sh
./download.sh
```

Mocht je een foutmelding krijgen, probeer het commando opnieuw. Gemeentes die
reeds compleet zijn. worden niet opnieuw gedownload.

Voor de volgende gemeenten is een apart commando nodig:
* Losser
* Oosterhout
* Nissewaard
* Enschede
* Zwijndrecht
* Haarlem
* Zandvoort

```sh
./overige-gemeentes.sh
```

Voor Leiden dient handmatig een zip bestand gedownload te worden, zie issue #68

Roermond (issue #56) en Assen (issue #55) gebruiken Google Drive en Stack Storage,
dus moeten ook handmatig gedownload worden.

# Hashes en timestamp

De sha256 hashes van alle processen-verbaal staan in [Timestamps/2023-TK.asc](Timestamps/2023-TK.asc), als volgt:

```sh
find * -type f -not -path '**/*.DS_Store' -not -path '*.txt' -exec shasum -a 256 {} \; | sort -k 2 --version-sort > 2023-TK
gpg --clear-sign 2023-TK
```

Daarnaast heb ik een timestamp gemaakt, welke te verifieren is op [opentimestamps.org](https://opentimestamps.org)
of met [ots-client](https://github.com/opentimestamps/opentimestamps-client) en je eigen Bitcoin node:

```
ots verify 2023-TK.asc.ots
Success! Bitcoin block 818632 attests existence as of 2023-11-26 CET
```

## Gemeente toevoegen

Meestal als volgt:

1. Zoek de processen-verbaal op de site van de gemeente:

    a) via de links van Kiesraad [hier](https://www.kiesraad.nl/verkiezingen/tweede-kamer/uitslagen/uitslagen-per-gemeente-tweede-kamer); of

    b) Google "[gemeente] processen-verbaal tweede kamerverkiezingen 2023"

2. Kopieer de URL van de pagina met de tabel van processen-verbaal
3. Controleer of hij te verwerken is: `./urls-from-html.py https://www.[gemeente].nl/verkiezingen/processen-verbaal-.../`: zie je een lijst met PDF's?
4. Zoek het .txt bestand voor de gemeente, bv. "0034 Utrecht.txt"
5. Sla op: `./urls-from-html.py https://utrecht.nl/... | uniq > 2023-TK/0034\ Utrecht.txt`
6. Download de documenten: `./download.sh`
7. Controleer één of meer PDF-bestanden om te zien of de download gelukt is
8. Commit: zet de URL in de commit message
9. Maak een pull request

10. (Optioneel): voeg het gebruikte download commando toe aan `scrape-urls.sh`

Om te zien welke gemeentes nog ontbreken:

```
./progress.sh
```

N.B. sommmige in deze lijst hebben wel processen-verbaal gepubliceerd, maar
die zijn niet via een rechtstreekse URL te benaderen.

## Eigenaardigheden per gemeente

* De bestanden van de volgende gemeenten hebben geen PDF-extensie; voeg evt. `.pdf` toe om ze te bekijken
    * Emmen
    * Venray
    * Tiel
    * Coevorden
    * Oudewater
    * Raalte
    * Borne
    * Oss
    * Borger-Odoorn
    * Leidschendam-Voorburg
    * Culemborg
    * Duiven
    * Drimmelen
    * Dinkelland
    * Gemert-Bakel
    * Kampen
    * Haarlemmermeer
    * Hendrik-Ido-Ambacht
    * Bodegraven-Reeuwijk
