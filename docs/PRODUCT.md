# Produkt: Pho2Album v1

## Løftet

> Pek appen til bildene fra 2025, uansett hvor de ligger, og få et ferdig familiealbum med de beste og mest meningsfulle bildene fra året. Bildene blir hjemme.

## Hvorfor dette er unikt

1. **Flere kilder blandes.** Familiens bilder ligger spredt: mammas iPhone (iCloud), pappas Android (Google Disk/Google Foto-eksport), Dropbox-kameraopplasting og mapper på PC-en. Pho2Album leser alle, fjerner dubletter på tvers og ser hele året samlet. Ingen av skytjenestene kan gjøre dette alene.
2. **Smart utvalg.** Ikke bare de skarpeste bildene, men de *viktigste*: variasjon gjennom året, alle i familien representert, sjeldne personer løftet fram (oldemor, en god venn), høydepunkter fått mer plass, serier brukt som et kunstnerisk grep når de er spesielt gode. Se `SCORING.md`.
3. **Lokalt først.** Analysen skjer på maskinen. Ingen sky trenger å se barna.

## Målgruppe

Familier og foreldre (særlig den i familien som «har ansvaret for bildene»), 30–55 år, som har tusenvis av bilder per år og dårlig samvittighet for at de aldri blir trykket. Sekundært: besteforeldre som gave.

## v1-omfang

**Med:**
- Ett produkt: **Årets familiealbum** (ett år, ett album). Format: innbundet 21 × 28 cm (endelige formater bestemmes med trykkeri).
- Kilder: lokale mapper + de synkroniserte mappene til Dropbox, iCloud for Windows/Mac (iCloud Photos) og Google Drive for desktop. Brukeren legger til så mange mapper hun vil; hver mappe merkes med kilde.
- Filformater: JPEG, HEIC/HEIF, PNG, WebP; ev. RAW senere. Videoer: hoppes over i v1 (ev. stillbilde fra Live Photo senere).
- Dublettfjerning på tvers av kilder (eksakte og nesten like).
- Analyse og poengsetting lokalt, med forklaring per bilde.
- Personer: lokal ansiktsgjenkjenning og gruppering; brukeren navngir de viktigste og kan markere roller (barn, besteforeldre, venner) og «viktig for oss».
- Utkast: forside, månedsvise kapitler, variert layout (helside, 2, 3, 4 bilder, serie-oppslag), bildetekster med måned/dato og stedsnavn.
- Forsidehjelp (se under).
- Redigering: bytte, flytte, fjerne, legge til, bytte layout per side, beskjære.
- Eksport til trykkklar PDF og bestilling (kun PDF sendes).

**Ikke med i v1:** kalendere, fotobøker for enkelthendelser, deling/samarbeid, mobilapp, nettbasert redigering, sky-lagring av prosjekter.

## Brukerflyt

1. **Velkommen** → «Velg bildemappene dere vil lage årets familiealbum av». Kort med kilder: PC, Dropbox, iCloud, Google Disk. Hjelpetekst om hvor mappene vanligvis ligger. Appen kan foreslå kjente stier automatisk (med samtykke).
2. **Velg år** (standard: forrige kalenderår, eller året med flest bilder).
3. **Analyse** (kan ta tid; vis fremdrift og hva som skjer: «Fant 8 412 bilder · 1 230 dubletter · leter etter de beste …»). Fortsett i bakgrunnen; kan pauses.
4. **Hvem er med?** Vis de 6–12 største ansiktsgruppene. Brukeren navngir og markerer roller. Spør særskilt: «Er det noen som er spesielt viktige å få med, som besteforeldre eller oldeforeldre?»
5. **Forslag**: rutenett per måned med forslaget forhåndsvalgt, poeng og begrunnelse ved hover («Skarp, alle smiler, oldemor er med – 1 av 4 bilder av henne i år»).
6. **Forside** (se under).
7. **Utkast** → redigering → **Lagre PDF / Bestill**.

## Forsidehjelp

Forsiden er et vanskelig valg. Appen foreslår kandidater i tre kategorier, med **stemningsbilde uten personer som standard** (eierens preferanse):

| Kategori | Hva | Typiske kilder i analysen |
|---|---|---|
| **Stemningsbilde** (standard) | Tidløst bilde uten personer: hytta, havet, fjellet, snø, solnedgang, et dekket bord | Ingen ansikter, høy estetikk, landskap/scene-tagger, fra en hendelse som betydde mye (mange bilder samme dag/sted) |
| **Familiebilde** | Hele eller nesten hele familien samlet | Flest av kjernefamilien, ansikter synlige, øyne åpne, smil |
| **Årets høydepunkt** | Fra årets største hendelse: ferie, bryllup, konfirmasjon, jul | Største hendelsesklynge, tag for reise/fest |

Vis 3–6 kandidater per kategori, plassert på en ekte forside-mal med tittel, så brukeren ser resultatet. Forsiden krever stående eller beskjærbar komposisjon med rolig område til tittelen (se `SCORING.md`, «Forsidepoeng»).

## Suksesskriterier for v1

- Brukeren beholder ≥ 70 % av forslaget uten endringer (målt lokalt, kun aggregat hvis brukeren samtykker).
- Tid fra start til ferdig utkast < 20 min for 10 000 bilder på en vanlig bærbar maskin.
- I brukertester: «Den fant bilder jeg hadde glemt» og «Alle er med».
