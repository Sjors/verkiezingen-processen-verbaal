#!/bin/sh
# Voor elke gemeente met processen-verbaal,
# download ze hooguit één keer.

pushd 2023-TK
    find *.txt -type f -size +0 | while read -r file; do
        n=`echo $file | awk '{print $1}'`
        naam=`echo $file | awk '{print substr($2, 1, length($2)-4)}'`
        echo "Gemeente $n: $naam"
        mkdir -p $n
        cat "$file" | while read url || [[ -n $url ]]; do
          dest="$n/`basename $url | cut -d? -f1`"
          if [ ! -s "$dest" ]; then
            wget $url -O "$dest" --no-check-certificate
          fi
        done
    done

popd