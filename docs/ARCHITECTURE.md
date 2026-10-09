# Arkitektur

## Vedtatte beslutninger (4. oktober 2026)

| Område | Beslutning | Begrunnelse |
|---|---|---|
| App-skall | **Tauri 2** | Små installasjonsfiler, Rust-kjerne for tung bildebehandling, streng kontroll på nettverk (CSP og capabilities) |
| Grensesnitt | **React + TypeScript + Vite** | Størst økosystem; virtualisert rutenett for tusenvis av bilder. Tokens fra `design/tokens.css` brukes direkte |
| Fonter | Følger med appen (`@fontsource`), ikke Google Fonts | Appen gjør ingen nettverkskall. `@import` fra Google fjernes fra `tokens.css` ved bygging |
| PDF | **Lages i Rust (`krilla`)**, ikke via webview-utskrift | Webviewen er WebKit på Mac og Chromium på Windows; webview-utskrift gir ikke kontroll på bleed, PDF/X og fargeprofil. Sideoppsettet beskrives i millimeter i kjernen, og både UI og PDF tegner fra samme beskrivelse |
| Bildedekoding | Plattformens API-er først (ImageIO på Mac, WIC på Windows), Rust `image` for JPEG/PNG/WebP, `libheif` bare som reserve | Unngår HEVC-patent- og LGPL-spørsmål der det går. `libheif` krever juridisk vurdering før bruk |
| Apple Bilder | **PhotoKit-plugin (Swift) i v1**, som eget spor (M1b) rett etter M1 | iCloud Bilder på Mac er ikke en mappe; uten PhotoKit mister vi hovedkilden for Mac-familier med iPhone |
| Minimum OS | macOS 12, Windows 10 22H2 / 11 (arbeidshypotese, ikke bekreftet) | |
| Kryptering | **Fra M1**: familieprofil, ansiktsdata, kommentarer, poeng og miniatyrer krypteres på brukerens maskin | Personopplysninger skal ligge hos brukeren, og kryptering er mye vanskeligere å legge på i etterkant |
| Gjenoppretting | **Gjenopprettingsnøkkel + kryptert sikkerhetskopi** som familien selv oppbevarer | Familieprofilen skal kunne brukes år etter år; uten gjenoppretting går den tapt med maskinen |
| Kunde-ID | Nøkkelpar laget lokalt; den offentlige nøkkelen er kunde-ID. Ingen konto, e-post eller passord hos oss | Minimale opplysninger hos oss (se «Personvern og sikkerhet») |

## Personvern og sikkerhet

### Hos brukeren (kryptert)

- **Hva:** familieprofilen (personer, roller, fødselsdatoer, kommentarer, overstyringer, lærte vekter), ansiktsdata (embeddings, klynger), poeng og forklaringer, og miniatyrer (de er bilder av barna). Selve bildene ligger urørt der de var.
- **Hovednøkkel:** tilfeldig 256-bits nøkkel laget ved første oppstart. Lagres i operativsystemets nøkkelring (Nøkkelring på Mac, Credential Manager/DPAPI på Windows), så brukeren slipper passord til daglig.
- **Gjenoppretting:** ved første oppstart vises en gjenopprettingsnøkkel som familien skriver ned: 28 tegn i grupper på fire (`ABCD-EFGH-…`, Crockford base32, 128 tilfeldige bit pluss kontrollsum som fanger skrivefeil). Valgt fremfor en ordliste fordi den er språknøytral, kort og ikke krever en kvalitetssikret norsk ordliste; formatet er versjonert og kan byttes. Hovednøkkelen pakkes inn med en nøkkel avledet fra gjenopprettingsnøkkelen (HKDF-SHA256; 128 tilfeldige bit trenger ingen treg passord-KDF) og lagres i `nokkel.json`. En kryptert sikkerhetskopi (databasen + `nokkel.json`) kan lagres hvor familien vil, f.eks. i egen Dropbox; den kan bare åpnes med gjenopprettingsnøkkelen.
- **Kryptering (vedtatt i M1):**
  - Databasen: **SQLCipher** (hele filen kryptert, AES-256) via `rusqlite`. Mac bruker Apples CommonCrypto. Windows bygger inn OpenSSL sitt kryptobibliotek (`openssl-sys`, vendored), som bare brukes til AES i SQLCipher. Høynivåpakken `openssl` er fortsatt forbudt.
  - Miniatyrer og nøkkelfil: **XChaCha20-Poly1305** (RustCrypto), med innholdshashen som AAD så filer ikke kan byttes om.
  - Undernøkler avledes fra hovednøkkelen med HKDF-SHA256, én per formål.
  - Testet: ingen klartekst (navn, «SQLite format 3», miniatyrinnhold) finnes i filene på disken (`crates/p2a-store/tests/vault.rs`).
- **«Slett alle data»** sletter databasen, miniatyrene og nøkkelen i nøkkelringen.

### Identitet, historikk og bytte av maskin (forslag)

- **Kunde-ID avledes fra hovednøkkelen** (HKDF → Ed25519-nøkkelpar). Den som har gjenopprettingsnøkkelen, får dermed tilbake både familieprofilen og kunde-ID-en på en ny maskin, og kan se og bestille tidligere album uten konto.
- **Albumhistorikken** (prosjekt: hvilke bilder på hvilken side, tekster, beskjæring; pluss den ferdige trykk-PDF-en) ligger kryptert lokalt og er med i den krypterte sikkerhetskopien.
- **«Bestill flere»** sender PDF-en fra det lokale arkivet på nytt. Mangler PDF-en (ny maskin uten sikkerhetskopi), lages den på nytt fra prosjektet så lenge bildene finnes.
- **Hos oss** finnes bare ordrene (per kunde-ID), som i tabellen under.
- **Åpent valg (alternativ B):** en ende-til-ende-kryptert kopi av familieprofilen og arkivet hos oss, som vi ikke kan lese. Gir enklere maskinbytte uten egen sikkerhetskopi, men bryter med prinsippet om at ingenting annet enn trykk-PDF-en forlater maskinen, og krever derfor eierens beslutning.

### Hos oss (minimalt)

| Opplysning | Hvorfor | Hvor lenge |
|---|---|---|
| Kunde-ID (offentlig nøkkel) | Kjenne igjen en kunde ved ny bestilling eller support uten konto | Så lenge kunden har ordrer hos oss |
| Navn og leveringsadresse | Posten/trykkeriet må vite hvor albumet skal | Til levering og reklamasjonsfrist; ordredata deretter så lenge bokføringsreglene krever |
| Ordre (produkt, pris, betalingsreferanse) | Regnskap og reklamasjon | Etter bokføringsreglene (avklares med regnskapsfører) |
| Trykk-PDF | Trykking. Det mest følsomme vi har: inneholder ansiktene til barna | Slettes når albumet er levert og reklamasjonsfristen er ute. Kryptert under overføring og lagring |

- **Betaling:** Stripe Checkout eller Vipps. Kort og telefonnummer håndteres av dem; vi lagrer bare en referanse og ber ikke om profildata fra Vipps ut over det leveringen krever.
- **Trykkeri:** databehandleravtale med krav om sletting av PDF og adresse.
- **Ansiktsgjenkjenning** skjer bare på familiens maskin (GDPR-unntaket for privat bruk). Vi behandler aldri biometriske data.
- **E-post for leveringsvarsel** er valgfritt og kan gå direkte til transportøren.

### Struktur

```
apps/desktop/          Tauri-app: src-tauri/ (Rust-skall) + src/ (React)
crates/p2a-core/       domenetyper, poengsetting, utvalg, sideoppsett (ren, ingen I/O)
crates/p2a-ingest/     kilder, skanning, EXIF, dekoding, hashing, dubletter
crates/p2a-store/      kryptert lagring (SQLCipher), katalog, familieprofil, miniatyrer, «Slett alle data»
crates/p2a-heic/       HEIC via ImageIO (Mac) og WIC (Windows); eneste pakke med unsafe (FFI)
crates/p2a-cli/        verktøyet `p2a`: bench (ytelse), demo (testdata), les-inn, eval (gullsett og evaluering)
crates/p2a-eval/       evaluering: bilder ut av album-PDF, matching mot biblioteket, gullsett, målinger
crates/p2a-keychain/   nøkkelringen på Mac og Windows (delt av appen og p2a)
crates/p2a-policy/     tester som håndhever prinsippene (ingen nettverk i analysekoden)
scripts/               byggesjekker (ingen eksterne ressurser, røyktest)
```

### Slik håndheves «ingen nettverk»

1. `crates/p2a-policy/tests/no_network.rs` går gjennom avhengighetstreet til alle `p2a-*`-pakker og feiler ved HTTP-klienter, sockets eller TLS (og `tokio` med `net`), og ved bruk av `std::net` i kildekoden. Nye `p2a-*`-pakker dekkes automatisk.
2. `deny.toml` forbyr de samme pakkene i hele workspacet (cargo-deny i CI). Bestilling (M7) får en egen pakke, `p2a-order`, som eneste unntak.
3. Tauri: ingen HTTP-, shell- eller fs-plugins; capabilities er bare `core:default` og `dialog:allow-open` (mappevelgeren). CSP: `default-src 'self'`, `connect-src` bare til IPC, `img-src` bare til `miniatyr:`-protokollen, som dekrypterer miniatyrer i minnet.
4. Grensesnittet: ESLint forbyr `fetch`, `XMLHttpRequest`, `WebSocket` og `EventSource`, og `scripts/check-no-remote.mjs` feiler bygget hvis HTML/CSS/JS refererer til noe på nettet.

## Lisenser

Sjekkes automatisk av `cargo deny check licenses` (tillatte lisenser står i `deny.toml`). Tabellen under gjelder direkte avhengigheter og lisenser som krever en merknad.

| Avhengighet | Bruk | Lisens | Merknad |
|---|---|---|---|
| Tauri 2 (`tauri`, `tauri-build`, `@tauri-apps/api`, `@tauri-apps/cli`) | App-skall | MIT OR Apache-2.0 | |
| React, React DOM | Grensesnitt | MIT | |
| Vite, Vitest, TypeScript, ESLint, Prettier | Utviklingsverktøy (følger ikke med appen) | MIT / Apache-2.0 | |
| serde, serde_json | Serialisering | MIT OR Apache-2.0 | |
| Fraunces (`@fontsource-variable/fraunces`) | Font, overskrifter | SIL OFL 1.1 | Kommersiell bruk tillatt. Lisensteksten må følge med appen (legges i «Om appen»/tredjepartslisenser før distribusjon) |
| Nunito Sans (`@fontsource/nunito-sans`) | Font, UI | SIL OFL 1.1 | Som over |
| `cssparser`, `selectors`, `dtoa-short`, `option-ext` (via Tauri) | Transitive | MPL-2.0 | Svak copyleft på filnivå; greit så lenge vi ikke endrer filene |
| ICU-pakker (via Tauri) | Transitive | Unicode-3.0 | |
| SQLCipher Community Edition (via `rusqlite`/`libsqlite3-sys`, `bundled-sqlcipher`) | Kryptert database | BSD-3-Clause (SQLite selv: public domain) | Lisensteksten må følge med appen. cargo-deny ser ikke denne, fordi koden ligger inne i `libsqlite3-sys` |
| OpenSSL 3 (bare Windows, via `openssl-sys`/`openssl-src`) | Kryptobackend for SQLCipher | Apache-2.0 | Lisensteksten må følge med appen |
| `chacha20poly1305`, `hkdf`, `sha2`, `zeroize`, `getrandom` (RustCrypto) | Kryptering av miniatyrer og nøkkelfil | MIT OR Apache-2.0 | |
| `keyring` | Nøkkelring på Mac og Windows (app-skallet) | MIT OR Apache-2.0 | |
| `image`, `jpeg-decoder` | Dekoding av JPEG, PNG og WebP; miniatyrer | MIT OR Apache-2.0 | |
| `kamadak-exif` | EXIF fra JPEG, HEIC, PNG og WebP | BSD-2-Clause | |
| `objc2-image-io`, `objc2-core-graphics`, `objc2-core-foundation` (bare Mac) | HEIC via ImageIO (`crates/p2a-heic`) | Zlib OR Apache-2.0 OR MIT | Dekoderen følger med macOS |
| `windows` (bare Windows) | HEIC via WIC (`crates/p2a-heic`) | MIT OR Apache-2.0 | Krever «HEIF Image Extensions» og HEVC-støtte fra Microsoft Store; uten dem får HEIC-bilder ingen miniatyr, og appen sier fra |
| `blake3` | Innholdshash | CC0-1.0 OR Apache-2.0 | |
| `lopdf` | Lese bilder ut av album-PDF-er (evaluering, M2) | MIT | |
| `walkdir`, `rayon`, `regex` | Skanning, parallell lesing, datoer i filnavn | MIT OR Apache-2.0 / Unlicense OR MIT | |

Ikke ta inn GPL, LGPL eller AGPL uten en vurdering her først.

---

# Opprinnelig forslag


## Anbefalt stakk

| Del | Forslag | Hvorfor | Alternativ |
|---|---|---|---|
| App-skall | **Tauri 2** (Rust-kjerne + webgrensesnitt) | Små installasjonsfiler for Mac og Windows, rask filhåndtering i Rust, lokal-først passer naturlig | Electron (større, men enklere økosystem) |
| Grensesnitt | TypeScript + React (eller Svelte), CSS-variabler fra `design/tokens.css` | Gjenbruker designsystemet direkte | |
| Lokal database | SQLite (cache for metadata, hash, funksjoner, embeddings, personer, prosjekter) | Robust, én fil, enkel å slette | |
| Bildedekoding | Rust `image` + **libheif** for HEIC | HEIC er avgjørende (iPhone) | Plattformens egne API-er (ImageIO på Mac, WIC på Windows) |
| EXIF | Rust EXIF-bibliotek (f.eks. kamadak-exif) | | exiftool som sidekar (tyngre) |
| Hashing | BLAKE3 (eksakt), pHash/dHash (perceptuell) | | |
| ML-kjøring | **tract** (ONNX i ren Rust, CPU) for ansiktsmodellene. ONNX Runtime kan vurderes senere for tyngre modeller (CLIP) | Ingen eksterne biblioteker å pakke med, ingen nettverk, bygges likt på Mac og Windows | ONNX Runtime (CoreML/DirectML) |
| Geokoding | Offline GeoNames-utdrag for Norden + resten av verden (by-nivå) | Ingen nettverkskall | |
| PDF | **pdf-writer** (MIT/Apache-2.0) i `crates/p2a-print`: trykkfilen lages lokalt fra utkastet. Tekst tegnes som streker fra Fraunces og Nunito Sans (SIL OFL 1.1) med **ttf-parser** (MIT/Apache-2.0), bilder legges inn som JPEG i 300 ppi | Ingen fonter å bygge inn, likt resultat hos alle trykkerier, ingen eksterne biblioteker | Krav fra trykkeriet (CMYK/ICC, omslag med rygg) avgjør resten. Feltene på sidene (rammer, større/mindre, utsnitt) regnes ut ett sted, `p2a_print::layout::page_frames`, og appen viser sidene med de samme feltene, så skjermen og trykket er like |

## Modeller (krever lisenssjekk før bruk)

| Behov | Kandidater å vurdere | Merknad |
|---|---|---|
| Ansiktsdeteksjon | YuNet (OpenCV Zoo), RetinaFace/SCRFD-varianter | **Sjekk lisens for modellvektene.** Noen populære ansiktsmodeller (bl.a. deler av InsightFace) er bare for ikke-kommersiell bruk |
| Ansikts-embeddings | Åpne ArcFace-lignende modeller med kommersiell lisens | Samme lisensadvarsel; ev. trene/finjustere egen |
| Scene/innhold + likhet | CLIP / OpenCLIP (velg vekter med tillatende lisens) | Brukes til tagger, likhet (redundans) og estetikk |
| Estetikk | Lineært hode på CLIP-embeddings (f.eks. LAION-aesthetic-tilnærming) | Kan kalibreres på egne data |
| Øyne åpne / smil | Små klassifikatorer på ansiktsutsnitt, eller landemerker | |

### Valgte modeller

| Behov | Modell | Versjon og sjekksum (SHA-256) | Lisens | Kilde |
|---|---|---|---|---|
| Ansiktsdeteksjon (boks + 5 landemerker) | YuNet | `face_detection_yunet_2023mar.onnx`, `8f2383e4…2552fa4` (232 kB) | MIT (Shiqi Yu) | OpenCV Zoo, `models/face_detection_yunet` |
| Ansiktskjennetegn (128 tall) | SFace (MobileFaceNet) | `face_recognition_sface_2021dec.onnx`, `0ba9fbfa…087c34e79` (38 MB) | Apache-2.0 | OpenCV Zoo, `models/face_recognition_sface` |
| *Mulig senere:* deteksjon og gjenkjenning | Luxand FaceSDK | – | Kommersiell: 12 950 USD (Windows + macOS), 2 500 USD/år for oppdateringer | Vurderes bare når kundebasen bærer det og evalueringen viser behov; krav: helt frakoblet lisensaktivering |

Modellene ligger i `crates/p2a-faces/models/` med lisensfilene, og bygges inn i programmet. Kontrollert mot OpenCVs egen implementasjon (oktober 2026): samme ansikter i samme oppløsning; samme kjennetegn for samme opprettede ansikt (likhet 1,000). Samme person i to bilder gir likhet 0,86–0,99, ulike personer under 0,2; terskelen er 0,363 (OpenCVs anbefaling). Tid: ca. 0,2 s per bilde for deteksjon og 0,06 s per ansikt, på én kjerne.

**Beslutning (6. oktober 2026, eieren):** M4 bruker YuNet (deteksjon) og SFace (kjennetegn) via ONNX Runtime, bak et byttbart grensesnitt. Luxand FaceSDK (tilbud: 12 950 USD for Windows og macOS, deretter 2 500 USD/år for oppdateringer) vurderes først når kundebasen kan bære kostnaden, og bare hvis evalueringen viser at det trengs. Krav ved et eventuelt kjøp: helt frakoblet lisensaktivering. Personer (navn, roller) lagres uavhengig av kjennetegnene, så ansiktene kan analyseres på nytt ved modellbytte.

Status mot beslutningen (6. oktober 2026):
- *Kjøremotor:* koden bruker i dag **tract** (ONNX i ren Rust), ikke ONNX Runtime. Samme modellfiler og samme resultater som OpenCV (se over). Bytte til ONNX Runtime (`ort`-pakken) krever at biblioteket pakkes med appen for Mac og Windows og sjekkes i `cargo deny`. Avklares med eieren før byttet.
- *Byttbart grensesnitt:* gjort (9. oktober 2026). Trekket `p2a_faces::FaceAnalyzer` (`model()`, `faces(bilde)`, `same_person()`) er det resten av appen bruker; `FaceEngine` (YuNet + SFace via tract, modellnavn `yunet-2023mar+sface-2021dec`) er den ene implementasjonen. En annen motor eller leverandør settes inn i `p2a_faces::default_analyzer` med et nytt modellnavn.
- *Personer uavhengig av kjennetegn:* gjort (9. oktober 2026). Personer ligger i `persons`, ansiktene i `faces` med `person_id`, og hvert bilde har `photos.faces_model` (skjema v6). Når modellnavnet endres, analyseres bildene på nytt ved neste innlesing; `Store::put_faces` flytter brukerens valg (person, «ikke viktig», gruppe) over til det nye ansiktet som ligger på samme sted (overlapp minst 0,4, parvis), og bare ansikter fra samme modell grupperes sammen.

**Åpent spørsmål (juridisk):** vektene har tillatende lisenser, men SFace er trent på offentlige ansiktsdatasett (bl.a. avledet av MS-Celeb-1M/CASIA-WebFace) med egne vilkår. Bør vurderes før lansering; alternativet er å trene eller finjustere en egen modell på lisensierte data. Små ansikter (under ca. 1/60 av bildets lengste side) finnes ikke ennå; det kan løses med deteksjon i ruter for gruppebilder.

## Kilder

- **v1:** brukeren peker på mapper. Dropbox, iCloud for Windows/Mac og Google Drive for desktop synkroniserer til lokale mapper; appen kan foreslå de vanlige stiene (med samtykke). Håndter «kun i skyen»-filer (plassholdere som ikke er lastet ned): vis antall, og la brukeren velge å laste dem ned via klienten.
- **Mac:** vurder å lese Apple Bilder-biblioteket via PhotoKit (krever en liten native Swift-plugin for Tauri). Gir tilgang til favoritter, album og iCloud-bilder.
- **Senere:** direkte API-integrasjoner (Dropbox API, Google). Merk at Google har strammet inn tilgangen til Google Foto-biblioteket via API; Google Takeout-eksport kan være en praktisk vei. Avklar dagens regler før arbeid starter.

## Ytelse

- Mål: 10 000 bilder analysert på < 20 min på en vanlig bærbar maskin (CPU).
- Trinnvis: (1) metadata og hash for alt (raskt) → (2) miniatyrer og billige funksjoner → (3) tunge modeller bare på kandidater (etter dubletter og grov filtrering).
- Alt cachet på innholdshash; avbryt og gjenoppta.
- Kjør analyse i bakgrunnstråder; UI skal aldri fryse.
- **Målt (M1, 4. oktober 2026):** innlesing (hash, EXIF, dekoding, miniatyr, pHash, dubletter) av syntetiske 12 MP JPEG-er: **20 ms per bilde med 4 tråder**, dvs. ca. 3,3 min for 10 000 bilder. 2048 × 1536: ca. 9 ms per bilde. JPEG dekodes direkte i 1/4 størrelse (`jpeg-decoder`), som kuttet tiden fra 48 ms. Merk: filene lå i diskbufferen; ekte bibliotek på SSD legger til lesetid (ca. 30 GB for 10 000 bilder à 3 MB). Kjør selv med `cargo run --release -p p2a-cli -- bench --antall 1000 --bredde 4032`.
- **Full kjøring, 10 000 bilder** (1600 × 1200, 4,5 GB, 10 % eksakte kopier): innlesing på **108 s** med 4 tråder, alle 1 000 kopier funnet. Kriteriet for M1 («10 000 bilder skannet») er oppfylt.

## Personvern i koden

- Analysemodulen har ingen nettverkstilgang (test som håndhever dette).
- Ingen telemetri med innhold; eventuelle krasjrapporter er opt-in og uten filstier/bilder.
- «Slett alle data»-knapp som fjerner cache-databasen og miniatyrer.
- Eneste utgående dataflyt: trykk-PDF ved bestilling, til trykkeriets API, med tydelig bekreftelse.

## Trykk

Avklar med trykkeriet: format (21 × 28 cm innbundet er arbeidshypotese), utfallende (bleed, typisk 3 mm), sikkerhetsmarg, fargeprofil (sRGB vs. CMYK/PDF/X), minimum ppi, omslag (rygg-bredde avhenger av sideantall) og innsendingsmetode (API/opplasting).
