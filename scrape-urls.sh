#!/bin/sh
stad () {
    find . -type f -name '*.txt' | grep $1
}
pushd 2023-TK
    ../urls-from-html.py https://www.utrecht.nl/bestuur-en-organisatie/verkiezingen/ > "$(stad 0344)" ".pdf" https://www.utrecht.nl
popd