#!/bin/sh
stad () {
    find . -type f -name '*.txt' | grep $1
}
pushd 2023-TK
    ../urls-from-html.py https://www.utrecht.nl/bestuur-en-organisatie/verkiezingen/ > "$(stad 0344)" ".pdf" https://www.utrecht.nl
    ../urls-from-html.py https://www.eindhoven.nl/bestuur-en-beleid/verkiezing-tweede-kamer-2023/verkiezingsuitslagen > "$(stad 0772)"
    ../urls-from-html.py https://tellingen.stembureausinrotterdam.nl/Documenten-TK23 > "$(stad 0599)" ".pdf" https://tellingen.stembureausinrotterdam.nl
    ../urls-from-html.py https://www.zwolle.nl/uitslagen-verkiezingen > "$(stad 0193)" ".pdf" https://www.zwolle.nl
    ../urls-from-html.py https://zutphen.nl/verkiezingen/uitslagen > "$(stad 0301)" ".pdf" https://zutphen.nl
    ../urls-from-html.py https://www.zoeterwoude.nl/gemeenteraad-en-bestuur/verkiezingen > "$(stad 0638)" ".pdf" https://www.zoeterwoude.nl
popd