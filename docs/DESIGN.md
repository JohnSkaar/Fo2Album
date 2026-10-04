# Design

**Kilde:** Figma-filen «Pho2Album Design System» – https://www.figma.com/design/fHbub6TGtXyWNOG9MODR1U

- **Foundations:** farger (primitiver + semantiske, lys og mørk modus), typografi, avstander (4px), hjørner, skygger.
- **Components:** Button (Primary/Secondary/Ghost × Medium/Large × Default/Hover/Disabled), Input, Badge, Avatar, Photo Thumbnail (Default/Selected/Loading), Album Card, Navigation Bar, Icon/* (Home, Album, Plus, User, Check, Upload, Heart, Image, Folder, Lock, Shield, Download).
- **Screens:** «Desktop / Velg bilder» (hovedskjermen i appen) og «Web / Forside».

**Regler**
- Bruk semantiske tokens (`--color-bg-brand` osv.), aldri hex.
- Maks én Primary-knapp per skjerm.
- Fraunces for overskrifter (følelse), Nunito Sans for UI.
- Låsmerknaden «Bildene blir på denne maskinen» skal være synlig der bilder velges.
- Ikonet Upload brukes bare om innsending av ferdig album til trykk, aldri om bildene.
- Tilgjengelighet: kontrast ≥ 4,5:1, fokusring 2px `--color-border-focus`, trykkflater ≥ 44px, full tastaturstøtte i bilderutenett og albumvisning.

**Referanse for flyt:** `prototype/pho2album-prototype.html` (åpne i Chrome/Edge).
