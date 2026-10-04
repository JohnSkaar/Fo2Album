# Fo2Album – overlevering til Claude Code

## Kom i gang
1. Pakk ut denne mappen der du vil ha prosjektet, f.eks. `Dokumenter\fo2album`.
2. (Anbefalt) Gjør den til et git-repo: `git init` og første commit, så alle endringer kan spores.
3. Åpne mappen i Claude Code (Claude-desktopappen › Code, eller `claude` i terminalen fra mappen).
4. Lim inn teksten i `FIRST_PROMPT.md` som første melding.

Claude Code leser `CLAUDE.md` automatisk. Den beskriver prinsippene (lokalt først, algoritmen er produktet, norsk tekst) og peker videre til dokumentasjonen i `docs/`.

## Innhold
- `CLAUDE.md` – prosjektkontekst og regler for Claude Code
- `docs/PRODUCT.md` – produkt, v1-omfang (årets familiealbum), kilder, forsidehjelp
- `docs/SCORING.md` – poengsetting og utvalg (kjernen)
- `docs/ARCHITECTURE.md` – teknisk forslag og lisenshensyn
- `docs/ROADMAP.md` – milepæler M0–M8
- `docs/DESIGN.md` – designsystem og UI-regler
- `design/` – tokens (CSS og JSON)
- `prototype/` – klikkbar HTML-prototype
- `website/` – markedssiden for fo2album.no (se `website/LES-MEG.txt`)

## Utvikling

Krever Rust (stable), Node 22 og pnpm 10. På Linux trengs i tillegg Tauri sine systempakker (`libwebkit2gtk-4.1-dev` m.fl.).

```
pnpm install
pnpm dev          # starter appen
```

Se `CLAUDE.md` for alle kommandoer og `docs/ARCHITECTURE.md` for beslutninger og lisenser. CI (`.github/workflows/ci.yml`) bygger, tester og starter appen på macOS og Windows.
