# Overlevering mellom økter

Sist oppdatert 4. oktober 2026, etter M2.5 (komplett utkast å vurdere) og navnebyttet til Fo2Album. Les `CLAUDE.md` først, så denne filen, så `docs/PRODUCT.md` og `docs/SCORING.md`.

## Hvor vi er

- **Gren:** `claude/dazzling-johnson-6zkxf3` (bygger på `claude/pho2album-architecture-plan-trvgs9`). Ingen pull request er opprettet; eieren har ikke bedt om det.
- **Ferdig:** M0 (oppsett), M1 (kilder og innlesing), M2 (evalueringsoppsett), M2.5 (komplett utkast å vurdere). Se `docs/ROADMAP.md`.
- **Navn:** Fo2Album (bekreftet av eieren). Appnavn, tekster, dokumenter, nettside (fo2album.no er hovedadressen, pho2album.no videresendes i `website/_redirects`), app-ID `no.fo2album.app` (ny datamappe og nøkkelring), prototypen heter `prototype/fo2album-prototype.html`. Bevisst **ikke** endret: pakkenavnene `p2a-*`, miljøvariablene `P2A_*` og etikettene i krypteringen (`pho2album/database/v1` o.l.; endres de, kan data ikke åpnes).
- **CI:** Grønn til og med M1.5 på Mac og Windows. For M2 (commit `3fca1c9`) ble push-kjøringen avbrutt av en manuell kjøring (`workflow_dispatch`, run 10), som også bygger `p2a` for Mac og Windows som nedlasting (*Artifacts*: `p2a-macos`, `p2a-windows`). **Sjekk at run 10 ble grønn** før du går videre.

## Hva som finnes

| Del | Status |
|---|---|
| `crates/p2a-store` | Kryptert lagring: SQLCipher-database, krypterte miniatyrer, gjenopprettingsnøkkel (28 tegn, Crockford base32), familieprofil (personer, kommentarer), `documents`-tabell (gullsett). Skjema v2 |
| `crates/p2a-ingest` | Skanning, sky-plassholdere, EXIF, datoer (EXIF → filnavn → endringstid), BLAKE3, dekoding (JPEG nedskalert, PNG, WebP, HEIC via `p2a-heic`), miniatyrer, pHash, prototypens kvalitetsmål |
| `crates/p2a-core` | Domenetyper, dubletter (`dedup`), enkel hendelsesinndeling (`events`, 3 t gap), prototypens utvalg (`select::baseline`), parametre (`config`) |
| `crates/p2a-heic` | ImageIO (Mac, testet i CI med ekte HEIC) og WIC (Windows, ikke testet med ekte fil; krever HEIF-utvidelsen) |
| `crates/p2a-eval` | Bilder ut av album-PDF (lopdf), matching mot biblioteket som tåler beskjæring, gullsett, målinger (hendelsesdekning er hovedmålet) |
| `crates/p2a-cli` | `p2a bench`, `demo`, `les-inn`, `eval lag-gullsett/liste/kjor` |
| `crates/p2a-keychain` | Nøkkelring (Mac/Windows), nøkkelfil på Linux (bare utvikling) |
| `apps/desktop` | Tauri 2 + React: velkomst, gjenopprettingsnøkkel, gjenoppretting, kilder (mappevelger, forslag etter samtykke), **Albumutkast** (hovedvisningen: «Lag utkast» med analyse steg for steg, hendelser med sider, med/ikke med og begrunnelser, bytter, «Hvorfor?», «Dette har appen lært», pris i trinn), «Alle bilder» (år, måneder, fremdrift, merknader), «Slett alle data». Miniatyrer via `miniatyr://`-protokollen |
| `crates/p2a-core` (nytt) | `select::draft` (hendelsesstyrt utkast med begrunnelser og sidetak), `layout` (sider per historie: helside, luft, rutenett), `learn` (vekter fra brukerens svar), `RelationKind` (f.eks. fadder) |
| `crates/p2a-store` (skjema v3) | `overrides` (gjeldende valg per bilde og år), `feedback` (logg over valg og svar, alle år), `person_relations` (f.eks. hvem som er fadder for hvem) |
| `crates/p2a-policy` | Tester som feiler hvis en `p2a-*`-pakke får nettverksavhengigheter eller bruker `std::net` |

Ytelse: 10 000 bilder lest inn på 108 s (4 tråder); 12 MP: 20 ms per bilde.

## Eierens føringer (viktigst, alle skrevet inn i SCORING.md og PRODUCT.md)

1. **Komplett utkast først:** appen foreslår hele albumet før brukeren velger noe.
2. **Hver hendelse (treffpunkt/besøk) minst én side,** to når det er mange bilder; maks 4 sider som utgangspunkt, ikke absolutt.
3. **Alle med navn og profilbilde er med minst én gang.**
4. **Helhetsvurdering:** uskarphet diskvalifiserer ikke viktige bilder (eneste bilde av oldemor, nyfødt). Person og situasjon veier mest (SCORING §3.6, §6.1).
5. **Gjenstander er fyll,** aldri helside. Stemningsbilder er små drypp; ekstrem natur kan åpne en historie.
6. **200–300 sider** som normalt; sidetall følger av hendelsene.
7. **Bare bilder som standard;** dato, sidetall og tekst slås på av kunden.
8. **Redigeringsvisning:** albumet kronologisk, med bilder som ikke er med ved siden av, hver med kort begrunnelse.
9. **Personvern:** alt personlig kryptert hos kunden; minimalt hos oss (adresse, ordre, PDF til levering). Gjenopprettingsnøkkel og kryptert sikkerhetskopi.

Nye føringer (4. oktober 2026, fra dåpsalbumet):

10. **«Lag utkast» kjører en grundig analyse** og viser et komplett forslag. «Velg bilder» er ikke første steg; prototypens «Foreslå de beste bildene» er fjernet.
11. **Brukeren vurderer forslaget**: bildene som er med og ikke med, med kort begrunnelse for begge, og bytter der hun er uenig.
12. **Appen lærer forsiktig** ved å spørre hvorfor, og bygger kundeforståelse over tid.
13. **Store historier får stor plass** (grensen på 4 sider er borte). En dåp for eget barn kan få 10+ sider. Brukeren velger selv et mindre album.
14. **Varier oppsettet**: helside uten marg og bilder med luft rundt.
15. **Pris regnes ut automatisk, i trinn** (foreløpig 4 kr per side, trinn på 50 sider), og alternativene vises.
16. **Dåpsmønsteret**: seremonien åpner, fadderne får ett portrett hver (de står ved døpefonten), gjestene i portrettgallerier til slutt. Slike læringer (hvem som er fadder) skal familieprofilen ta med seg.

Føringer etter at eieren prøvde prototypen (4. oktober 2026):

17. Albumnavnet følger året («Familiealbum 2011» for 2011-bildene).
18. Første analysesteg heter «Henter miniatyrbilder og måler kvalitet».
19. Maks ca. 500 bilder per album (foreløpig; eieren spør trykkeriene). Heller mange bilder enn få.
20. For mange «meningsløse» gjenstander: ting uten personer skal bort hvis de ikke er estetisk viktige.
21. Rask opprydding: marker mange bilder, fjern hele dager og sider, ta andre inn.
22. Albumforslaget må komme tydeligere fram (større sider øverst, helskjerm med ett klikk inn og ett ut). Med og ikke med vises like store.
23. Bildemeny i albumet: ta bort, fremhev mer, ta med men demp, endre utsnitt.
24. Merke for bilder som kan være uskarpe; skarphetsmålet bommet.
25. Hele motivet skal være med; forhåndsvisningen viser hele bildet. Enkle rammevalg med symboler (kvadratisk, liggende, to delt vannrett/loddrett, tre, fire kvadratiske/liggende …).
26. Alle bilder kan vises stort med en rask vurdering og en merknad om at det er en forhåndsvisning.

Føringer fra eierens andre runde (4. oktober 2026):

27. **Ingen tak** i første forslag (500-bildersgrensen er fjernet). 500 er en mulig **sidegrense** hos trykkeriet; blir albumet større, foreslår appen å redusere etter at forslaget er sett, og kunden velger hvor mye.
28. Stemningsbilder setter omgivelsene mellom personpresentasjonene, de skal ikke ta to sider på rad. Personer først. Et flott oversiktsbilde kan få en hel side innimellom, men en side med bare ting skal ikke skje i første forslag.
29. Turer uten barn (gutte-/jenteturer, jobbturer, kamerat-/venninneturer): 2–4 sider som myk maks. Portrettside + stemningsbilder rundt. Kjenner appen få av personene, skal alle få et eget bilde (M4). Se etter gruppebilder uten barn (M4).
30. Marker mange sider og si «presenter dette på x sider»; et nytt forslag, ikke nødvendigvis direkte endelig. Rask korrekturlesing og nye revisjoner.
31. «Fornøyd» på sider: beholdes i neste revisjon og forblir merket, helt til siden endres.
32. Barna (M4): finn ut hvem barna er; omtrent lik fordeling av bilder per historie.
33. Forsiden: forslag fra de beste bildene gjennom året, minst to med personer og minst fire oversiktsbilder. Kandidater kan merkes hvor som helst i prosessen. Baksiden fra de samme kandidatene.
34. Kilder og «Lag utkast» på samme side, så det er lett å legge til flere kilder før utkastet lages. Albumutkastet vises først når analysen er startet.

35. Kunden må kunne justere antall sider både ned og opp etter at gjennomkjøringen er gjort; de fleste vil trenge det. Prisene justeres med sideantallet (grunnpris for permen osv.); eieren kommer tilbake til prisene.

36. Foreslå å kombinere 2, 3 og 4 dager til én historie under gjennomgangen.
37. Revideringen må være rask: marker mange bilder, sider og dager samtidig, og utfør med ett trykk (fjern, nedprioriter, opprioriter, fjern sidene og bildene i dem osv.).

38. Startsiden forblir kildesiden etter at en mappe er valgt: samme førstebilde, med «Lagt til» og «Lag utkast» under kortene, så flere kilder kan legges til først. Ingen årstall øverst; året velges i et felt («Album for»/«År», standard året med flest bilder). Albumutkastet viser bare gjennomgangen i sju punkter når den er startet, og deretter utkastet. Gjort i prototypen og appen.
39. Når flere bilder er markert, kan de settes sammen på én side («Sett på én side»): bildene flyttes fra sidene de sto på, tas med om de ikke var med, og samles på en ny side i dagen til det første bildet (over 12 bilder fordeles på flere sider). Siden merkes «Fornøyd» så den beholdes i neste revisjon, og rammevalget åpnes. Kan angres. Gjort i prototypen; i appen kommer det sammen med markering og «Fornøyd» (krever lagring av låste sider).
40. «Egen historie» for markerte bilder: bildene tas ut av dagene sine og blir en egen historie med egne sider, i tidsrekkefølge. Huskes i neste revisjon og kan legges tilbake. Gjort i prototypen.
41. Bilder kan roteres (meny, stor visning og markering). Gjort i prototypen; rotasjonen følger med til trykkfilen.
42. Opprioriter gjør bildet større trinn for trinn: større på siden, større igjen, egen side, til slutt hele siden. «Ta med, men demp» går motsatt vei, til samme størrelse som de andre på siden og så litt mindre. Gjort i prototypen.
43. Høyre panel har «Lagre utkast» og «Gå til bestilling». Lagret utkast hentes fram fra startsiden når de samme bildemappene er valgt («Fortsett på lagret utkast»). Bestillingen viser sider, pris, en sjekkliste og at bare det ferdige albumet sendes. Gjort i prototypen (lagret i nettleseren; i appen i den krypterte databasen).
44. Valgt forside vises tydelig («Valgt nå» under «Forside og bakside» og i panelet), også når den velges fra bildemenyen.
45. Tekst i albumet: forsiden (forslag «Øyeblikk fra <år>», undertittel med navnene i familien), ryggen, første side inne i albumet (tekst om året, med «Foreslå en start») og baksiden. Klikk på forsiden, ryggen, første side eller baksiden går rett til teksten. Gjort i prototypen.
46. Gå videre til ansiktsgjenkjenning og et komplett system (oktober 2026). **M4 er påbegynt og virker i appen:** ansikter finnes lokalt (YuNet + SFace via tract, se ARCHITECTURE.md), lagres kryptert og grupperes i personer; «Hvem er med?» i menyen lar brukeren gi navn og rolle, slå sammen med en person som finnes, ta ut et feil ansikt og si «Ikke viktig». Utvalget bruker ansiktene: personer først, barna (rolle «Barn i familien») får omtrent like mange bilder per historie, en tur over flere dager uten noen av barna blir tur uten barn av seg selv, og portrettsiden får ett bilde per person.

Status 36–37: gjort i prototypen. Kjernen: `DraftHints::merged_events`, `Draft::merge_suggestions`, `Decision::Opp`, `Action::Opp`/`SlaaSammen`. Appen viser ikke forslagene og markeringen ennå.

Status 35: gjort i prototypen, appen og kjernen (`DraftHints::album_pages`: skalaen som gir nærmest ønsket sidetall, 0,2–3,0; flere sider gir plass til en større andel av bildene).

Status 27–34: alt er i **prototypen** (turer krever GPS i bildene; spørsmålet «Var det en tur uten barn?» erstatter ansiktsgjenkjenning til M4). I Rust-kjernen: ingen tak, stemningsbilder bare alene når de er blant årets flotteste, turer slått sammen med GPS, turer uten barn med myk maks og portrettside, «Fornøyd»-sider og «presenter på x sider» (`DraftHints`), og forsideforslag (`select::cover`). I appen: kilder og «Lag utkast» på samme side. Ikke i appen ennå: lagring av «Fornøyd», turtype, x-sider og forsidekandidater (ny tabell i `p2a-store`), og grensesnittet for dem.

Status: 17–26 er gjort i **prototypen**. I Rust-kjernen er algoritmedelen gjort (17–20, 24: personer via hudtoner, ting bare når de er flotte, skarphet på motivet rangert mot året, maks 500, fremhev/demp i oppsettet). Appens grensesnitt har albumnavn, uskarphetsmerke og like store bilder uten beskjæring; **markering av mange, fjern dag/side, bildemeny, rammevalg, helskjerm og stor visning er ikke bygd i appen ennå** (se neste steg).

## Åpne spørsmål til eieren

1. **Sidegrensen** hos trykkeriene (nå antatt 500 sider, `MAX_SIDER` i prototypen). Ingen grense i første forslag; grensen brukes bare til å foreslå en reduksjon.
1b. **Prisene** (eieren kommer tilbake til dem): modellen er grunnpris for permen + pris per side i trinn, med plassholdere 300 kr + 4 kr per side, trinn på 50 sider. Sidetallet kan justeres ned og opp etter gjennomkjøringen (føring 35).
1c. *(Eldre spørsmål)* **Pristrinnene:** eksempelet ditt (350 sider = 1 400 kr, 300 sider = 1 000 kr) passer ikke helt med 4 kr per side (300 sider blir 1 200 kr). Nå er prisen 4 kr per side i trinn på 50 sider. Skal trinnene ha egne priser (f.eks. en fast pris per trinn med rabatt nedover)? Tabellen ligger i `apps/desktop/src/pris.ts` og øverst i prototypen.
2. **Alternativ B for maskinbytte:** ende-til-ende-kryptert kopi hos oss (bryter prinsippet om at bare trykk-PDF forlater maskinen). Anbefalt: vent; bruk lokal kryptert sikkerhetskopi. Se ARCHITECTURE.md, «Identitet, historikk og bytte av maskin».
3. **Minimum OS** (macOS 12, Windows 10 22H2/11) er arbeidshypotese, ikke bekreftet.

## Neste steg

0a. **Bygg prototypens nye utkastskjerm i appen** når eieren er fornøyd med flyten: markering av mange, fjern dag/side, bildemeny (fremhev/demp/utsnitt), rammevalg med symboler, helskjerm og stor visning. Kjernen og lagringen støtter allerede fremhev/demp (`Decision`, `Action`). Rammer og utsnitt må lagres i `p2a-store` (ny tabell per side).
0. **Eieren prøver den nye flyten** i prototypen (`prototype/fo2album-prototype.html`, Chrome/Edge): legg til en mappe, trykk «Lag utkast», bytt noen bilder og svar på «Hvorfor?». Hva læringen har fanget opp, lagres bare i nettleseren (`localStorage`, nøkkel `fo2album-laering`).
1. **Eieren kjører evalueringen lokalt** (album-PDF-ene på ca. 260 MB skal ikke lastes opp noe sted): `p2a les-inn`, `p2a eval lag-gullsett --pdf … --aar 2010 --navn familie-2010` for 2006–2010, så `p2a eval kjor --resultater eval/RESULTS.md`. Se `eval/LES-MEG.md`. Be om oppsummeringslinjene (treffprosent) og om kontrollsiden ser riktig ut. Er treffprosenten lav, kan BookSmart-PDF-ene ha bildene lagt inn annerledes (f.eks. hele sider som ett bilde); da trengs et lite utdrag.
2. **M3 (bildekvalitet):** det hendelsesstyrte utkastet finnes nå (`select::draft`), og `p2a eval kjor` måler det ved siden av utgangspunktet (rad «… (utkast)»). Neste: bedre Q_tech og estetikk, og kalibrere `pages_per_sqrt_photo`/`photos_per_page` mot gullsettet (familiens album har ca. 4–5 bilder per side). Kjent svakhet: prototypens skarphetsmål går i metning (straffer uskarphet svakt).
2b. **M4 (personer), resten:** (a) eieren prøver «Hvem er med?» på egne bilder: `p2a demo --data /tmp/p2a-demo --ansikter <mappe med familiebilder>` og `P2A_DATA_DIR=/tmp/p2a-demo pnpm dev`; meld fra om grupper som blander personer eller deler én person i flere. (b) Dåpsmønsteret: fadderne (ved døpefonten) foreslås som `fadder_for`-relasjon og får portrettside hver; gjestene i portrettgalleri til slutt. (c) Sjeldenhetsbonus og dekning (alle navngitte personer med). (d) Små ansikter i gruppebilder (deteksjon i ruter). (e) Øyne åpne/smil. (f) Vise «tur uten barn» som appen har gjettet, med mulighet til å si nei, i utkastskjermen (`adultTripGuess`; svaret lagres som `family_trips`). (g) Juridisk vurdering av treningsdataene bak SFace (se ARCHITECTURE.md).
3. **M1b (Apple Bilder via PhotoKit)** er vedtatt for v1, ikke startet.
4. Kjente mangler: virtualisering av rutenettet ved mange tusen bilder per år; Windows-HEIC og ekte sky-plassholdere er ikke prøvd på ekte maskiner; nøkkelringen er ikke prøvd på ekte Mac/Windows.

## Arbeidsmåte som har fungert

- Små steg, ett commit per delsteg, push etter hvert, sjekk CI på Mac og Windows.
- Før push: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check`, `pnpm lint && pnpm typecheck && pnpm test && pnpm build`.
- Appen kan kjøres i miljøet under Xvfb for skjermbilder (`p2a demo` lager testdata; `P2A_DATA_DIR=…`). På Linux trengs `libwebkit2gtk-4.1-dev` m.fl.
- Plattformkode for Mac/Windows typesjekkes med `cargo check -p p2a-heic --target aarch64-apple-darwin` / `x86_64-pc-windows-msvc`.
- Svar eieren på norsk bokmål. Foreslå plan og få ja før større endringer i poengsettingen.
