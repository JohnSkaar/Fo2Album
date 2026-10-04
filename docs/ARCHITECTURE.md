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

### Struktur

```
apps/desktop/          Tauri-app: src-tauri/ (Rust-skall) + src/ (React)
crates/p2a-core/       domenetyper, poengsetting, utvalg, sideoppsett (ren, ingen I/O)
crates/p2a-ingest/     kilder, skanning, EXIF, dekoding, hashing, dubletter
crates/p2a-store/      SQLite-katalog, miniatyr-cache, «Slett alle data»
crates/p2a-policy/     tester som håndhever prinsippene (ingen nettverk i analysekoden)
scripts/               byggesjekker (ingen eksterne ressurser, røyktest)
```

### Slik håndheves «ingen nettverk»

1. `crates/p2a-policy/tests/no_network.rs` går gjennom avhengighetstreet til alle `p2a-*`-pakker og feiler ved HTTP-klienter, sockets eller TLS (og `tokio` med `net`), og ved bruk av `std::net` i kildekoden. Nye `p2a-*`-pakker dekkes automatisk.
2. `deny.toml` forbyr de samme pakkene i hele workspacet (cargo-deny i CI). Bestilling (M7) får en egen pakke, `p2a-order`, som eneste unntak.
3. Tauri: ingen HTTP-, shell- eller fs-plugins; capabilities er bare `core:default`. CSP: `default-src 'self'`, `connect-src` bare til IPC.
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
| ML-kjøring | **ONNX Runtime** lokalt (CPU; CoreML på Mac, DirectML på Windows når tilgjengelig) | Én motor for alle modeller | |
| Geokoding | Offline GeoNames-utdrag for Norden + resten av verden (by-nivå) | Ingen nettverkskall | |
| PDF | Generer trykkfil lokalt (Rust PDF-bibliotek, eller rendre layout i webview og skrive ut til PDF) | Krav fra trykkeri avgjør (se under) | |

## Modeller (krever lisenssjekk før bruk)

| Behov | Kandidater å vurdere | Merknad |
|---|---|---|
| Ansiktsdeteksjon | YuNet (OpenCV Zoo), RetinaFace/SCRFD-varianter | **Sjekk lisens for modellvektene.** Noen populære ansiktsmodeller (bl.a. deler av InsightFace) er bare for ikke-kommersiell bruk |
| Ansikts-embeddings | Åpne ArcFace-lignende modeller med kommersiell lisens | Samme lisensadvarsel; ev. trene/finjustere egen |
| Scene/innhold + likhet | CLIP / OpenCLIP (velg vekter med tillatende lisens) | Brukes til tagger, likhet (redundans) og estetikk |
| Estetikk | Lineært hode på CLIP-embeddings (f.eks. LAION-aesthetic-tilnærming) | Kan kalibreres på egne data |
| Øyne åpne / smil | Små klassifikatorer på ansiktsutsnitt, eller landemerker | |

Noter valgt modell, versjon, lisens og kilde i en tabell her når det er bestemt.

## Kilder

- **v1:** brukeren peker på mapper. Dropbox, iCloud for Windows/Mac og Google Drive for desktop synkroniserer til lokale mapper; appen kan foreslå de vanlige stiene (med samtykke). Håndter «kun i skyen»-filer (plassholdere som ikke er lastet ned): vis antall, og la brukeren velge å laste dem ned via klienten.
- **Mac:** vurder å lese Apple Bilder-biblioteket via PhotoKit (krever en liten native Swift-plugin for Tauri). Gir tilgang til favoritter, album og iCloud-bilder.
- **Senere:** direkte API-integrasjoner (Dropbox API, Google). Merk at Google har strammet inn tilgangen til Google Foto-biblioteket via API; Google Takeout-eksport kan være en praktisk vei. Avklar dagens regler før arbeid starter.

## Ytelse

- Mål: 10 000 bilder analysert på < 20 min på en vanlig bærbar maskin (CPU).
- Trinnvis: (1) metadata og hash for alt (raskt) → (2) miniatyrer og billige funksjoner → (3) tunge modeller bare på kandidater (etter dubletter og grov filtrering).
- Alt cachet på innholdshash; avbryt og gjenoppta.
- Kjør analyse i bakgrunnstråder; UI skal aldri fryse.

## Personvern i koden

- Analysemodulen har ingen nettverkstilgang (test som håndhever dette).
- Ingen telemetri med innhold; eventuelle krasjrapporter er opt-in og uten filstier/bilder.
- «Slett alle data»-knapp som fjerner cache-databasen og miniatyrer.
- Eneste utgående dataflyt: trykk-PDF ved bestilling, til trykkeriets API, med tydelig bekreftelse.

## Trykk

Avklar med trykkeriet: format (21 × 28 cm innbundet er arbeidshypotese), utfallende (bleed, typisk 3 mm), sikkerhetsmarg, fargeprofil (sRGB vs. CMYK/PDF/X), minimum ppi, omslag (rygg-bredde avhenger av sideantall) og innsendingsmetode (API/opplasting).
