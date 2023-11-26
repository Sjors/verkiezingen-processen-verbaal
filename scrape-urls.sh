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
	../urls-from-html.py https://www.bernheze.org/bestuur-en-organisatie/verkiezingen/ | grep -v 3/6/9/1/verslag-controleprotocol-gsb.pdf > "$(stad 1721)"
	../urls-from-html.py https://www.ameland.nl/verkiezing-tweede-kamer-2023 > "$(stad 0060)"
	../urls-from-html.py https://www.achtkarspelen.nl/uitslag-en-processen-verbaal-tweede-kamerverkiezing-2023-in-achtkarspelen | grep -v uitslag > "$(stad 0059)"

    # B
    ../urls-from-html.py https://www.gemeentebest.nl/verkiezingen ".pdf" https://www.gemeentebest.nl > "$(stad 0753)"
    ../urls-from-html.py https://www.gemeentebeek.nl/tweedekamerverkiezingen/uitslagen > "$(stad 0888)"
    ../urls-from-html.py https://www.borne.nl/stemmen-en-uitslagen ".pdf" https://www.borne.nl > "$(stad 0147)"
    ../urls-from-html.py https://www.buren.nl/verkiezingen/ > "$(stad 0214)"
    ../urls-from-html.py https://www.baarn.nl/verkiezingsuitslagen > "$(stad 0308)"
    ../urls-from-html.py https://www.breda.nl/uitslag-verkiezing-tweede-kamer ".pdf" https://www.breda.nl > "$(stad 0758)"
    ../urls-from-html.py https://www.bunnik.nl/verkiezingen ".pdf" https://www.bunnik.nl > "$(stad 0312)"
    ../urls-from-html.py https://www.boekel.nl/bestuur-en-organisatie/verkiezingen/tweede-kamer-verkiezing-22-november-2023/uitslagen/ ".pdf" https://www.boekel.nl > "$(stad 0755)"
    ../urls-from-html.py https://www.boxtel.nl/verkiezingen/uitslagen > "$(stad 0757)"
    ../urls-from-html.py https://www.beesel.nl/verkiezingen ".pdf" https://www.beesel.nl | grep tk-2023 > "$(stad 0889)"
    ../urls-from-html.py https://www.bladel.nl/uitslag-verkiezingen-22-november-2023 | grep -v docreader > "$(stad 1728)"
    ../urls-from-html.py https://www.brummen.nl/inwoner-en-ondernemer/verkiezingsuitslagen-1 ".pdf" https://www.brummen.nl > "$(stad 0213)"
    ../urls-from-html.py https://www.borsele.nl/processen-verbaal-tweede-kamerverkiezingen-2023 > "$(stad 0654)"
    ../urls-from-html.py https://www.blaricum.nl/Bestuur/Tweede_Kamerverkiezing_op_22_november_2023 "dsresource" https://www.blaricum.nl | grep -v 6849b6db-bfc6-4b5f-8d9b-0b664191929d > "$(stad 0376)"
    ../urls-from-html.py https://www.bergeijk.nl/uitslag-tweede-kamerverkiezingen-2023 | grep -v docreader > "$(stad 1724)"
    ../urls-from-html.py https://www.bernheze.org/bestuur-en-organisatie/verkiezingen/ ".pdf" https://www.bernheze.org | grep -v 3/6/9/1/verslag-controleprotocol-gsb.pdf > "$(stad 1721)"
    ../urls-from-html.py https://www.barneveld.nl/over-barneveld/verkiezingen/verkiezing-tweede-kamer-2023/uitslag-verkiezingen ".pdf" https://www.barneveld.nl > "$(stad 0203)"
    ../urls-from-html.py https://www.beuningen.nl/projecten/Tweede_Kamerverkiezingen_2023/Uitslag_verkiezing ".org" https://www.beuningen.nl | grep -v bestuur_en_organisatie > "$(stad 0209)"
    echo "https://www.beuningen.nl/projecten/Tweede_Kamerverkiezingen_2023/Uitslag_verkiezing/0209_Proces_verbaal_GSB_TK23.pdf" >> "$(stad 0209)"
    ../urls-from-html.py https://www.beverwijk.nl/verkiezingen > "$(stad 0375)"
    ../urls-from-html.py https://www.bunschoten.nl/uitslag-verkiezingen > "$(stad 0313)"
    ../urls-from-html.py https://www.bergen-nh.nl/verkiezingen ".pdf" https://www.bergen-nh.nl > "$(stad 0373)"
    ../urls-from-html.py https://www.gemeenteberkelland.nl/verkiezingen/uitslagen/ > "$(stad 1859)"
    ../urls-from-html.py https://www.beekdaelen.nl/uitslagen-tweede-kamer-verkiezingen-22-november-2023 > "$(stad 1954)"
    ../urls-from-html.py https://www.bergen.nl/tweede-kamer-2023 > "$(stad 0893)"
    ../urls-from-html.py https://www.bloemendaal.nl/over-de-gemeente/verkiezingen/uitslagen ".pdf" https://www.bloemendaal.nl > "$(stad 0377)"
    ../urls-from-html.py https://www.barendrecht.nl/actueel/voorlopige-uitslag-tweede-kamerverkiezing/ | sort -u --version-sort > "$(stad 0489)"
    ../urls-from-html.py https://www.bergendal.nl/uitslag-tweede-kamerverkiezing-november-2023 > "$(stad 1945)"
    ../urls-from-html.py https://www.baarle-nassau.nl/verkiezingsuitslagen ".pdf" https://www.baarle-nassau.nl > "$(stad 0744)"
    ../urls-from-html.py https://www.borger-odoorn.nl/voorlopige-uitslag-tweede-kamerverkiezingen-2023 "file" https://www.borger-odoorn.nl | grep -v gemeenteborgerodoorncsv | grep -v controleprotocol-en-telling > "$(stad 1681)"
    ../urls-from-html.py https://www.bergenopzoom.nl/verkiezingen/uitslagen > "$(stad 0748)"
    # Brunssum
    ../urls-from-html.py https://www.brunssum.nl/Verkiezingen/Uitslagen_Tweede_Kamer_verkiezingen_2023/Uitslagen_per_stembureau/01_Gemeentehuis ".pdf" https://www.brunssum.nl | uniq > "$(stad 0899)"
    ../urls-from-html.py https://www.brunssum.nl/Verkiezingen/Uitslagen_Tweede_Kamer_verkiezingen_2023/Uitslagen_per_stembureau/02_Brikke_Oave ".pdf" https://www.brunssum.nl | grep -v controle >> "$(stad 0899)"
    ../urls-from-html.py https://www.brunssum.nl/Verkiezingen/Uitslagen_Tweede_Kamer_verkiezingen_2023/Uitslagen_per_stembureau/03_Sporthal_Rumpen_1 ".pdf" https://www.brunssum.nl | grep -v controle >> "$(stad 0899)"
    ../urls-from-html.py https://www.brunssum.nl/Verkiezingen/Uitslagen_Tweede_Kamer_verkiezingen_2023/Uitslagen_per_stembureau/04_Sporthal_Rumpen_2 ".pdf" https://www.brunssum.nl | grep -v controle >> "$(stad 0899)"
    ../urls-from-html.py https://www.brunssum.nl/Verkiezingen/Uitslagen_Tweede_Kamer_verkiezingen_2023/Uitslagen_per_stembureau/05_BS_De_Caleidoscoop ".pdf" https://www.brunssum.nl | grep -v controle >> "$(stad 0899)"
    ../urls-from-html.py https://www.brunssum.nl/Verkiezingen/Uitslagen_Tweede_Kamer_verkiezingen_2023/Uitslagen_per_stembureau/06_Noorderhuis ".pdf" https://www.brunssum.nl | grep -v controle >> "$(stad 0899)"
    ../urls-from-html.py https://www.brunssum.nl/Verkiezingen/Uitslagen_Tweede_Kamer_verkiezingen_2023/Uitslagen_per_stembureau/07_Microhal_1 ".pdf" https://www.brunssum.nl | grep -v controle >> "$(stad 0899)"
    ../urls-from-html.py https://www.brunssum.nl/Verkiezingen/Uitslagen_Tweede_Kamer_verkiezingen_2023/Uitslagen_per_stembureau/08_Microhal_2 ".pdf" https://www.brunssum.nl | grep -v controle >> "$(stad 0899)"
    ../urls-from-html.py https://www.brunssum.nl/Verkiezingen/Uitslagen_Tweede_Kamer_verkiezingen_2023/Uitslagen_per_stembureau/09_Gymzaal_BMV_Bronsheim_1 ".pdf" https://www.brunssum.nl | grep -v controle >> "$(stad 0899)"
    ../urls-from-html.py https://www.brunssum.nl/Verkiezingen/Uitslagen_Tweede_Kamer_verkiezingen_2023/Uitslagen_per_stembureau/15_MFA_Emma_Nova ".pdf" https://www.brunssum.nl | grep -v controle >> "$(stad 0899)"
    ../urls-from-html.py https://www.brunssum.nl/Verkiezingen/Uitslagen_Tweede_Kamer_verkiezingen_2023/Uitslagen_per_stembureau/17_Mobiel_Stembureau ".pdf" https://www.brunssum.nl | grep -v controle >> "$(stad 0899)"

    # C
    ../urls-from-html.py https://www.coevorden.nl/verkiezingsuitslagen "pdf" https://www.coevorden.nl > "$(stad 0109)"
    ../urls-from-html.py https://www.culemborg.nl/uitslagen "pdf" https://www.culemborg.nl > "$(stad 0216)"
    ../urls-from-html.py https://www.castricum.nl/verkiezingen ".pdf" https://www.castricum.nl > "$(stad 0383)"
    ../urls-from-html.py https://www.cranendonck.nl/uitslag-tweede-kamerverkiezingen-2023 | grep -v docreader > "$(stad 1706)"
    ../urls-from-html.py https://www.capelleaandenijssel.nl/processen-verbaal-verkiezing-tweede-kamer-2023 > "$(stad 0502)"

    # D
    ../urls-from-html.py https://tellingen.stembureausindenhaag.nl/Processen-verbaal-Tweede-Kamerverkiezing-2023 ".pdf" https://tellingen.stembureausindenhaag.nl > "$(stad 0518)"
    ../urls-from-html.py https://tellingen.stembureausindenhaag.nl/Algemeen ".pdf" https://tellingen.stembureausindenhaag.nl >> "$(stad 0518)"
    ../urls-from-html.py https://www.delft.nl/proces-verbaal | grep -v .csv > "$(stad 0503)"
    ../urls-from-html.py https://www.druten.nl/verkiezingen-uitslag ".pdf" https://www.druten.nl > "$(stad 0225)"
    ../urls-from-html.py https://www.duiven.nl/voorlopige-uitslag-verkiezingen-tweede-kamer "pdf" https://www.duiven.nl > "$(stad 0226)"
    ../urls-from-html.py https://www.diemen.nl/Onderwerpen/Verkiezingen/Tweede_kamer_verkiezingen_2023/Definitieve_einduitslag_Tweede_Kamerverkiezingen_2023 ".pdf" https://www.diemen.nl > "$(stad 0384)"
    ../urls-from-html.py https://www.deurne.nl/uitslag-verkiezingen-tweede-kamer-2023 | grep -v docreader | grep -v totaaluitslag > "$(stad 0762)"
    ../urls-from-html.py https://www.dongen.nl/verkiezingen-1 ".pdf" https://www.dongen.nl | grep -v .csv > "$(stad 0766)"
    ../urls-from-html.py https://www.dronten.nl/direct-regelen/burgerzaken/verkiezingen/openbare-kennisgevingen-verkiezingen | grep -e 0303 -e Model-na31-1 -e Verslagcontroleprotocolgemeentelijkstembureau > "$(stad 0303)"
    ../urls-from-html.py https://www.debilt.nl/bestuur-en-organisatie/verkiezingen/voorlopige-uitslagen-tweede-kamerverkiezingen-2023 ".pdf" https://www.debilt.nl | grep -v docreader > "$(stad 0310)"
    ../urls-from-html.py https://www.deventer.nl/verkiezingen/processen-verbaal-verkiezing-22-november-2023 > "$(stad 0150)"
    ../urls-from-html.py https://www.deventer.nl/verkiezingen/processen-verbaal-verkiezing-22-november-2023/uitkomsten-per-stembureau >> "$(stad 0150)"
    ../urls-from-html.py https://www.deventer.nl/verkiezingen/processen-verbaal-verkiezing-22-november-2023/proces-verbaal-van-stembureau >> "$(stad 0150)"
    ../urls-from-html.py https://www.doesburg.nl/proces-verbalen-verkiezingen-2023 > "$(stad 0221)"
    ../urls-from-html.py https://cms.dordrecht.nl/Bestuur/Overzicht_Bestuur/Verkiezingen/Tweede_Kamerverkiezing/Uitslagen_Tweede_Kamerverkiezing_22_november_2023 ".pdf" https://cms.dordrecht.nl > "$(stad 0505)"
    ../urls-from-html.py https://www.dewolden.nl/verkiezingen/bekijken-stemmen-per-stembureau ".pdf" https://www.dewolden.nl > "$(stad 1690)"
    ../urls-from-html.py https://drimmelen.nl/uitslagen-van-de-tweede-kamerverkiezingen-2023 "file" https://drimmelen.nl | grep -v docreader | grep -v 11318/download > "$(stad 1719)"
    ../urls-from-html.py https://www.doetinchem.nl/verkiezingen/uitslag > "$(stad 0222)"
    ../urls-from-html.py https://www.denhelder.nl/Onderwerpen/Actueel/Tweede_Kamerverkiezing_22_november_2023/Uitslagen_Tweede_Kamerverkiezingen "dsresource" https://www.denhelder.nl | grep -v abea78dc-e4ad-4495-a8db-f8ee7987594e > "$(stad 0400)"
    ../urls-from-html.py https://www.dinkelland.nl/tweede-kamerverkiezingen-2023 "file" https://www.dinkelland.nl | grep -v .csv > "$(stad 1774)"
    ../urls-from-html.py https://www.dantumadiel.frl/nieuwsoverzicht-0/definitieve-uitslag-tweede-kamerverkiezingen-2023 > "$(stad 1891)"
    ../urls-from-html.py https://www.dantumadiel.frl/processen-verbaal-verkiezingen-tweede-kamer-2023-0 >> "$(stad 1891)"
    ../urls-from-html.py https://www.drechterland.nl/bestuur-en-organisatie/verkiezingen/verkiezingsuitslag/ | sort -u --version-sort > "$(stad 0498)"
    ../urls-from-html.py https://www.dijkenwaard.nl/verkiezingen/uitslagen ".pdf" https://www.dijkenwaard.nl | sort -u --version-sort > "$(stad 1980)"
    ../urls-from-html.py https://gemeente.derondevenen.nl/Bestuur_en_organisatie/Verkiezingen/Uitslag_verkiezingen ".org" https://gemeente.derondevenen.nl | grep Verkiezingen/Uitslag_verkiezingen/ | grep -v osv4 > "$(stad 0736)"
    ../urls-from-html.py https://www.defryskemarren.nl/verkiezingen/uitslagen/ > "$(stad 1940)"

    # E
    ../urls-from-html.py https://www.eindhoven.nl/bestuur-en-beleid/verkiezing-tweede-kamer-2023/verkiezingsuitslagen > "$(stad 0772)"
    ../urls-from-html.py https://gemeente.emmen.nl/processen-verbaal "file" https://gemeente.emmen.nl | grep -v csv > "$(stad 0114)"
    ../urls-from-html.py https://www.epe.nl/processen-verbaal-tweede-kamerverkiezing-2023 > "$(stad 0232)"
    ../urls-from-html.py https://www.elburg.nl/Bestuur_en_organisatie/Uitslag_Tweede_Kamerverkiezing_2023 "pdf" https://www.elburg.nl > "$(stad 0230)"
    ../urls-from-html.py https://www.elburg.nl/Bestuur_en_organisatie/Uitslag_Tweede_Kamerverkiezing_2023/Processen_verbaal_van_de_stembureaus_model_N10_2 "pdf" https://www.elburg.nl >> "$(stad 0230)"
    ../urls-from-html.py https://www.elburg.nl/Bestuur_en_organisatie/Uitslag_Tweede_Kamerverkiezing_2023/Alle_bijlagen_bij_het_proces_verbaal_gemeentelijk_stembureau_bijlage_1_en_bijlage_2 "pdf" https://www.elburg.nl >> "$(stad 0230)"
    ../urls-from-html.py https://www.ermelo.nl/verkiezingen/uitslagen ".pdf" https://www.ermelo.nl > "$(stad 0233)"
    ../urls-from-html.py https://www.eersel.nl/uitslagen-verkiezingen-22-november-2023 | grep -v docreader > "$(stad 0770)"
    ../urls-from-html.py https://www.enkhuizen.nl/bestuur-en-organisatie/verkiezingen/verkiezingsuitslag/ | sort -u --version-sort > "$(stad 0388)"
    ../urls-from-html.py https://www.eemsdelta.nl/uitslag-tweede-kamerverkiezing-2023 > "$(stad 1979)"
    ../urls-from-html.py https://www.etten-leur.nl/uitslagen-tweede-kamerverkiezing/ > "$(stad 0777)"
    ../urls-from-html.py https://www.edam-volendam.nl/uitslagen-tweede-kamerverkiezingen-2023 | grep -v Totaaluitslag > "$(stad 0385)"
    ../urls-from-html.py https://www.echt-susteren.nl/verkiezingsuitslag | grep -v docreader > "$(stad 1711)"
    ../urls-from-html.py https://www.eijsden-margraten.nl/verkiezingen ".pdf" https://www.eijsden-margraten.nl | grep /1903_ > "$(stad 1903)"

    # G
    ../urls-from-html.py https://www.geertruidenberg.nl/uitslagen-van-de-verkiezingen ".pdf" https://www.geertruidenberg.nl > "$(stad 0779)"
    ../urls-from-html.py https://www.geldrop-mierlo.nl/voorlopige-uitslag-tweede-kamerverkiezingen-2023 > "$(stad 1771)"
    ../urls-from-html.py https://www.gemert-bakel.nl/uitslagen-verkiezingen "pdf" https://www.gemert-bakel.nl > "$(stad 1652)"
    ../urls-from-html.py https://www.gennep.nl/uitslag-tweede-kamerverkiezing-2023 > "$(stad 0907)"
    ../urls-from-html.py https://www.gilzerijen.nl/verkiezingsuitslagen ".pdf" https://www.gilzerijen.nl > "$(stad 0784)"
    ../urls-from-html.py https://www.goeree-overflakkee.nl/uitslag-tweede-kamerverkiezing-2023 | grep -v docreader > "$(stad 1924)"
    ../urls-from-html.py https://www.goes.nl/definitieve-uitslag-tweede-kamerverkiezing-2023 > "$(stad 0664)"
    ../urls-from-html.py https://www.goirle.nl/verkiezingen ".pdf" https://www.goirle.nl > "$(stad 0785)"
    ../urls-from-html.py https://gooisemeren.nl/verkiezingen-tweede-kamer-22-november-2023/uitslag-tweede-kamerverkiezing-22-november-2023/ ".pdf" https://gooisemeren.nl > "$(stad 1942)"
    ../urls-from-html.py https://www.gorinchem.nl/tweede-kamerverkiezingen-2023/processen-verbaal-tweede-kamerverkiezingen-2023 > "$(stad 0512)"
    ../urls-from-html.py https://www.gouda.nl/bestuur-en-organisatie/tweede-kamerverkiezingen/processen-verbaal-tweede-kamerverkiezing/ > "$(stad 0513)"
    wget -nv -nd -nc -np -r --remote-encoding=utf-8 --local-encoding=utf-8 --spider --no-parent https://documenten.groningen.nl/TK2023/ 2>&1 | grep "pdf 200 OK" | awk '{print $4}' > "$(stad 0014)"
    ../urls-from-html.py https://www.gulpen-wittem.nl/onderwerpen/verkiezingen/uitslag-verkiezingen "tk23.*pdf" https://www.gulpen-wittem.nl > "$(stad 1729)"
    echo https://www.gulpen-wittem.nl/data/downloadables/1/8/5/8/1729_pv_stembureau_11_wilder-tref_tk23_zh.pdf>> "$(stad 1729)"

    # H
    ../urls-from-html.py https://www.gemeentehw.nl/tweede-kamerverkiezingen-2023/processen-verbaal-en-formulieren/ > "$(stad 1963)"
    ../urls-from-html.py https://www.hoogeveen.nl/verkiezingen/bekijken-stemmen-per-stembureau/ ".pdf" https://www.hoogeveen.nl > "$(stad 0118)"

    # I
    ../urls-from-html.py https://www.ijsselstein.nl/Bestuur_en_organisatie/Verkiezingen/Tweede_Kamerverkiezing_2023/processen_verbaal ".pdf" https://www.ijsselstein.nl > "$(stad 0353)"

    # K
    ../urls-from-html.py https://www.kaagenbraassem.nl/Bestuur_en_organisatie/Verkiezingen/Definitieve_uitslag_Tweede_Kamerverkiezingen dsresource https://www.kaagenbraassem.nl | grep -v pinterest > "$(stad 1884)"
    ../urls-from-html.py  https://www.kampen.nl/tweede-kamerverkiezingen ".pdf" https://www.kampen.nl > "$(stad 0166)"
    # TODO: Kapelle
    ../urls-from-html.py https://www.katwijk.nl/gemeente-en-burgerzaken/verkiezingen/uitslagen ".pdf" https://www.katwijk.nl > "$(stad 0537)"
    ../urls-from-html.py https://www.kerkrade.nl/uitslagen-tweede-kamerverkiezing-2023 > "$(stad 0928)"
    ../urls-from-html.py https://www.koggenland.nl/definitieve-uitslag-tweede-kamerverkiezing-22-november-2023 > "$(stad 1598)"
    ../urls-from-html.py https://krimpenaandenijssel.nl/dossiers/definitieve-uitslag-tweede-kamerverkiezingen-2023/ > "$(stad 0542)"
    ../urls-from-html.py https://www.krimpenerwaard.nl/verkiezingen > "$(stad 1931)"

    # L
    ../urls-from-html.py https://www.laarbeek.nl/uitslagen-tweede-kamer-verkiezingen-2023 file https://www.laarbeek.nl > "$(stad 1659)"
    ../urls-from-html.py https://www.gemeentelandvancuijk.nl/processen-verbaal-tweede-kamerveriezingen-2023 ".pdf" https://www.gemeentelandvancuijk.nl > "$(stad 1982)"
    ../urls-from-html.py https://www.landgraaf.nl/processen-verbaal-verkiezingen > "$(stad 0882)"
    ../urls-from-html.py https://www.landsmeer.nl/nieuws_en_bekendmakingen/tweede_kamer_verkiezingen_2023 bestand https://www.landsmeer.nl > "$(stad 0415)"
    ../urls-from-html.py https://www.lansingerland.nl/politiek-en-organisatie/tweede-kamerverkiezing/processen-verbaal-tweede-kamerverkiezing-2023/ > "$(stad 1621)"
    ../urls-from-html.py https://www.laren.nl/Bestuur/Tweede_Kamerverkiezing_op_22_november_2023 dsresource https://www.laren.nl > "$(stad 0417)"
    ../urls-from-html.py https://gemeente.leiden.nl/bestuur/tweede-kamerverkiezingen-op-22-november-2023/ ".pdf" https://gemeente.leiden.nl > "$(stad 0546)"
    ../urls-from-html.py https://gemeente.leiden.nl/bestuur/tweede-kamerverkiezingen-op-22-november-2023/ ".zip" >> "$(stad 0546)"
    ../urls-from-html.py https://www.leiderdorp.nl/verkiezingen ".pdf" https://www.leiderdorp.nl > "$(stad 0547)"
    ../urls-from-html.py https://www.lv.nl/voorlopige-uitslag-tweede-kamerverkiezing "/file" https://www.lv.nl > "$(stad 1916)"
    ../urls-from-html.py https://www.lelystad.nl/4/verkiezingen/verkiezingsuitslagen/Tweede-Kamerverkiezing-2023.html ".pdf" https://www.lelystad.nl > "$(stad 0995)"
    ../urls-from-html.py https://www.leudal.nl/uitslag-tweede-kamerverkiezing-2023 > "$(stad 1640)"
    ../urls-from-html.py https://www.leusden.nl/verkiezingen/uitslagen/uitslag-tweede-kamerverkiezingen-2023 ".pdf" https://www.leusden.nl > "$(stad 0327)"
    ../urls-from-html.py https://www.lingewaard.nl/uitslag-tweede-kamerverkiezing-2023 > "$(stad 1705)"
    ../urls-from-html.py https://www.lisse.nl/verkiezingen/uitslag-tweede-kamerverkiezingen-2023 ".pdf" https://www.lisse.nl > "$(stad 0553)"
    ../urls-from-html.py https://www.lochem.nl/bestuur-en-organisatie/verkiezingen/uitslagen ".pdf" https://www.lochem.nl > "$(stad 0262)"
    ../urls-from-html.py https://www.loonopzand.nl/organisatie-en-politiek/tweedekamerverkiezingen/verkiezingsuitslag-tweedekamerverkiezing-2023 ".pdf" https://www.loonopzand.nl > "$(stad 0809)"
    ../urls-from-html.py https://www.lopik.nl/uitslag-tweede-kamerverkiezing | uniq > "$(stad 0331)"
    ../urls-from-html.py https://www.leeuwarden.nl/uitslagen-en-processen-verbaal-tweede-kamerverkiezingen-2023/ | sort -u --version-sort

    # M
    ../urls-from-html.py https://www.maasdriel.nl/inwoner-en-ondernemer/verkiezingen/tweede-kamerverkiezingen-2023/uitslagen-per-stembureau-tweede-kamerverkiezingen-2023 ".pdf" https://www.maasdriel.nl > "$(stad 0263)"
    ../urls-from-html.py https://www.gemeentemaasgouw.nl/verkiezingsuitslag > "$(stad 1641)"
    ../urls-from-html.py https://www.maassluis.nl/uitslag-tweede-kamerverkiezingen > "$(stad 0556)"
    ../urls-from-html.py https://www.gemeentemaastricht.nl/verkiezingen/uitslag-tweede-kamerverkiezing-2023 ".pdf" https://www.gemeentemaastricht.nl > "$(stad 0935)"
    ../urls-from-html.py https://www.medemblik.nl/raad-en-bestuur/verkiezingen-tweede-kamer-2023/voorlopige-uitslag-verkiezing-tweede-kamer-2023/alle-uitslagen-per-stembureau-proces-verbalen ".pdf" https://www.medemblik.nl > "$(stad 0420)"
    ../urls-from-html.py https://www.meerssen.nl/nieuws_en_bekendmakingen/actueel/2023/11 "bestand" https://www.meerssen.nl > "$(stad 0938)"
    ../urls-from-html.py https://www.meierijstad.nl/Alle_onderwerpen/Bestuur_en_organisatie/Verkiezingen/Uitslag_verkiezingen ".pdf" https://www.meierijstad.nl > "$(stad 1948)"
    ../urls-from-html.py https://www.meierijstad.nl/Alle_onderwerpen/Bestuur_en_organisatie/Verkiezingen/Uitslag_verkiezingen/De_proces_verbalen ".pdf" https://www.meierijstad.nl >> "$(stad 1948)"
    ../urls-from-html.py https://www.meppel.nl/Inwoner/Burgerzaken/Verkiezingen_2023/Verkiezingsuitslag ".pdf" https://www.meppel.nl > "$(stad 0119)"
    ../urls-from-html.py https://www.middendelfland.nl/uitslag-tweede-kamerverkiezing > "$(stad 1842)"
    ../urls-from-html.py https://www.middendrenthe.nl/verkiezingen-tweede-kamer/uitslag-verkiezingen ".pdf" https://www.middendrenthe.nl > "$(stad 1731)"
    ../urls-from-html.py https://www.midden-groningen.nl/uitslagen > "$(stad 1952)"
    ../urls-from-html.py https://www.moerdijk.nl/web/Verkiezingen/verkiezingen-tweede-kamer-22-november/Aanmelden/Voorlopige-uitslag-verkiezing-Tweede-Kamer-2023.html ".pdf" https://www.moerdijk.nl | grep -v factsheet > "$(stad 1709)"
    ../urls-from-html.py https://www.montferland.info/uitslagen-tweede-kamerverkiezing-2023 > "$(stad 1955)"
    ../urls-from-html.py https://www.montfoort.nl/Alle_onderwerpen/Bestuur_en_organisatie/Verkiezingen/Uitslag_verkiezingen:8_RNTENIQDCNDgDytC9bLA ".pdf" https://www.montfoort.nl > "$(stad 0335)"
    ../urls-from-html.py https://www.mookenmiddelaar.nl/uitslag-van-de-tweede-kamer-verkiezing-22-november-2023-0 | grep -v huisregels > "$(stad 0944)"

    # N
    ../urls-from-html.py https://www.nederbetuwe.nl/verkiezingen "dumpFile" > "$(stad 1740)"
    ../urls-from-html.py https://www.nieuwegein.nl/gemeente-bestuur-en-organisatie/verkiezingen ".pdf" https://www.nieuwegein.nl | grep -v docreader > "$(stad 0356)"
    ../urls-from-html.py https://www.nieuwkoop.nl/verkiezingsuitslagen | grep -v docreader > "$(stad 0569)"
    ../urls-from-html.py https://www.nijkerk.eu/uitslag-verkiezingen-tweede-kamer-22-november-2023 > "$(stad 0267)"
    ../urls-from-html.py https://www.nijmegen.nl/diensten/verkiezingen/uitslagen-verkiezingen/ > "$(stad 0268)"
    echo https://www.noardeast-fryslan.nl/sites/default/files/2023-11/Stembureau%2023%20It%20Koetshus%20Easternijtsjerk%20ZH.pdf > "$(stad 1970)"
    ../urls-from-html.py https://www.noardeast-fryslan.nl/processen-verbaal-verkiezingen-tweede-kamer-2023 >> "$(stad 1970)"
    ../urls-from-html.py https://www.noardeast-fryslan.nl/nieuwsoverzicht/definitieve-uitslagen-tweede-kamerverkiezingen-2023-noardeast-fryslan >> "$(stad 1970)"
    ../urls-from-html.py https://www.noord-beveland.nl/uitslag-tweede-kamerverkiezing-2023 > "$(stad 1695)"
    ../urls-from-html.py https://www.noordenveld.nl/processen-verbaal-tweede-kamer-verkiezing-22-november-2023 > "$(stad 1699)"
    ../urls-from-html.py https://www.noordoostpolder.nl/uitslagen > "$(stad 0171)" ".pdf" https://www.noordoostpolder.nl
    ../urls-from-html.py https://www.noordwijk.nl/onderwerp/burgerzaken/verkiezingen/verkiezingen-informatie-en-uitslagen/processen-verbaal-verkiezingen-tweede-kamer-2023/ | grep -v Bibliotheek > "$(stad 0575)"
    ../urls-from-html.py https://verkiezingen.nuenen.nl/processen-verbaal-en-bijlagen > "$(stad 0820)"
    ../urls-from-html.py https://www.nunspeet.nl/verkiezingen/uitslagen ".pdf" https://www.nunspeet.nl > "$(stad 0302)"

    # O
    ../urls-from-html.py https://www.oegstgeest.nl/bestuur/tweede-kamerverkiezingen-2023/voorlopige-uitslag ".pdf" https://www.oegstgeest.nl > "$(stad 0579)"
    ../urls-from-html.py https://www.oirschot.nl/verkiezingen | grep -v docreader | grep -v ronl > "$(stad 0823)"
    ../urls-from-html.py https://www.oisterwijk.nl/bestuur-en-organisatie/tweede-kamerverkiezingen ".pdf" https://www.oisterwijk.nl > "$(stad 0824)"
    ../urls-from-html.py https://www.gemeente-oldambt.nl/bestanden-verkiezingsuitslag-tweede-kamerverkiezing-oldambt-2023 ".pdf" https://www.gemeente-oldambt.nl > "$(stad 1895)"
    ../urls-from-html.py https://www.oldebroek.nl/Bestuur_en_organisatie/Verkiezingen/Uitslagen_verkiezingen/Processen_verbaal_Tweede_Kamerverkiezingen_2023 ".pdf" https://www.oldebroek.nl > "$(stad 0269)"
    ../urls-from-html.py https://www.oldebroek.nl/Bestuur_en_organisatie/Verkiezingen/Uitslagen_verkiezingen/Bijlagen_2_uitkomsten_per_Stembureau_TK2023 ".pdf" https://www.oldebroek.nl >> "$(stad 0269)"
    ../urls-from-html.py https://www.oldebroek.nl/Bestuur_en_organisatie/Verkiezingen/Uitslagen_verkiezingen "dsresource" https://www.oldebroek.nl >> "$(stad 0269)"
    ../urls-from-html.py https://www.oldenzaal.nl/verkiezingsuitslag ".pdf" https://www.oldenzaal.nl  > "$(stad 0173)"
    ../urls-from-html.py https://www.olst-wijhe.nl/bestuur/verkiezingen/uitslagen-verkiezingen/definitieve-uitslag-tweede-kamer-verkiezingen-2023/controle-centrale-tellingen > "$(stad 1773)"
    ../urls-from-html.py https://www.olst-wijhe.nl/bestuur/verkiezingen/uitslagen-verkiezingen/definitieve-uitslag-tweede-kamer-verkiezingen-2023/proces-verbaal-centrale-tellingen >> "$(stad 1773)"
    ../urls-from-html.py https://www.olst-wijhe.nl/bestuur/verkiezingen/uitslagen-verkiezingen/definitieve-uitslag-tweede-kamer-verkiezingen-2023/proces-verbaal-stembureaus >> "$(stad 1773)"
    ../urls-from-html.py https://www.ommen.nl/tweede-kamerverkiezingen-2023/uitslagen-kennisgevingen/ | sort -u --version-sort > "$(stad 0175)"
    ../urls-from-html.py https://www.oostgelre.nl/uitslagen-tweede-kamerverkiezingen-22-november-2023-oost-gelre ".pdf" https://www.oostgelre.nl > "$(stad 1586)"
    ../urls-from-html.py https://oostzaan.nl/verkiezingen/uitslagen-verkiezingen-tweede-kamer-2023 ".pdf" https://oostzaan.nl > "$(stad 0431)"
    ../urls-from-html.py https://www.opsterland.nl/verkiezingen/uitslagen | grep -v docreader > "$(stad 0086)"
    ../urls-from-html.py https://www.oss.nl/Tonen-op-pagina-standaard/Uitslag-Tweede-Kamer-verkiezingen-2.htm "to2" https://www.oss.nl > "$(stad 0828)"
    ../urls-from-html.py https://www.oude-ijsselstreek.nl/processen-verbaal-n10-n31-en-verslag-controleprotocol > "$(stad 1509)"
    ../urls-from-html.py https://www.ouder-amstel.nl/Home/Verkiezingen/Definitieve_uitslag_Tweede_Kamerverkiezingen_2023 ".pdf" https://www.ouder-amstel.nl > "$(stad 0437)"
    ../urls-from-html.py https://www.oudewater.nl/Verkiezingen/Verkiezingen_2023 ".org" https://www.oudewater.nl > "$(stad 0589)"
    ../urls-from-html.py https://www.overbetuwe.nl/verkiezingen/uitslagen > "$(stad 1734)"
    ../urls-from-html.py https://www.ooststellingwerf.nl/verkiezingen/uitslag > "$(stad 0085)"

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
    ../urls-from-html.py https://www.gemeente-steenbergen.nl/inwoners_overzicht/verkiezingen/tweede_kamerverkiezingen_2023/processen-verbaal/ bestand > "$(stad 0851)"
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
    ../urls-from-html.py https://www.vlieland.nl/uitslag-tweede-kamerverkiezing-2023-in-de-gemeente-vlieland > "$(stad 0096)"
    ../urls-from-html.py https://www.veendam.nl/Onderwerpen/Actueel/Verkiezingen/Processen_verbaal_Tweede_Kamerverkiezingen "pdf" https://www.veendam.nl > "$(stad 0047)"
    ../urls-from-html.py https://www.veendam.nl/Onderwerpen/Actueel/Verkiezingen/Processen_verbaal_Tweede_Kamerverkiezingen "org" https://www.veendam.nl | grep -v Bestuur_en_organisatie | grep -v a0b33a62-4b04-4642-a60b-bae0939acdbe >> "$(stad 0047)"

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
    ../urls-from-html.py https://www.weststellingwerf.nl/uitslag-tweede-kamerverkiezing-2023 > "$(stad 0098)"

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
