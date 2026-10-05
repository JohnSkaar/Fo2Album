# Design

**Kilde:** Figma-filen «Fo2Album Design System» – https://www.figma.com/design/fHbub6TGtXyWNOG9MODR1U

- **Foundations:** farger (primitiver + semantiske, lys og mørk modus), typografi, avstander (4px), hjørner, skygger.
- **Components:** Button (Primary/Secondary/Ghost × Medium/Large × Default/Hover/Disabled), Input, Badge, Avatar, Photo Thumbnail (Default/Selected/Loading), Album Card, Navigation Bar, Icon/* (Home, Album, Plus, User, Check, Upload, Heart, Image, Folder, Lock, Shield, Download).
- **Screens:** «Desktop / Velg bilder» og «Web / Forside». Hovedskjermen i appen er nå **Albumutkast** (hendelser med sider, «Med i albumet» og «Ikke med» med begrunnelser, «Hvorfor?» nederst, pris i trinn); den finnes ikke i Figma ennå. Se `apps/desktop/src/components/DraftScreen.tsx` og prototypen.

**Regler**
- Bruk semantiske tokens (`--color-bg-brand` osv.), aldri hex.
- Maks én Primary-knapp per skjerm.
- Fraunces for overskrifter (følelse), Nunito Sans for UI.
- Låsmerknaden «Bildene blir på denne maskinen» skal være synlig der bilder velges.
- Ikonet Upload brukes bare om innsending av ferdig album til trykk, aldri om bildene.
- Tilgjengelighet: kontrast ≥ 4,5:1, fokusring 2px `--color-border-focus`, trykkflater ≥ 44px, full tastaturstøtte i bilderutenett og albumvisning.

**Referanse for flyt:** `prototype/fo2album-prototype.html` (åpne i Chrome/Edge).

**Logo** (oktober 2026, valgt av eieren blant forslag fra Higgsfield, tegnet som vektor)
- Symbolet: familien sett bakfra ser inn i et åpent fotoalbum sammen med oss; et foto på venstre side og «Fotoalbum» håndskrevet på høyre, et lite salviegrønt hjerte over. Verdien vi skaper er bildene i bokform.
- Ordbildet: «Fo2» i terrakotta (foto), «Album» i mørk brun (boka) og en liten, dempet «.no». Fraunces 600.
- Filer i `design/logo/`: `fo2album-logo.svg` (hele logoen), `fo2album-symbol.svg` (bare symbolet), `fo2album-ikon.svg` (forenklet appikon for 16–1024 px). Teksten er gjort om til streker. Lages på nytt med `python design/logo/lag-logo.py` (krever `fonttools`, `brotli`), som også skriver `apps/desktop/src/components/logoTekst.ts`.
- I appen: `Logo.tsx` med fargene fra `--color-logo-*` i `design/tokens.css`. Appikonene i `src-tauri/icons/` er laget fra `fo2album-ikon.svg` med `pnpm tauri icon`.
- Bruk det forenklede ikonet når logoen er mindre enn ca. 64 px; detaljene i symbolet forsvinner under det.
