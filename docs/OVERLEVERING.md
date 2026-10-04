# Overlevering mellom økter

Sist oppdatert 4. oktober 2026, etter M2. Les `CLAUDE.md` først, så denne filen, så `docs/PRODUCT.md` og `docs/SCORING.md`.

## Hvor vi er

- **Gren:** `claude/pho2album-architecture-plan-trvgs9` (all kode og alle dokumenter). Ingen pull request er opprettet; eieren har ikke bedt om det.
- **Ferdig:** M0 (oppsett), M1 (kilder og innlesing), M2 (evalueringsoppsett). Se `docs/ROADMAP.md`.
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
| `apps/desktop` | Tauri 2 + React: velkomst, gjenopprettingsnøkkel, gjenoppretting, kilder (mappevelger, forslag etter samtykke), «Velg bilder» (år, måneder, fremdrift, merknader), «Slett alle data». Miniatyrer via `miniatyr://`-protokollen |
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

## Åpne spørsmål til eieren

1. **Navnebytte til Fo2Album?** Eieren synes Fo2Album er bedre på norsk og vil lansere i Norge først. Ikke bekreftet ennå. Hvis ja: appnavn, `tekster.ts`, dokumenter, nettside, domene (fo2album.no som hoved, pho2album.no videresender) og app-ID `no.pho2album.app` → `no.fo2album.app` (endrer datamappe og nøkkelring; billig nå, dyrt senere). `p2a`-verktøyets standard datamappe må følge med (`crates/p2a-cli/src/main.rs`, `data_dir`).
2. **Alternativ B for maskinbytte:** ende-til-ende-kryptert kopi hos oss (bryter prinsippet om at bare trykk-PDF forlater maskinen). Anbefalt: vent; bruk lokal kryptert sikkerhetskopi. Se ARCHITECTURE.md, «Identitet, historikk og bytte av maskin».
3. **Minimum OS** (macOS 12, Windows 10 22H2/11) er arbeidshypotese, ikke bekreftet.

## Neste steg

1. **Eieren kjører evalueringen lokalt** (album-PDF-ene på ca. 260 MB skal ikke lastes opp noe sted): `p2a les-inn`, `p2a eval lag-gullsett --pdf … --aar 2010 --navn familie-2010` for 2006–2010, så `p2a eval kjor --resultater eval/RESULTS.md`. Se `eval/LES-MEG.md`. Be om oppsummeringslinjene (treffprosent) og om kontrollsiden ser riktig ut. Er treffprosenten lav, kan BookSmart-PDF-ene ha bildene lagt inn annerledes (f.eks. hele sider som ett bilde); da trengs et lite utdrag.
2. **M3 (bildekvalitet) og hendelsesstyrt utvalg:** bygg utvalget etter føringene over, ikke «beste per måned». Første rapport i `eval/RESULTS.md` er utgangspunktet å slå. Kjent svakhet: prototypens skarphetsmål går i metning (straffer uskarphet svakt).
3. **M1b (Apple Bilder via PhotoKit)** er vedtatt for v1, ikke startet.
4. Kjente mangler: virtualisering av rutenettet ved mange tusen bilder per år; Windows-HEIC og ekte sky-plassholdere er ikke prøvd på ekte maskiner; nøkkelringen er ikke prøvd på ekte Mac/Windows.

## Arbeidsmåte som har fungert

- Små steg, ett commit per delsteg, push etter hvert, sjekk CI på Mac og Windows.
- Før push: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check`, `pnpm lint && pnpm typecheck && pnpm test && pnpm build`.
- Appen kan kjøres i miljøet under Xvfb for skjermbilder (`p2a demo` lager testdata; `P2A_DATA_DIR=…`). På Linux trengs `libwebkit2gtk-4.1-dev` m.fl.
- Plattformkode for Mac/Windows typesjekkes med `cargo check -p p2a-heic --target aarch64-apple-darwin` / `x86_64-pc-windows-msvc`.
- Svar eieren på norsk bokmål. Foreslå plan og få ja før større endringer i poengsettingen.
