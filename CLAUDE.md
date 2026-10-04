# Fo2Album

Desktop-app for Mac og Windows som lager **årets familiealbum** av bildene familien allerede har, spredt over PC, Dropbox, iCloud og Google Disk. Appen finner de beste og mest meningsfulle bildene, lager et ferdig utkast og sender kun den ferdige trykkfilen til trykk.

Les dette først, deretter **`docs/OVERLEVERING.md`** (status, eierens føringer, åpne spørsmål og neste steg), `docs/PRODUCT.md` og `docs/SCORING.md`.

## Ufravikelige prinsipper

1. **Lokalt først.** Bilder, miniatyrer, analyse, ansiktsgjenkjenning og poengsetting skjer på brukerens maskin. Ingen bilder, embeddings, ansiktsdata eller metadata sendes til noen server. Det eneste som forlater maskinen er den ferdige trykk-PDF-en, og bare når brukeren selv bestiller. Telemetri: av som standard, aldri innhold.
2. **Algoritmen er produktet.** Kvaliteten på utvalget, variasjonen og hvem som får plass er hovedgrunnen til at kunden velger Fo2Album. Alle endringer i poengsettingen skal være forklarbare (hvert bilde har en begrunnelse) og testes mot evalueringssettet (se `docs/SCORING.md`, «Evaluering»).
3. **v1 = ett årsalbum per familie.** Ikke bygg generelle fotoalbum, kalendere eller deling før v1 er god.
4. **Brukeren har siste ord.** Algoritmen foreslår, brukeren bestemmer. Alle valg kan overstyres, og overstyringer brukes som signal.

## Språk og tekst

- All UI-tekst er norsk bokmål, du-form, varm og konkret. Setningsstor bokstav.
- Si aldri «last opp bildene». Si «velg bildemapper», «gi tilgang», «hent bilder fra maskinen».
- Fremhev at trykte bilder samler familien («Minnene blir sterkere når dere blar i dem sammen»). Unngå kamerarull-språk.
- Ved bestilling: si tydelig at bare det ferdige albumet sendes.

## Design

- Tokens: `design/tokens.css` (CSS-variabler, lys og mørk) og `design/tokens.json` (DTCG-format). Bruk alltid semantiske tokens (`--color-bg-brand`, `--color-text-primary` …), aldri hex direkte.
- Fonter: Fraunces (overskrifter), Nunito Sans (UI og tekst).
- Figma (kilde for design): https://www.figma.com/design/fHbub6TGtXyWNOG9MODR1U — sider: Cover, Getting Started, Foundations, Components, Screens.
- Referanseskjermer: Figma › Screens › «Desktop / Velg bilder» og «Web / Forside».
- Se `docs/DESIGN.md` for komponenter og regler.

## Repo-oversikt (startpunkt)

```
CLAUDE.md            ← denne filen
docs/PRODUCT.md      ← produkt, v1-omfang, brukerflyt
docs/SCORING.md      ← poengsetting og utvalgsalgoritme (kjernen)
docs/ARCHITECTURE.md ← teknisk arkitektur og valg
docs/ROADMAP.md      ← milepæler
docs/DESIGN.md       ← designsystem og UI-regler
design/              ← tokens
prototype/           ← klikkbar HTML-prototype (referanse for flyt og UI, ikke kode å bygge videre på)
website/             ← markedsside for fo2album.no (statisk, Netlify)
eval/RESULTS.md      ← logg over evalueringskjøringer
apps/desktop/        ← Tauri-appen: src-tauri/ (Rust) + src/ (React). All UI-tekst i src/tekster.ts
crates/p2a-*/        ← Rust-kjernen: core (domene, dubletter), store (kryptert lagring),
                       ingest (skanning, EXIF, dekoding), heic (ImageIO/WIC), cli (verktøy), policy (tester)
scripts/             ← byggesjekker og røyktest
```

## Kommandoer

```
pnpm install                 # én gang
pnpm dev                     # start appen i utviklingsmodus
pnpm lint && pnpm typecheck && pnpm test && pnpm build
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace       # inkluderer «ingen nettverk»-testen (crates/p2a-policy)
cargo deny check             # lisenser og forbudte pakker
cargo run --release -p p2a-cli -- bench --antall 1000 --bredde 4032   # ytelsesmåling
cargo run -p p2a-cli -- demo --data /tmp/p2a-demo                     # testdata, så:
P2A_DATA_DIR=/tmp/p2a-demo pnpm dev                                   # appen med testdataene
cargo run --release -p p2a-cli -- eval kjor --resultater eval/RESULTS.md # evaluering (se eval/LES-MEG.md)
```

## Arbeidsmåte

- Foreslå plan før større endringer. Små, testbare steg.
- Skriv tester for poengsetting og utvalg (deterministiske, med faste testsett).
- Ingen nettverkskall fra analysekoden. Legg til en test som feiler hvis analysemodulen importerer nettverksbiblioteker.
- Modeller og biblioteker: sjekk lisens (kommersiell bruk) før de tas inn, og noter i `docs/ARCHITECTURE.md`.
