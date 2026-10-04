# Arkitektur (forslag – skal vurderes i første økt i Claude Code)

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
