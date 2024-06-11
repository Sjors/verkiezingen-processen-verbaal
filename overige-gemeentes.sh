#!/bin/sh
pushd 2024-EP
    mkdir -p 0153 && pushd 0153
        ../../mijnstembureau.py https://mijnstembureau-enschede.nl/uitslagen/ep/totaal
    popd
    mkdir -p 0168 && pushd 0168
        ../../mijnstembureau.py https://mijnstembureau-losser.nl/uitslagen/ep/totaal
    popd
    mkdir -p 0642 && pushd 0642
        ../../mijnstembureau.py https://mijnstembureau-zwijndrecht.nl/uitslagen/ep/totaal
    popd
    mkdir -p 0826 && pushd 0826
        ../../mijnstembureau.py https://www.mijnstembureau-oosterhout.nl/uitslagen/ep/totaal
    popd
    mkdir -p 1930 && pushd 1930
        ../../mijnstembureau.py https://stembureau-nissewaard.nl/uitslagen/ep/totaal
    popd
    mkdir -p 0392 && pushd 0392
        ../../pleio.py Haarlem
    popd
    mkdir -p 0473 && pushd 0473
        ../../pleio.py Zandvoort
    popd
popd
