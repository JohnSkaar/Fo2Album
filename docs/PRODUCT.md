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
- Ett produkt: **Årets familiealbum** (ett år, ett album). Format: innbundet 21 × 28 cm (endelige formater bestemmes med trykkeri). **Normalt 200–300 sider** (som eierens egne album fra 2006–2010); sidetallet følger av hendelsene. Avklar med trykkeriet hvor mange sider innbindingen tåler.
- Kilder: lokale mapper + de synkroniserte mappene til Dropbox, iCloud for Windows/Mac (iCloud Photos) og Google Drive for desktop. Brukeren legger til så mange mapper hun vil; hver mappe merkes med kilde.
- Filformater: JPEG, HEIC/HEIF, PNG, WebP; ev. RAW senere. Videoer: hoppes over i v1 (ev. stillbilde fra Live Photo senere).
- Dublettfjerning på tvers av kilder (eksakte og nesten like).
- Analyse og poengsetting lokalt, med forklaring per bilde.
- Personer: lokal ansiktsgjenkjenning og gruppering; brukeren navngir de viktigste og kan markere roller (barn, besteforeldre, venner) og «viktig for oss».
- Utkast: forside, kapitler per hendelse (hvert treffpunkt eller besøk minst én side, to når det er mange bilder) samlet i måneder, variert layout (helside, 2, 3, 4 bilder, serie-oppslag), bildetekster med måned/dato og stedsnavn. Sidetallet følger av hendelsene; brukeren kan sette et tak.
- Alle personer med navn og profilbilde er med minst én gang.
- Forsidehjelp (se under).
- Redigering: bytte, flytte, fjerne, legge til, bytte layout per side, beskjære.
- **Redigeringsvisning (eierens føring):** albumet vises slik det er, kronologisk side for side. Ved siden av vises bildene som **ikke** er med (fra samme tid/hendelse), hver med en kort begrunnelse («Nesten likt et bedre bilde», «Uskarpt», «Tre andre bilder fra samme øyeblikk er med»), så det er lett å bytte inn.
- **Tekst på sidene er valgfritt.** Standard er at det **ikke står noe annet enn bildene**. Kunden kan slå på datoer per side, sidetall og bildetekster (sted/hendelse) hver for seg.
- **Helhetsvurdering:** uskarpe bilder utelukkes ikke før appen har vurdert hvor mye personen og situasjonen betyr (se `SCORING.md` §3.6 og §6.1). Uskarpe, men viktige bilder får litt mindre plass.
- **Oppskarping** tilbys for bilder brukeren vil fremheve; aldri automatisk, og originalen endres ikke.
- **Kommentarer på utkastet** (bilde, side, person, hendelse, hele albumet). Strukturerte kommentarer justerer utvalget; fritekst lagres og vises igjen.
- **Familieprofilen:** kryptert på brukerens maskin. Tar vare på personer, roller, fødselsdatoer, kommentarer og det appen har lært, så neste års album starter der dette slapp.
- **Gjenoppretting:** en gjenopprettingsnøkkel og en kryptert sikkerhetskopi som familien selv oppbevarer, så familieprofilen ikke går tapt om maskinen gjør det.
- Eksport til trykkklar PDF og bestilling (kun PDF sendes).

**Ikke med i v1:** kalendere, fotobøker for enkelthendelser, deling/samarbeid, mobilapp, nettbasert redigering, sky-lagring av prosjekter.

## Brukerflyt

1. **Velkommen** → «Velg bildemappene dere vil lage årets familiealbum av». Kort med kilder: PC, Dropbox, iCloud, Google Disk. Hjelpetekst om hvor mappene vanligvis ligger. Appen kan foreslå kjente stier automatisk (med samtykke).
2. **Velg år** (standard: forrige kalenderår, eller året med flest bilder).
3. **Analyse** (kan ta tid; vis fremdrift og hva som skjer: «Fant 8 412 bilder · 1 230 dubletter · leter etter de beste …»). Fortsett i bakgrunnen; kan pauses.
3b. **Komplett utkast først (eierens føring).** Appen foreslår et ferdig album, med alle sider, før brukeren har valgt noe som helst. Brukeren justerer etterpå; hun skal aldri måtte bygge albumet fra et tomt utvalg. Stegene under forbedrer utkastet, de er ikke forutsetninger for det.
4. **Hvem er med?** (forbedrer utkastet; ukjente personer er med som «person 1, 2 …» til de får navn) Vis de 6–12 største ansiktsgruppene. Brukeren navngir og markerer roller. Spør særskilt: «Er det noen som er spesielt viktige å få med, som besteforeldre eller oldeforeldre?»
5. **Forslag**: rutenett per måned med forslaget forhåndsvalgt, poeng og begrunnelse ved hover («Skarp, alle smiler, oldemor er med – 1 av 4 bilder av henne i år»).
6. **Forside** (se under).
7. **Utkast** → redigering og **kommentarer** → **Lagre PDF / Bestill**.
8. **Neste år:** appen åpner med familieprofilen: kjente personer, fjorårets kommentarer og det den har lært. Brukeren bekrefter eller justerer før analysen starter.

## Mine album (historikk og «Bestill flere»)

Forslag, venter på beslutning (se ARCHITECTURE.md, «Identitet, historikk og bytte av maskin»). Eieren vil ha en historikk som hos Blurb: én rad per album med forside, tittel, år, sider, format og dato, og knappene **Forhåndsvis**, **Bestill flere**, **Last ned PDF** og **Slett**. Historikken er knyttet til nøkkelen, så den følger familien ved bytte av PC (gjenopprettingsnøkkel eller kryptert sikkerhetskopi). Det finnes ingen mobilapp i v1; bytte av telefon påvirker bare hvor bildene kommer fra (iCloud og Google beholder dem).

## Personvern: hva vi vet om kunden

Så lite som mulig. Alt som kan være personopplysninger ligger helst på brukerens maskin (kryptert), og bare unntaksvis hos oss.

| Hos brukeren (kryptert) | Hos oss | Hos betalings- og trykkpartner |
|---|---|---|
| Bilder (urørt), miniatyrer, ansiktsdata, personer og roller, kommentarer, poeng, familieprofil | Kunde-ID (offentlig nøkkel, ingen konto, e-post eller passord), ordre, navn og leveringsadresse, trykk-PDF til albumet er levert og reklamasjonsfristen er ute | Stripe/Vipps: betalingsopplysninger (vi får bare en referanse). Trykkeri: PDF og leveringsadresse, under databehandleravtale med krav om sletting |

Ansiktsgjenkjenning skjer bare på familiens egen maskin; vi behandler aldri biometriske data. Detaljer i `ARCHITECTURE.md`, «Personvern og sikkerhet».

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
