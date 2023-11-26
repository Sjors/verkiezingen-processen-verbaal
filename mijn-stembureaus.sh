#!/bin/sh
pushd 2023-TK
    mkdir -p 0168 && pushd 0168
        ../../mijnstembureau.py https://mijnstembureau-losser.nl/uitslagen/tk/totaal
    popd
    mkdir -p 0826 && pushd 0826
        ../../mijnstembureau.py https://www.mijnstembureau-oosterhout.nl/uitslagen/tk/totaal
    popd
    mkdir -p 1930 && pushd 1930
        ../../mijnstembureau.py https://stembureau-nissewaard.nl/uitslagen/tk/totaal
    popd
popd