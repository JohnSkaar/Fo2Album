# Forespørsel til trykkerier

Til bruk når eieren henvender seg til trykkerier (12. oktober 2026). Del 1 er e-posten som kan sendes slik den står (fyll inn hakeparentesene). Del 2 er hele spørsmålslisten, som kan legges ved. Del 3 er et skjema for å sammenligne svarene.

Det viktigste for Fo2Album:

1. **API:** bestilling, status og sporing uten manuelt arbeid.
2. **Personvern:** PDF-ene inneholder bilder av barn. Trykkeriet må slette PDF og adresse etter levering, og det må være en databehandleravtale.
3. **Nøytral pakke** (white label): albumet kommer fra Fo2Album, ikke fra trykkeriet.

---

## Del 1: E-posten

**Emne:** Forespørsel om trykk av fotobøker for Fo2Album (API, white label)

Hei,

Jeg heter [Ditt navn] og driver Fo2Album (fo2album.no). Det er en app for Mac og PC som lager årets familiealbum av bildene familien allerede har. Alt arbeid med bildene skjer på kundens egen maskin. Det eneste som forlater maskinen, er den ferdige trykkfilen (PDF) når kunden bestiller.

Vi ser etter et trykkeri som kan produsere og sende fotobøkene direkte til kundene våre i Norge, og vil gjerne høre om dere kan være det. Vi regner med [anslag, f.eks. 200–500] album det første året, mest i oktober–desember, og flere etter hvert.

Det viktigste for oss:

1. **Bestilling via API.** Appen sender ferdig PDF og leveringsadresse. Vi trenger status (mottatt, i produksjon, sendt) og sporingsnummer tilbake, helst som webhook.
2. **Personvern.** Albumene inneholder bilder av barn. Vi trenger en databehandleravtale, og at PDF og adresse slettes når albumet er levert og reklamasjonsfristen er ute. Data bør lagres i EU/EØS.
3. **Nøytral pakke.** Albumet sendes i nøytral emballasje eller med vår logo, uten reklame fra trykkeriet.
4. **Format.** Våre filer er i dag 21 × 28 cm stående med 3 mm utfallende kant, bilder i 300 ppi, sRGB. Vi tilpasser gjerne filene til deres krav (rygg, omslag, fargeprofil).

Kan dere sende oss:

- produktspesifikasjon og filkrav for innbundne fotobøker (format, papir, sideantall, rygg)
- API-dokumentasjon, og om dere har et testmiljø
- priser per album etter sideantall og volum, og pris for frakt i Norge
- produksjons- og leveringstid, også i førjulstiden
- databehandleravtale og hvor lenge dere lagrer filer og adresser
- mulighet for et prøvetrykk av en av våre filer

Jeg legger ved en liste med flere spørsmål, men et første svar på punktene over er mer enn nok til å komme i gang. Jeg ser gjerne et kort møte også.

Vennlig hilsen
[Ditt navn]
Fo2Album · plussdelta@fo2album.no · [Telefon] · fo2album.no

---

### English version (for printers outside Norway)

**Subject:** Photo book printing for Fo2Album (API, white label)

Hello,

I run Fo2Album (fo2album.no), a Mac and PC app that turns a family's own photos into a printed yearly album. All photo processing happens on the customer's computer; the only thing that leaves it is the finished print-ready PDF when the customer orders.

We are looking for a printer that can produce hardcover photo books and ship them directly to our customers in Norway. We expect [estimate, e.g. 200–500] albums in the first year, mostly October–December.

What matters most to us:

1. **Ordering via API:** we send the PDF and the delivery address, and need order status and tracking back (ideally via webhook).
2. **Privacy:** the albums contain photos of children. We need a data processing agreement (GDPR), deletion of PDFs and addresses after delivery and the complaint period, and data stored in the EU/EEA.
3. **White label:** neutral packaging or our branding, no printer marketing inside.
4. **Format:** our files are currently 21 × 28 cm portrait with 3 mm bleed, images at 300 ppi, sRGB. We will adapt to your requirements (spine, cover, colour profile).

Could you send us your product and file specifications, API documentation (and whether you have a sandbox), prices by page count and volume including shipping to Norway, production and delivery times (also before Christmas), your data processing agreement and retention times, and whether you can make a test print of one of our files?

Best regards,
[Your name]
Fo2Album · plussdelta@fo2album.no · [Phone] · fo2album.no

---

## Del 2: Alle spørsmålene

### Produkt

- Hvilke formater har dere for innbundne fotobøker? Vi bruker 21 × 28 cm stående; har dere det eller noe nær (f.eks. A4)?
- Papir (vekt, matt/blank), innbinding (limbundet, lay-flat), omslag (hardt med trykt omslag, tekstil).
- Minimum og maksimum antall sider, og i hvilke trinn sidene må komme (f.eks. delelig med 2 eller 4).
- Kan kunden bestille flere eksemplarer av samme album til én pris per eksemplar?

### Fil og trykk

- Filkrav: PDF-versjon (PDF/X-1a, PDF/X-4?), utfallende kant, sikkerhetsmarg, oppløsning.
- Fargerom: tar dere imot sRGB, eller må vi levere CMYK med en bestemt ICC-profil?
- Omslaget: eget oppslag med rygg? Hvordan regnes ryggbredden ut fra sideantallet (formel eller tabell)?
- Én fil for innmat og én for omslag, eller én samlet fil?
- Største filstørrelse per ordre.
- Sjekker dere filene automatisk, og hva skjer om en fil avvises?

### API og integrasjon

- Har dere et API for ordrer? REST/JSON eller annet? Lenke til dokumentasjon.
- Testmiljø (sandbox) der vi kan sende testordrer uten at de trykkes.
- Opprette ordre: sender vi PDF-en (opplasting), eller henter dere den fra en midlertidig, signert lenke hos oss?
- Status tilbake: webhook eller må vi spørre? Hvilke statuser finnes (mottatt, godkjent, i produksjon, sendt, levert, avvist)?
- Sporingsnummer og transportør tilbake via API.
- Kansellering og endring av adresse før produksjon.
- Autentisering (API-nøkkel, OAuth), og om dere støtter idempotens (samme ordre sendt to ganger gir ikke to album).
- Prisliste og produktkatalog via API, eller bare som avtale?
- Begrensninger: antall ordrer per minutt, filstørrelse, tidsavbrudd ved opplasting.

### Levering og logistikk

- Produksjonstid i normalperioden og i førjulstiden. Siste bestillingsdag for levering før jul.
- Frakt til Norge: transportør, pris, leveringstid, levering til postkasse eller hentested.
- Toll og mva ved levering til Norge fra utlandet (ved trykkeri utenfor Norge): hvem står som avsender og importør?
- White label: nøytral eske, vår logo, eget følgebrev, ingen reklame fra trykkeriet.

### Personvern og sikkerhet

- Databehandleravtale (GDPR art. 28). Kan vi få den til gjennomlesning?
- Hvor lagres og behandles filene og adressene (land)? Hvilke underleverandører brukes?
- Hvor lenge lagres PDF og adresse? Kan de slettes når albumet er levert og reklamasjonsfristen er ute, og bekreftes slettingen?
- Kryptering under overføring og lagring. Hvem hos dere kan se filene?
- Brukes filene til noe annet (f.eks. trening av modeller, markedsføring)? Det må de ikke.

### Kommersielt

- Pris per album etter sideantall, og eventuelle volumrabatter.
- Oppstartskostnad, månedspris eller minimumsvolum.
- Betaling: månedlig faktura, betalingsfrist, valuta.
- Avtaleperiode og oppsigelse.

### Kvalitet og reklamasjon

- Hvordan håndteres trykkfeil og skader i transport? Nytt eksemplar uten kostnad?
- Fargekalibrering og kvalitetskontroll før pakking.
- Prøvetrykk: kan vi få trykket 1–3 album av våre egne filer, og hva koster det?

---

## Del 3: Sammenligne svarene

Gi hvert trykkeri 1–5 per punkt. Personvern og API veier mest. Et trykkeri uten databehandleravtale og sletting kan ikke brukes.

| Punkt | Vekt | Trykkeri A | Trykkeri B | Trykkeri C |
|---|---|---|---|---|
| API (ordre, status via webhook, sporing, testmiljø) | 3 | | | |
| Personvern (databehandleravtale, sletting, EU/EØS) | 3 (krav) | | | |
| Kvalitet (prøvetrykk, papir, farger) | 3 | | | |
| Pris per album inkl. frakt (24, 48, 96 sider) | 2 | | | |
| Leveringstid, også før jul | 2 | | | |
| White label (nøytral pakke, egen logo) | 1 | | | |
| Format og filkrav (nær 21 × 28 cm, sRGB eller enkel CMYK) | 1 | | | |
| Reklamasjon og kundeservice | 1 | | | |

## Hva appen må tilpasses når trykkeriet er valgt

- Omslaget som eget oppslag med rygg etter trykkeriets formel (`p2a-print`).
- Fargeprofil (CMYK/ICC) om trykkeriet krever det.
- Format og sidetrinn, hvis trykkeriet ikke har 21 × 28 cm.
- `p2a-order`: bestilling mot trykkeriets API, med statusoppfølging i adminsiden (`docs/ADMIN.md`).
