#!/bin/sh
stad () {
    find . -type f -name '*.txt' | grep $1
}
pushd 2023-TK
    ../urls-from-html.py https://www.waalwijk.nl/verkiezingsuitslag-verkiezingen-tweede-kamer-2023 > "$(stad 0867)"
    ../urls-from-html.py https://www.wageningen.nl/actueel/tweede-kamer-verkiezingen/uitslagen/ > "$(stad 0289)"
    ../urls-from-html.py https://www.wassenaar.nl/uitslag-van-de-verkiezingen-voor-de-tweede-kamer-2023 > "$(stad 0629)"
    ../urls-from-html.py https://www.waterland.nl/uitslagen-tweede-kamerverkiezingen-2023 > "$(stad 0852)"
    ../urls-from-html.py https://www.weert.nl/verkiezingen ".pdf" https://www.weert.nl > "$(stad 0988)"
     ../urls-from-html.py https://www.westmaasenwaal.nl/uitslag-tweede-kamerverkiezingen > "$(stad 0668)"
    echo "https://www.gemeentewesterveld.nl/Bestuur_en_organisatie/Actueel/Verkiezingen/Uitslag_verkiezingen_22_november_2023/Verslag_controleprotocol_gemeentelijk_stembureau.pdf" > "$(stad 1701)"
    ../urls-from-html.py https://www.gemeentewesterveld.nl/Bestuur_en_organisatie/Actueel/Verkiezingen/Uitslag_verkiezingen_22_november_2023/Proces_verbaal_per_stembureau ".pdf" https://www.gemeentewesterveld.nl >> "$(stad 1701)"
    ../urls-from-html.py https://www.gemeentewesterveld.nl/Bestuur_en_organisatie/Actueel/Verkiezingen/Uitslag_verkiezingen_22_november_2023/Uitkomsten_per_stembureau ".pdf" https://www.gemeentewesterveld.nl >> "$(stad 1701)"
    ../urls-from-html.py https://www.westervoort.nl/voorlopige-uitslag-verkiezingen-tweede-kamer  ".pdf" https://www.westervoort.nl > "$(stad 0293)"
    ../urls-from-html.py https://www.westerwolde.nl/verkiezingsuitslagen-bekijken > "$(stad 1950)"
    ../urls-from-html.py https://www.wijdemeren.nl/4/verkiezingen/Verkiezingsuitslag.html  ".pdf" https://www.wijdemeren.nl > "$(stad 1969)"
    ../urls-from-html.py https://www.wijkbijduurstede.nl/verkiezingen/uitslagen-verkiezing-tweede-kamer-2023  ".pdf" https://www.wijkbijduurstede.nl > "$(stad 0352)"
    ../urls-from-html.py https://www.wijkbijduurstede.nl/verkiezingen/proces-verbalen-verkiezing-tweede-kamer-2023 ".pdf" https://www.wijkbijduurstede.nl >> "$(stad 0352)"
    ../urls-from-html.py https://www.wijkbijduurstede.nl/verkiezingen/uitslagen-per-stembureau ".pdf" https://www.wijkbijduurstede.nl >> "$(stad 0352)"
    ../urls-from-html.py https://www.winterswijk.nl/uitslagen | grep -v readspeaker > "$(stad 0294)"
    ../urls-from-html.py https://www.woensdrecht.nl/uitslagen-verkiezingen-tweede-kamer-2023  > "$(stad 0873)"
    ../urls-from-html.py https://www.woerden.nl/Verkiezingen/Verkiezingen_2023 > "$(stad 0632)" ".pdf" https://www.woerden.nl
    ../urls-from-html.py https://wormerland.nl/verkiezingen/uitslagen-verkiezingen-tweede-kamer-2023 > "$(stad 0880)" ".pdf" https://wormerland.nl
    ../urls-from-html.py https://www.woudenberg.nl/tweede-kamerverkiezingen-2023 > "$(stad 0351)" ".pdf" https://www.woudenberg.nl
    ../urls-from-html.py https://www.utrecht.nl/bestuur-en-organisatie/verkiezingen/ > "$(stad 0344)" ".pdf" https://www.utrecht.nl
    ../urls-from-html.py https://www.eindhoven.nl/bestuur-en-beleid/verkiezing-tweede-kamer-2023/verkiezingsuitslagen > "$(stad 0772)"
    ../urls-from-html.py https://tellingen.stembureausinrotterdam.nl/Documenten-TK23 > "$(stad 0599)" ".pdf" https://tellingen.stembureausinrotterdam.nl
    ../urls-from-html.py https://www.zwolle.nl/uitslagen-verkiezingen > "$(stad 0193)" ".pdf" https://www.zwolle.nl
    ../urls-from-html.py https://zutphen.nl/verkiezingen/uitslagen > "$(stad 0301)" ".pdf" https://zutphen.nl
    ../urls-from-html.py https://www.zoeterwoude.nl/gemeenteraad-en-bestuur/verkiezingen > "$(stad 0638)" ".pdf" https://www.zoeterwoude.nl
    ../urls-from-html.py https://www.zoetermeer.nl/processen-verbaal-tellingen-tweede-kamerverkiezing-2023  > "$(stad 0637)"
    ../urls-from-html.py https://www.zevenaar.nl/voorlopige-uitslag-verkiezingen-tweede-kamer > "$(stad 0299)" ".pdf" https://www.zevenaar.nl
    ../urls-from-html.py https://www.zeist.nl/gemeente-bestuur-en-organisatie/verkiezingen/uitslag-verkiezingen ".pdf" https://www.zeist.nl | grep -v docreader > "$(stad 0355)"
    ../urls-from-html.py https://www.zeewolde.nl/verkiezingen/uitslagen ".pdf" https://www.zeewolde.nl > "$(stad 0050)"
    ../urls-from-html.py https://www.zaltbommel.nl/inwoner-en-ondernemer/tweede-kamer-verkiezingen-2023/uitslag-verkiezingen-tweede-kamer/uitslag-verkiezingen-tweede-kamer ".pdf" https://www.zaltbommel.nl > "$(stad 0297)"
    ../urls-from-html.py https://www.hoogeveen.nl/verkiezingen/bekijken-stemmen-per-stembureau/ > "$(stad 0118)" ".pdf" https://www.hoogeveen.nl
popd