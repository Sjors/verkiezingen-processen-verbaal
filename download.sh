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
        if (( $n_urls > $n_files )); then
          echo "Gemeente $n: $naam, $n_urls bestanden beschikbaar, $n_files reeds gedownload..."
          cat "$file" | while read url || [[ -n $url ]]; do
            case $n in
              0317|0677|1680)
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
                dest="$n/`basename $url | cut -d? -f1`"
                if [ ! -s "$dest" ]; then
                  wget $url -O "$dest" --no-check-certificate
                fi
                ;;
            esac
          done
        fi
    done

popd