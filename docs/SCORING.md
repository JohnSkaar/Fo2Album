# Poengsetting og utvalg

Dette er kjernen i Pho2Album. Målet er ikke «de 200 skarpeste bildene», men **det albumet familien selv ville laget hvis de hadde tid**: de beste bildene, de viktigste øyeblikkene, alle menneskene som betyr noe, og stor variasjon.

All analyse skjer lokalt. Alle tall under er **startverdier** som skal kalibreres mot evalueringssettet (se nederst). Legg dem i én konfigurasjonsfil (`scoring.config`), ikke spredt i koden.

---

## 1. Pipeline

```
Kilder → Innlesing → Dubletter → Funksjoner per bilde → Hendelser → Personer
      → Betydning → Bildepoeng → Utvalg (optimering) → Sidefordeling/layout → Forside → Forklaringer
```

**Helhetsvurdering først.** Ingen bilder utelukkes på teknisk kvalitet alene før betydningen er regnet ut (§3.6). Et uskarpt bilde av noe som betyr mye (det eneste bildet av oldemor, de første dagene med en nyfødt) skal kunne komme med. Appen sletter aldri bilder; den lar bare være å foreslå dem, og forklarer hvorfor.

Alle steg er inkrementelle og cachet (SQLite) med filens innholdshash som nøkkel, så ny analyse etter at en mappe er lagt til går raskt.

---

## 2. Innlesing og dubletter

**Metadata:** EXIF `DateTimeOriginal` (+ `OffsetTimeOriginal`), GPS, kameramodell, orientering, Live Photo-par, burst-ID (Apple `BurstUUID`) hvis tilgjengelig. Fallback for dato: filnavn (`IMG_20250714_…`, `PXL_2025…`, `WhatsApp Image 2025-07-14 …`), deretter filens endringsdato (markeres som usikker).

**Tre nivåer av dubletter:**
1. **Eksakt:** samme innholdshash (BLAKE3/SHA-256 på bytes). Typisk samme fil i Dropbox og iCloud.
2. **Nesten lik / transkodet:** samme bilde i ulik oppløsning eller format (HEIC i iCloud, JPEG i Google/WhatsApp). Perceptuell hash (pHash/dHash) med Hamming-avstand ≤ 6 **og** opptakstid innen ±2 s (hvis begge har EXIF). Behold versjonen med høyest oppløsning og best metadata.
3. **Nesten-dubletter (serier):** ulike bilder av samme øyeblikk. Håndteres i §5, ikke slettes.

**Støy som filtreres før poengsetting** (ikke slettes, bare utelates fra forslag): skjermbilder (oppløsning = skjerm, ingen EXIF-kamera, filnavn `Screenshot`), dokumenter/kvitteringer/whiteboard (tekst-tetthet høy, scene-tag «document»), memes/videresendte bilder (WhatsApp/Messenger uten kamera-EXIF og lav oppløsning), helt svarte/utbrente bilder, bilder tatt i lomma.

**Ubrukelig** (`Q_tech < 0.15`: helt svart/utbrent, lommebilde, motivet ikke til å kjenne igjen) holdes utenfor forslaget **bare hvis betydningen også er lav** (`B_i < 0.8`, §3.6). Er det eneste bildet av en viktig person eller et spesielt øyeblikk, blir det med som kandidat og merkes for brukeren.

---

## 3. Funksjoner per bilde

Alle funksjoner normaliseres til 0–1. Der det står «modell», velges konkret modell i `ARCHITECTURE.md` (krav: kjører lokalt, lisens tillater kommersiell bruk).

### 3.1 Teknisk kvalitet `Q_tech`
| Signal | Metode | Merknad |
|---|---|---|
| Skarphet | Laplace-varians på motivet (ansiktsboks hvis ansikter finnes, ellers salient region), ikke hele bildet | Bakgrunnsuskarphet (bokeh) skal ikke straffes |
| Bevegelsesuskarphet | Retningsbestemt gradient-analyse / modell | |
| Eksponering | Histogram: klipping i høylys/skygger, middelverdi | Bevisst motlys/silhuett skal ikke straffes hardt hvis estetikk er høy |
| Støy | Estimat i flate områder | |
| Hvitbalanse/fargestikk | Avvik fra nøytral i grå områder | Lav vekt |
| Oppløsning | Nok for planlagt utskriftsstørrelse (≥ 200 ppi for helside, ≥ 150 ppi minimum) | Hard grense per layout, se §7 |

`Q_tech = 0.40·skarp + 0.25·eksponering + 0.15·(1−bevegelse) + 0.10·(1−støy) + 0.10·farge`

### 3.2 Estetikk `Q_aes`
Estetikkmodell (f.eks. CLIP-embedding + lineær estetikk-hode trent på åpne estetikkdatasett) gir 0–1. Suppleres med komposisjon: horisont i vater, motiv nær tredjedelslinjer, ikke kuttede hoder ved kanten, rotete bakgrunn (lav vekt).

### 3.3 Mennesker og øyeblikk `Q_moment` (bare når ansikter finnes)
| Signal | Merknad |
|---|---|
| Øyne åpne | Per ansikt; straff hvis viktig person blunker |
| Smil/latter, uttrykk | Ekte latter > posert smil > nøytral > gråt (gråt kan være søtt for spedbarn; lav straff) |
| Ansikt synlig | Ikke bakhode, ikke dekket, ikke for lite (ansiktshøyde ≥ 4 % av bildehøyden for gruppebilder) |
| Blikk/kontakt | Mot kamera eller mot hverandre |
| Gruppebilde-fullstendighet | Andel av personene i bildet med gode ansikter; et gruppebilde der én blunker får trekk |

`Q_moment = gjennomsnitt over ansikter, vektet med personens viktighet (§4)`

### 3.4 Innhold og type
Scene-tagger via lokal null-skudd-klassifisering (CLIP-lignende) mot en fast norsk/skandinavisk tag-liste: strand, brygge, hytte, fjell, ski, snø, skog, by, fest, bursdag, kake, julebord, juletre, 17. mai, bunad, påske, konfirmasjon, bryllup, skole, idrett, kjæledyr, mat, landskap, solnedgang, detalj …

Avledet **bildetype** (brukes til variasjon i §6 og §7): `portrett`, `par`, `gruppe`, `barn-lek/aksjon`, `landskap/stemning`, `detalj`, `mat`, `dyr`, `sted/bygning`.

### 3.5 Kontekst `C`
- **Hendelse** (§4.2): hvilket øyeblikk tilhører bildet, og hvor viktig er hendelsen.
- **Sted:** GPS → omvendt geokoding **lokalt** (offline database, f.eks. GeoNames) til stedsnavn for bildetekster. Avstand fra «hjem» (det hyppigste overnattingsstedet) indikerer reise.
- **Kalender:** norske høytider og merkedager (nyttår, vinterferie, påske, 17. mai, sankthans, fellesferie, høstferie, advent, jul). Fødselsdager når brukeren har oppgitt dem.

### 3.6 Betydning `B` (regnes ut før kvaliteten vektes)

Betydningen avgjør hvor mye teknisk kvalitet får lov å trekke ned (§6.1). Den har to deler:

- **Personens betydning `P_i`** (§6.1): rolle × sjeldenhet. Den synker når det finnes mange gode bilder av personen.
- **Situasjonens betydning `M_i`** = maks av hendelsens viktighet `E_e(i)` (§4.2) og **spesielle øyeblikk** `Ø_i`:

| Spesielt øyeblikk | Signal | Startverdi `Ø` |
|---|---|---|
| Ny i familien | De første ukene etter at en ny person (spedbarn) dukker opp i bildene, eller etter fødselsdato registrert i familieprofilen | 0.9 de første 14 dagene, deretter avtagende til 0 etter 90 dager |
| Merkedag for en person i bildet | Bursdag (fra familieprofilen), dåp, konfirmasjon, bryllup (scene-tagger og kalender) | 0.8 |
| Uvanlig mye fotografert | Mange bilder på kort tid sammenlignet med familiens vanlige rytme | 0.4–0.7 (skalert) |
| Brukeren har markert det | Favoritt/hjerte, kommentar «viktig» (§6.3), hendelse markert som viktig | 1.0 |

```
M_i = max(E_e(i), Ø_i)
B_i = 1 − (1 − P_i) · (1 − M_i)       // høy hvis enten personen eller situasjonen betyr mye
```

---

## 4. Personer og hendelser

### 4.1 Personer
1. Ansiktsdeteksjon + ansikts-embeddings (lokal modell).
2. Klynging (HDBSCAN eller tilsvarende) per bibliotek; slå sammen klynger over tid (barn endrer seg).
3. Brukeren navngir de største klyngene og setter **rolle**: `kjernefamilie`, `barn`, `besteforeldre/oldeforeldre`, `nær familie`, `venn`, `annen`. Valgfritt flagg: **«spesielt viktig»**.
4. Ukjente ansikter i bakgrunnen ignoreres (lite ansikt, ute av fokus, ikke i sentrum).

**Personvekt `w_p`** (hvor mye en god forekomst av personen er verdt):

| Rolle | Startvekt |
|---|---|
| barn i kjernefamilien | 1.0 |
| kjernefamilie (voksne) | 0.8 |
| besteforeldre / oldeforeldre | 0.9 |
| nær familie | 0.6 |
| venn (navngitt) | 0.5 |
| annen / ukjent | 0.1 |
| «spesielt viktig»-flagg | × 1.5 |

**Sjeldenhetsbonus.** Personer som er med i få bilder i løpet av året, er verdifulle når de faktisk er med. Det er dette som sikrer at oldemor eller en god venn får plass:

```
n_p   = antall gode bilder av person p i året (etter dublett- og seriefiltrering)
rar_p = 1 + α · exp(−n_p / τ)        α = 1.2, τ = 8
```
En person med 3 gode bilder får ≈ ×1.83, en med 40 får ≈ ×1.01.

**Dekning:** hver navngitt person med rolle ≠ annen skal være med i albumet minst `min_p` ganger (barn/kjernefamilie: 6, besteforeldre: 2, nær familie/venn: 1), hvis det finnes et akseptabelt bilde. **Akseptabelt** = `Q_tech ≥ 0.4`, eller for personer med `n_p ≤ 5`: det beste bildet som ikke er ubrukelig (§2). Kjernefamilien skal være rimelig jevnt fordelt (ikke 40 bilder av det ene barnet og 8 av det andre); balanse inngår i målfunksjonen (§6).

### 4.2 Hendelser
1. Sorter bilder på tid. Del i hendelser når tidsgapet > `max(3 t, 2 × median-gap i nabolaget)` **eller** stedet endres > 30 km.
2. Slå sammen påfølgende dager på samme sted til én hendelse (ferie).
3. **Hendelsesviktighet `E_e`** (0–1):
```
E_e = norm( 0.35·log(1 + antall bilder)            // hvor mye ble fotografert
          + 0.20·antall dager
          + 0.15·avstand-fra-hjem (log)
          + 0.15·høytid/merkedag
          + 0.15·andel bilder med flere viktige personer )
```
4. Hendelser får **kapittelplass**: viktige hendelser får flere sider; hverdager samles i månedsoppslag.

---

## 5. Serier og nesten-dubletter

**Serie:** bilder med samme personer/scene innen kort tid (≤ 20 s, eller embedding-likhet ≥ 0.92 innen 3 min).

**Standard:** velg det beste bildet i serien (høyest samlet poeng), demp resten kraftig (`redundans-straff`, §6).

**Kunstnerisk serie (unntak).** En serie kan i stedet bli et **serie-oppslag** (3–6 bilder som forteller en bevegelse: hopp fra brygga, blåse ut lys, første steg) når **alle** er oppfylt:
- seriens beste bilde er i topp 10 % av årets poeng, og median i serien er i topp 25 %
- bildene viser tydelig progresjon (embedding-endring jevnt fordelt, ikke nesten identiske; bevegelse/uttrykk endres)
- maks 1 serie-oppslag per 20 sider, og aldri to på rad
- minst én viktig person eller sterk stemning

Serie-oppslag er sjeldne og skal føles som en gave, ikke som at algoritmen ikke klarte å velge.

---

## 6. Samlet bildepoeng og utvalg

### 6.1 Bildepoeng (grunnverdi)

Person og situasjon får mest vekt. Teknisk kvalitet trekker ned, men mindre jo viktigere bildet er.

```
S_i = Q_tech_i^γ_i · ( 0.35·P_i + 0.30·M_i + 0.20·Q_aes_i + 0.15·Q_moment_i )
γ_i = γ_max − (γ_max − γ_min) · B_i          γ_max = 0.7, γ_min = 0.15
P_i = min(1, Σ_p∈i  w_p · rar_p · ansiktskvalitet_pi) / P_norm
```
- **Kvalitet straffer mindre når betydningen er høy.** Med `Q_tech = 0.3` beholder et vanlig bilde (`B = 0`) 43 % av poengene, mens det eneste bildet av oldemor (`B ≈ 1`) beholder 83 %. Et knivskarpt bilde av en vase slår fortsatt ikke et litt uskarpt bilde av oldemor.
- Bilder uten personer: `P_i = 0`, og vektene for `P` og `Q_moment` flyttes til `Q_aes` (stemningsbilder konkurrerer på estetikk og situasjon): `S_i = Q_tech_i^γ_i · (0.30·M_i + 0.70·Q_aes_i)`.
- Det gamle unntaket for sjeldne personer (`n_p ≤ 3` tillater `Q_tech` ned til 0.3) erstattes av `γ_i`, som gir samme effekt jevnt i stedet for med en terskel.
- Forklaringen skal si det når betydningen har reddet et uskarpt bilde: «Litt uskarpt, men det eneste bildet av oldemor i år».

### 6.2 Utvalg som optimering
Antall bilder `K` bestemmes av antall sider (standard 40 sider ≈ 110–150 bilder, avhengig av layoutmiks). Velg mengden `A` (|A| = K) som maksimerer:

```
F(A) =  Σ_i∈A S_i
      + λ_month · Σ_m  √(antall i A fra måned m)            // jevn dekning av året (avtagende utbytte)
      + λ_event · Σ_e  E_e · √(antall i A fra hendelse e)
      + λ_person· Σ_p  w_p · rar_p · √(antall i A med p)     // alle får være med, avtagende
      + λ_type  · Σ_t  √(antall i A av bildetype t)          // variasjon i typer bilder
      − λ_red   · Σ_i<j∈A  max(0, sim(i,j) − 0.80)           // straff for like bilder (embedding-likhet)
```
Startverdier: `λ_month = 0.6, λ_event = 0.8, λ_person = 1.0, λ_type = 0.4, λ_red = 2.0`.

Kvadratrot-leddene gjør funksjonen submodulær (avtagende utbytte), så **grådig utvelgelse med «lazy evaluation»** gir et nær-optimalt resultat raskt. Deretter:
- **Harde krav:** dekning `min_p` (§4.1), minst 3 bilder per måned med bilder (hvis det finnes ≥ 3 akseptable), maks 25 % fra én enkelt hendelse.
- **Lokal forbedring:** 1–2 runder med bytte (fjern ett, legg til ett) hvis F øker.

### 6.3 Brukerens egne signaler (lokalt)
- Favoritter/hjerter fra Apple Photos/Google (hvis tilgjengelig i metadata) → +0.15 på S.
- Bilder brukeren har redigert/beskåret → +0.05.
- Bilder delt til familien (f.eks. eksportert til WhatsApp-mappe) → svakt signal.
- **Overstyringer i appen:** når brukeren fjerner/legger til bilder, juster vekter lokalt (f.eks. lavere vekt på en bildetype som stadig fjernes). Lagres per bruker, aldri sendt.
- **Kommentarer på utkastet:** brukeren kan kommentere bilder, sider, personer, hendelser og hele albumet. Strukturerte kommentarer («mer av», «mindre av», «viktig», «ikke ta med») justerer vektene direkte. Fritekst lagres og vises igjen, men tolkes ikke i v1.
- **Familieprofilen** (kryptert, lokalt; se `ARCHITECTURE.md`) tar vare på personer, roller, fødselsdatoer, kommentarer, overstyringer og lærte vekter fra år til år. Neste års utvalg starter fra den: fjorårets kommentarer vises, og vektene er allerede justert.

---

## 7. Sidefordeling og layout (sidevekt)

Hvert valgt bilde får en **sidevekt** som avgjør plassen det får:

| Plass | Kriterier (alle startverdier) |
|---|---|
| **Helside** | S i topp 5 % **og** oppløsning holder til helside, **og** minst én av: (a) sjelden viktig person (`n_p ≤ 5`, rolle besteforeldre/oldeforeldre/«spesielt viktig») med godt ansikt; (b) portrett av navngitt venn eller familiemedlem med Q_moment ≥ 0.8; (c) stemnings-/landskapsbilde med Q_aes i topp 3 %; (d) hendelsens «nøkkelbilde» for en hendelse med E ≥ 0.8 |
| **Halv side** (2-bildesmal, stort felt) | S i topp 20 % |
| **Rutenett** (3–4 per side) | Resten, gruppert etter hendelse |
| **Serie-oppslag** | Se §5 |

**Uskarpe, men viktige bilder** får mindre plass, der uskarpheten synes mindre: `Q_tech < 0.5` gir aldri helside, og `Q_tech < 0.35` gir et felt i rutenett. Brukeren kan alltid gjøre bildet større. Senere: regn ut hvor stor uskarpheten blir på trykk (i mm) for feltstørrelsen, i stedet for faste terskler.

**Rytme og variasjon i layout:**
- Maks 1 helside per 4 sider i snitt, aldri to helsider ved siden av hverandre unntatt bevisst «oppslag-par» (to helsider som hører sammen, f.eks. landskap + portrett fra samme sted).
- Ikke samme mal mer enn 2 ganger på rad.
- Bland bildetyper på hvert oppslag: unngå et oppslag med bare gruppebilder; sett gjerne en detalj eller et stemningsbilde inn mellom.
- Kronologi innen kapittel, men tillat små omrokkeringer (± 1 dag) for bedre komposisjon.
- Kapittelstart: hendelser med E ≥ 0.6 får egen åpningsside med tittel (stedsnavn eller hendelse: «Sommer på Hvaler», «Jul hos besteforeldre»), ellers månedsoverskrift.
- Bildetekster: dato og sted fra lokal geokoding; brukeren kan redigere.

**Oppløsningskrav:** effektiv ppi = pikselbredde / feltbredde i tommer. Under 150 ppi: ikke tillatt; 150–200: varsel; ≥ 200: ok. Helside 21 × 28 cm krever ≈ 1650 × 2200 px.

---

## 8. Forsidepoeng

Tre kategorier (se `PRODUCT.md`). Felles krav: horisontalt eller beskjærbart til forsidens format uten å kutte motivet; rolig område (lav kantetetthet) der tittelen skal stå; oppløsning til helside.

```
Cover_mood   = Q_tech · (0.5·Q_aes + 0.2·E_e + 0.2·rolig_tittelområde + 0.1·sesong_symbolikk) · [ingen ansikter > 1 % av bildet]
Cover_family = Q_tech · (0.3·Q_aes + 0.4·andel_kjernefamilie_med_godt_ansikt + 0.2·Q_moment + 0.1·rolig_tittelområde)
Cover_event  = Q_tech · (0.4·Q_aes + 0.4·E_e(årets største) + 0.2·rolig_tittelområde)
```
Vis topp 3–6 per kategori fra ulike hendelser. Standardfanen er **Stemningsbilde**. Valgt forsidebilde trekkes ut av innmaten (eller beholdes hvis det er ekstremt godt, da på en annen layout).

---

## 9. Forklaringer

Hvert foreslått bilde lagrer de 2–3 viktigste grunnene som korte norske setninger, generert fra leddene som bidro mest:
- «Skarp, og alle tre smiler»
- «Oldemor er med, ett av fire bilder av henne i år»
- «Fra sommerferien på Hvaler, årets største hendelse»
- «Beste bilde i en serie på 14»

Forkastede bilder kan også forklares («Nesten likt et bedre bilde», «Uskarpt»). Dette bygger tillit og gjør det lett å overstyre.

---

## 9b. Oppskarping (tilbud, aldri automatisk)

- Tilbys for bilder i albumet med `Q_tech < 0.5`, når brukeren vil fremheve dem.
- Kjøres lokalt. Brukeren ser før og etter og velger selv. Originalen endres aldri; oppskarpingen lagres som en redigering i familieprofilen og brukes bare i trykkfilen.
- v1: klassiske metoder (uskarp maske, dekonvolusjon). En ML-modell for oppskarping kan vurderes senere (lisenssjekk, se `ARCHITECTURE.md`).
- Vær ærlig i grensesnittet: litt uskarphet kan bedres, kraftig bevegelsesuskarphet kan ikke reddes.

---

## 10. Evaluering (må bygges tidlig)

1. **Gullsett:** 3–5 ekte familiebibliotek (med samtykke, kun lokalt hos utvikler) der familien selv har valgt «sitt» album for et år. Pluss syntetiske testsett for enhetstester.
   - **Eierens eget bibliotek:** fasit (ferdige album) for 2006–2010, som brukes som første gullsett og til kalibrering når hovedstrukturen er på plass. **2011** brukes som eierens eget testår uten fasit (bl.a. nyfødtbilder i dårlig lys), for å prøve helhetsvurderingen i §3.6 og §6.1 i praksis.
2. **Metrikker:**
   - Presisjon/recall mot familiens eget utvalg (på hendelsesnivå og bildenivå; tillat «nesten samme bilde» som treff).
   - Dekning: andel navngitte personer over `min_p`; måneder representert.
   - Redundans: snitt av høyeste parvise likhet i utvalget.
   - Variasjon: entropi over bildetyper.
   - **Sjeldne personer:** andel personer med `n_p ≤ 5` som er med (mål: 100 % når et akseptabelt bilde finnes).
   - Beholdt-andel i brukertester (mål ≥ 70 %).
3. **Regresjonstest:** hver endring i vekter kjøres mot gullsettet; resultat logges i `eval/RESULTS.md`.

---

## 11. Parametre (samlet)

| Navn | Start | Beskrivelse |
|---|---|---|
| burst_seconds | 20 | Maks tid mellom bilder i serie |
| near_dup_hamming | 6 | pHash-terskel for transkodede dubletter |
| rarity_alpha / tau | 1.2 / 8 | Sjeldenhetsbonus |
| min_p (barn, kjerne, besteforeldre, venn) | 6, 6, 2, 1 | Dekningskrav |
| λ_month, λ_event, λ_person, λ_type, λ_red | 0.6, 0.8, 1.0, 0.4, 2.0 | Målfunksjon |
| max_event_share | 0.25 | Maks andel fra én hendelse |
| full_page_percentile | 0.05 | Topp-andel som kan få helside |
| series_spread_max | 1 per 20 sider | Kunstneriske serier |
| ppi_min / ppi_ok | 150 / 200 | Oppløsningskrav |
| γ_max / γ_min | 0.7 / 0.15 | Hvor hardt teknisk kvalitet straffer, ved lav og høy betydning (§6.1) |
| unusable_q_tech / unusable_b_max | 0.15 / 0.8 | Ubrukelig holdes ute bare når betydningen er under grensen (§2) |
| newborn_days_full / newborn_days_end | 14 / 90 | «Ny i familien»: full verdi, deretter avtagende (§3.6) |
| blur_no_fullpage / blur_grid_only | 0.5 / 0.35 | Uskarpe bilder får mindre plass (§7) |

Prototypen (`prototype/pho2album-prototype.html`) bruker en svært forenklet versjon (skarphet, eksponering, farge, hudtoner som stedfortreder for personer). Den er bare en referanse for flyten.
