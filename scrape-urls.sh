#!/bin/sh
# Eén of meer commando's om er gemeente de URL's van alle processen-verbaal te
# verzamelen.

# Hulpfunctie om het nummer van een stad om te zetten in de naam van het tekstbestand.
# Voorbeeld:
# stad 0344
# 0344 Utrecht.txt
stad () {
    find . -type f -name '*.txt' | grep $1
}

# Deze lijst is niet compleet. De URL's van sommige gemeentes zijn handmatig
# verzameld, danwel het gebruikte commando is hier niet gedocumenteerd. Dit
# kan eventueel later aangevuld worden.
pushd 2023-TK
    # A
	../urls-from-html.py https://www.almere.nl/bestuur/verkiezingen/processen-verbaal-tweede-kamerverkiezingen-2023 ".pdf" https://www.almere.nl > "$(stad 0034)"
	../urls-from-html.py https://www.asten.nl/gemeentebestuur-en-organisatie/verkiezingen/tweede-kamer-verkiezingen/voorlopige-verkiezingsuitslag-tweede-kamer-2023 ".pdf" https://www.asten.nl | grep -v docreader > "$(stad 0743)"
	../urls-from-html.py https://www.almelo.nl/uitslagenverkiezingen ".pdf" https://www.almelo.nl > "$(stad 0141)"
    ../urls-from-html.py https://www.arnhem.nl/Bestuur/Overig/verkiezingen/Telling_en_processen_verbaal_Tweede_Kamerverkiezingen_2023 "dsresource" https://www.arnhem.nl | sort -u --version-sort > "$(stad 0202)"
	../urls-from-html.py https://www.alkmaar.nl/verkiezingsuitslagen/ | sort -u > "$(stad 0361)"
	../urls-from-html.py https://www.aalsmeer.nl/bestuur-organisatie/publicatie/verkiezingen_uitslagen_definitieve-uitslag-tweede-kamerverkiezingen-2023 ".pdf" https://www.aalsmeer.nl > "$(stad 0358)"
	../urls-from-html.py https://www.apeldoorn.nl/tk-verkiezingen-2023/uitslag-tweede-kamerverkiezingen-2023 ".pdf" https://www.apeldoorn.nl > "$(stad 0200)"
	../urls-from-html.py https://www.amersfoort.nl/verkiezingsuitslagen > "$(stad 0307)"
	../urls-from-html.py https://www.amstelveen.nl/bestuur-organisatie/publicatie/verkiezingen_uitslagen_definitieve-uitslagen-tweede-kamerverkiezing-2023 ".pdf" https://www.amstelveen.nl > "$(stad 0362)"
	../urls-from-html.py https://www.alblasserdam.nl/Bestuur_Organisatie/Alle_onderwerpen/Verkiezingen/Tweede_Kamerverkiezing/Uitslag_Tweede_Kamerverkiezing_2023_Alblasserdam ".pdf" https://www.alblasserdam.nl > "$(stad 0482)"
	../urls-from-html.py https://www.alphen-chaam.nl/verkiezingsuitslagen ".pdf" https://www.alphen-chaam.nl > "$(stad 1723)"
	../urls-from-html.py https://www.albrandswaard.nl/home/verkiezingen/documenten-verkiezingenuitslag/ | sort -u --version-sort > "$(stad 0613)"

exit
    # B

    # C
    ../urls-from-html.py https://www.coevorden.nl/verkiezingsuitslagen "pdf" https://www.coevorden.nl > "$(stad 0109)"

    # D
    ../urls-from-html.py https://tellingen.stembureausindenhaag.nl/Processen-verbaal-Tweede-Kamerverkiezing-2023 ".pdf" https://tellingen.stembureausindenhaag.nl > "$(stad 0518)"
    ../urls-from-html.py https://tellingen.stembureausindenhaag.nl/Algemeen ".pdf" https://tellingen.stembureausindenhaag.nl >> "$(stad 0518)"

    # E
    ../urls-from-html.py https://www.eindhoven.nl/bestuur-en-beleid/verkiezing-tweede-kamer-2023/verkiezingsuitslagen > "$(stad 0772)"
    ../urls-from-html.py https://gemeente.emmen.nl/processen-verbaal "file" https://gemeente.emmen.nl | grep -v csv > "$(stad 0114)"

    # F

    # H
    ../urls-from-html.py https://www.hoogeveen.nl/verkiezingen/bekijken-stemmen-per-stembureau/ > "$(stad 0118)" ".pdf" https://www.hoogeveen.nl

    # I

    # K

    # L

    # M

    # N

    # O
    ../urls-from-html.py https://www.oudewater.nl/Verkiezingen/Verkiezingen_2023 ".org" https://www.oudewater.nl > "$(stad 0589)"
    ../urls-from-html.py https://www.overbetuwe.nl/verkiezingen/uitslagen > "$(stad 1734)"

    # P

    ../urls-from-html.py https://www.papendrecht.nl/Inwoners/Overzicht_Inwoners/Verkiezingen/Verkiezing_2023/Verkiezingsuitslagen_Tweede_Kamer_22_november_2023 ".pdf" https://www.papendrecht.nl > "$(stad 0590)"
    ../urls-from-html.py https://www.peelenmaas.nl/bestuur-en-organisatie/verkiezingen/Verkiezingsuitslag ".pdf" https://www.peelenmaas.nl | sort -u --version-sort > "$(stad 1894)"
    ../urls-from-html.py https://www.pekela.nl/Onderwerpen/Actueel/Verkiezingen/Alle_onderwerpen_verkiezingen/Processen_verbaal_Tweede_Kamerverkiezingen dsresource https://www.pekela.nl | grep -v pinterest > "$(stad 0765)"
    ../urls-from-html.py https://www.pijnacker-nootdorp.nl/politiek-en-organisatie/verkiezingen/uitslagen-tweede-kamerverkiezingen-22-november-2023/ |  sort -u --version-sort > "$(stad 1926)"
    ../urls-from-html.py https://purmerend.nl/bestuur-en-organisatie/verkiezingen/tweede-kamerverkiezing-2023/uitslag-tweede-kamerverkiezing-2023 "/document" https://purmerend.nl > "$(stad 0439)"
    echo "https://www.putten.nl/dsresource?objectid=2d8ccef1-bf90-4229-9d87-a84fb28a1f42&type=pdf" > "$(stad 0273)"
    echo "https://www.putten.nl/dsresource?objectid=8c684d3a-1f41-40b7-84ed-80ee9a063ab6&type=pdf" >> "$(stad 0273)"
    ../urls-from-html.py https://www.putten.nl/Bestuur/Verkiezingen/Uitslag_en_proces_verbaal_Tweede_Kamer_verkiezing_2023/Proces_Verbaal_per_stembureau ".pdf" https://www.putten.nl >> "$(stad 0273)"
    ../urls-from-html.py https://www.putten.nl/Bestuur/Verkiezingen/Uitslag_en_proces_verbaal_Tweede_Kamer_verkiezing_2023/Uitslag_per_stembureau ".pdf" https://www.putten.nl >> "$(stad 0273)"
    # R
    ../urls-from-html.py https://www.raalte.nl/uitslagen-tweede-kamerverkiezingen-2023 "/file" https://www.raalte.nl > "$(stad 0177)"
    ../urls-from-html.py https://www.reimerswaal.nl/uitslag-verkiezingen > "$(stad 0703)"
    ../urls-from-html.py https://www.renkum.nl/Bestuur/Tweede_Kamerverkiezing_2023/Uitslag_Tweede_Kamerverkiezing_2023 "dsresource" https://www.renkum.nl | grep -v pinterest > "$(stad 0274)"
    ../urls-from-html.py https://www.renswoude.nl/verkiezingen | grep -v docreader > "$(stad 0339)"
    ../urls-from-html.py https://www.reuseldemierden.nl/zo-is-er-gestemd-in-reusel-de-mierden | grep -v docreader > "$(stad 1667)"
    ../urls-from-html.py https://www.rheden.nl/Inwoners/Tweede_Kamerverkiezingen_2023/Verkiezingsuitslag/Proces_verbalen ".pdf" https://www.rheden.nl > "$(stad 0275)"
    ../urls-from-html.py https://www.ridderkerk.nl/actueel/processen-verbaal-en-telling-tweede-kamerverkiezing/ > "$(stad 0597)"
    ../urls-from-html.py https://www.rijssen-holten.nl/uitslag-verkiezingen-tweede-kamer-2023/ > "$(stad 1742)"
    ../urls-from-html.py https://www.rijswijk.nl/voorlopige-uitslag-tweede-kamerverkiezing-2023 > "$(stad 0603)"
    ../urls-from-html.py https://www.roerdalen.nl/uitslagen-verkiezingen > "$(stad 1669)"
    ../urls-from-html.py https://roosendaal.nl/uitslag-tweede-kamerverkiezing "/download" https://roosendaal.nl > "$(stad 1674)"
    ../urls-from-html.py https://tellingen.stembureausinrotterdam.nl/Documenten-TK23 > "$(stad 0599)" ".pdf" https://tellingen.stembureausinrotterdam.nl
    ../urls-from-html.py https://www.rucphen.nl/verkiezingen/uitslag-verkiezingen ".pdf" https://www.rucphen.nl > "$(stad 0840)"

    # S
    ../urls-from-html.py https://www.schagen.nl/tweede-kamerverkiezingen-2023 > "$(stad 0441)"
    ../urls-from-html.py https://www.scherpenzeel.nl/uitslagen-verkiezing-tweede-kamer-2023 > "$(stad 0279)"
    ../urls-from-html.py https://www.schiedam.nl/a-tot-z/uitslagen-bekendmakingen > "$(stad 0606)"
    ../urls-from-html.py https://www.schiermonnikoog.nl/uitslagen-verkiezingen | grep -e tk23 -e 0088_controleprotocol_gsb > "$(stad 0088)"
    ../urls-from-html.py https://www.simpelveld.nl/uitslag-tweede-kamerverkiezingen-2023 > "$(stad 0965)"
    ../urls-from-html.py https://www.sint-michielsgestel.nl/verkiezingen/uitslagen > "$(stad 0845)"
    ../urls-from-html.py https://www.sittard-geleen.nl/Bestuur/Verkiezingen/Tweede_kamerverkiezingen_2023/UItslag_verkiezingen_TK_2023 "objectid" https://www.sittard-geleen.nl > "$(stad 1883)"
    ../urls-from-html.py https://www.sliedrecht.nl/Bestuur_politiek/Verkiezingen/Tweede_Kamerverkiezing/Uitslagen_Tweede_Kamerverkiezing_22_november_2023 ".org" https://www.sliedrecht.nl/ > "$(stad 0610)"
    ../urls-from-html.py https://www.smallingerland.nl/Onderwerpen/Verkiezingen/Uitslag_Tweede_Kamerverkiezingen_2023 "/Uitslag_Tweede_Kamerverkiezingen_2023/" https://www.smallingerland.nl | grep -v osv4_3 > "$(stad 0090)"
    ../urls-from-html.py https://www.soest.nl/verkiezingen/uitslag-verkiezingen ".pdf" https://www.soest.nl > "$(stad 0342)"
    ../urls-from-html.py https://www.someren.nl/raad-en-college/verkiezingen/tweede-kamerverkiezingen-2023/definitieve-uitslag ".pdf" https://www.someren.nl > "$(stad 0847)"
    ../urls-from-html.py https://verkiezingen.sonenbreugel.nl/processen-verbaal-en-bijlagen > "$(stad 0848)"
    ../urls-from-html.py https://www.staphorst.nl/uitslagen-verkiezingen > "$(stad 0180)"
    ../urls-from-html.py https://www.stedebroec.nl/bestuur-en-organisatie/verkiezingen/verkiezingsuitslag/ | sort | uniq > "$(stad 0532)"
    ../urls-from-html.py https://www.steenwijkerland.nl/Over_Steenwijkerland/Verkiezingen/Uitslagen_verkiezingen/Voorlopige_uitslag_Tweede_Kamerverkiezing_2023/Processen_verbaal_van_de_stembureaus/Donderdag_23_november_2023 ".pdf" https://www.steenwijkerland.nl > "$(stad 1708)"
    ../urls-from-html.py https://www.steenwijkerland.nl/Over_Steenwijkerland/Verkiezingen/Uitslagen_verkiezingen/Voorlopige_uitslag_Tweede_Kamerverkiezing_2023/Processen_verbaal_van_de_stembureaus/Woensdag_22_november_2023 ".pdf" https://www.steenwijkerland.nl >> "$(stad 1708)"
    ../urls-from-html.py https://www.gemeentestein.nl/uitslag-tweede-kamerverkiezing-2023 ".pdf" https://www.gemeentestein.nl > "$(stad 0971)"
    ../urls-from-html.py https://stichtsevecht.nl/onderwerp/8115/uitslagen-verkiezingen-tweede-kamer-2023-en-processen-verbaal/ > "$(stad 1904)"
    echo https://verkiezingen.sudwestfryslan.nl/TK2023/pv/Na_31-2_GSB.pdf > "$(stad 1900)"

    # T
    ../urls-from-html.py https://www.texel.nl/bestuur-en-organisatie/tweedekamerverkiezingen/verkiezingsuitslagen-tweede-kamerverkiezingen/ > "$(stad 0448)"
    ../urls-from-html.py https://www.teylingen.nl/verkiezingen/uitslag-tweede-kamerverkiezingen-2023 ".pdf" https://www.teylingen.nl > "$(stad 1525)"
    ../urls-from-html.py https://www.tholen.nl/verkiezingen/uitslag-tweede-kamerverkiezing-2023 ".pdf" https://www.tholen.nl > "$(stad 0716)"
    ../urls-from-html.py https://www.tubbergen.nl/tweede-kamerverkiezingen-2023 "download" https://www.tubbergen.nl | grep -v 4544 | grep -v 4556 | grep -v 4557 | grep -v 4611 | grep -v 4612  | grep -v 4613 | sort | uniq  > "$(stad 0183)"
    ../urls-from-html.py https://www.twenterand.nl/verkiezingsuitslagen-2023-stembureau ".pdf" > "$(stad 1700)"
    ../urls-from-html.py https://www.tynaarlo.nl/verkiezingen/uitslagen-verkiezingen ".pdf" https://www.tynaarlo.nl > "$(stad 1730)"
    ../urls-from-html.py https://www.t-diel.nl/uitslag-en-inzage-processen-verbaal-van-de-tweede-kamerverkiezing-2023 > "$(stad 0737)"

    # U
    ../urls-from-html.py https://www.uitgeest.nl/verkiezingen ".pdf" https://www.uitgeest.nl > "$(stad 0450)"
    ../urls-from-html.py https://www.uithoorn.nl/Home/Verkiezingen/Definitieve_uitslag_Tweede_Kamerverkiezingen_2023 ".pdf" https://www.uithoorn.nl > "$(stad 0451)"
    ../urls-from-html.py https://www.urk.nl/uitslag-tweede-kamer-verkiezing > "$(stad 0184)"
    ../urls-from-html.py https://www.utrecht.nl/bestuur-en-organisatie/verkiezingen/ > "$(stad 0344)" ".pdf" https://www.utrecht.nl
    ../urls-from-html.py https://www.heuvelrug.nl/uitslag-verkiezingen > "$(stad 1581)"

    # V
    ../urls-from-html.py https://www.vaals.nl/verkiezingen/tweede-kamerverkiezingen-2023/uitslag-tweede-kamerverkiezing-2023 ".pdf" https://www.vaals.nl > "$(stad 0981)"
    ../urls-from-html.py https://www.valkenburg.nl/voor-inwoners-en-ondernemers/verkiezingen/uitslag-verkiezingen-tweedekamer-2023 ".pdf" https://www.valkenburg.nl > "$(stad 0994)"
    ../urls-from-html.py https://www.valkenswaard.nl/uitslag-tweede-kamerverkiezingen-2023 > "$(stad 0858)"
    ../urls-from-html.py https://www.veenendaal.nl/info-over-de-gemeente/tweede-kamerverkiezing-2023/uitslag-tweede-kamerverkiezing-2023 ".pdf" https://www.veenendaal.nl | grep -v docreader > "$(stad 0345)"
    ../urls-from-html.py https://www.veere.nl/uitslag-tweede-kamerverkiezing > "$(stad 0717)"
    ../urls-from-html.py https://www.veldhoven.nl/inwoners-en-ondernemers/verkiezingen/uitslag > "$(stad 0861)"
    ../urls-from-html.py https://www.velsen.nl/publicaties-en-uitslagen ".pdf" https://www.velsen.nl > "$(stad 0453)"
    ../urls-from-html.py https://www.venlo.nl/processen-verbaal-tweede-kamerverkiezing > "$(stad 0983)"
    ../urls-from-html.py https://www.vijfheerenlanden.nl/Bestuur/Overzichtspagina_Bestuur/Verkiezingen/Tweede_Kamerverkiezing_2023/Voorlopige_uitslag_Tweede_Kamerverkiezing_2023 ".pdf" https://www.vijfheerenlanden.nl | uniq > "$(stad 1961)"
    ../urls-from-html.py https://www.vlissingen.nl/tweede-kamerverkiezing-22-november-2023-uitslag-en-proces-verbaal | grep -v docreader > "$(stad 0718)"
    ../urls-from-html.py https://www.voerendaal.nl/over-voerendaal/verkiezingen/uitslag-verkiezingen-2023-tweede-kamer ".pdf" https://www.voerendaal.nl > "$(stad 0986)"
    ../urls-from-html.py https://vught.nl/uitslag-tweede-kamerverkiezing ".pdf" https://vught.nl > "$(stad 0865)"

    # W
    ../urls-from-html.py https://www.waadhoeke.nl/uitslagen ".pdf" https://www.waadhoeke.nl > "$(stad 1949)"
    ../urls-from-html.py https://www.waalre.nl/inwoners-en-ondernemers/tweede-kamerverkiezingen-2023 ".pdf" https://www.waalre.nl > "$(stad 0866)"
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

    # Z
    ../urls-from-html.py https://www.zwolle.nl/uitslagen-verkiezingen > "$(stad 0193)" ".pdf" https://www.zwolle.nl
    ../urls-from-html.py https://zutphen.nl/verkiezingen/uitslagen > "$(stad 0301)" ".pdf" https://zutphen.nl
    ../urls-from-html.py https://www.zoeterwoude.nl/gemeenteraad-en-bestuur/verkiezingen > "$(stad 0638)" ".pdf" https://www.zoeterwoude.nl
    ../urls-from-html.py https://www.zoetermeer.nl/processen-verbaal-tellingen-tweede-kamerverkiezing-2023  > "$(stad 0637)"
    ../urls-from-html.py https://www.zevenaar.nl/voorlopige-uitslag-verkiezingen-tweede-kamer > "$(stad 0299)" ".pdf" https://www.zevenaar.nl
    ../urls-from-html.py https://www.zeist.nl/gemeente-bestuur-en-organisatie/verkiezingen/uitslag-verkiezingen ".pdf" https://www.zeist.nl | grep -v docreader > "$(stad 0355)"
    ../urls-from-html.py https://www.zeewolde.nl/verkiezingen/uitslagen ".pdf" https://www.zeewolde.nl > "$(stad 0050)"
    ../urls-from-html.py https://www.zaltbommel.nl/inwoner-en-ondernemer/tweede-kamer-verkiezingen-2023/uitslag-verkiezingen-tweede-kamer/uitslag-verkiezingen-tweede-kamer ".pdf" https://www.zaltbommel.nl > "$(stad 0297)"
popd
