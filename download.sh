#!/bin/sh
# Voor elke gemeente met processen-verbaal,
# download ze hooguit één keer.

set -e
pushd 2023-TK
    find *.txt -type f -size +0 | while read -r file; do
        n=`echo $file | awk '{print $1}'`
        naam=`echo $file | awk '{print substr($2, 1, length($2)-4)}'`
        mkdir -p $n
        # Tel aantal reeds gedownloade bestanden:
        # https://stackoverflow.com/a/11131443/313633
        shopt -s nullglob
        n_files=($n/*)
        n_files=${#n_files[@]}
        # Tel aantal regels in text bestand:
        n_urls=`grep -c ^ "$file"`
        # Sla over als er niet meer files zijn, dit is met name
        # nodig voor de --content-disposition bestanden, om
        # te voorkomen dat we die servers hameren.
        echo "Gemeente $n: $naam, $n_urls bestanden beschikbaar, $n_files reeds gedownload..."
        if (( $n_urls > $n_files )); then
          cat "$file" | while IFS= read -r url || [[ -n $url ]]; do
            # Haal line end characters weg:
            url="${url%%[[:cntrl:]]}"
            case $n in
              1680)
                # Informatie (tijdelijk) niet meer beschikbaar, laat bestaande downloads met rust
              ;;
              0047|0183|0202|0274|0317|0677|1674|1680|1883|1961)
                # URL's van het type dsresource?objectid=c52cd...
                # Deze zouden anders allemaal "dsresource" heten.
                # --no-clobber zorgt dat bestaande bestanden niet vervangen
                # worden.
                # --restrict-file-names is nodig voor bestandsnamen
                # als "Gemeenschapscentrum ´t Heike"
                # https://stackoverflow.com/a/22013384/313633
                wget "$url" --content-disposition --restrict-file-names=ascii --no-check-certificate --no-clobber --directory-prefix=$n
                ;;
              *)
                # Download alleen nieuwe bestanden (negeert wijzigigen)
                path=`basename "$url"`
                # Haal query string weg ?...
                dest="$n/${path%%\?*}"
                if [ ! -s "$dest" ]; then
                  sleep 1
                  wget "$url" -O "$dest" --no-check-certificate --read-timeout=5
                fi
                ;;
            esac
          done
        fi
    done

popd