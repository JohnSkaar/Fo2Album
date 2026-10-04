# Veikart

Rekkefølgen er valgt slik at algoritmen kan måles tidlig. Hver milepæl ender med noe som kan kjøres.

| # | Milepæl | Innhold | Ferdig når |
|---|---|---|---|
| M0 | Oppsett | Tauri-prosjekt, tokens inn i UI, CI for Mac og Windows, «ingen nettverk i analyse»-test | Tom app bygges og starter på begge plattformer |
| M1 | Kilder og innlesing | Legg til flere mapper med kilde-merking, rask skanning, EXIF/datoer, HEIC, SQLite-cache, eksakte og transkodede dubletter. **Kryptert lagring** (hovednøkkel i nøkkelringen, gjenopprettingsfrase) og datamodell for familieprofilen | 10 000 bilder skannet, dubletter på tvers av Dropbox/iCloud fjernet |
| M1b | Apple Bilder (Mac) | PhotoKit-plugin i Swift for Tauri: tilgang med samtykke, les bilder og metadata (favoritter, album, burst), håndter originaler som bare ligger i iCloud | Et iCloud Bilder-bibliotek på Mac leses inn og blandes med mappekildene, med dubletter fjernet |
| M2 | Evalueringsoppsett | Gullsett-format, metrikker fra SCORING §10, kommandolinjeverktøy som kjører utvalget og skriver rapport. Første gullsett: eierens album for 2006–2010; 2011 som eget testår | Første rapport i `eval/RESULTS.md` |
| M3 | Bildekvalitet | Q_tech, Q_aes, bildetyper, støyfiltre (skjermbilder, kvitteringer), serier. Helhetsvurdering: ubrukelig holdes ute bare ved lav betydning | Utvalg uten personer slår prototypen på gullsettet |
| M4 | Personer | Ansiktsdeteksjon, embeddings, klynging, «Hvem er med?»-skjerm, roller, sjeldenhetsbonus, dekning | Alle navngitte personer med i utvalget; sjeldne personer 100 % |
| M5 | Hendelser og full målfunksjon | Hendelsesklynging, viktighet, kalender, lokal geokoding, spesielle øyeblikk og betydning (SCORING §3.6), submodulært utvalg, forklaringer | Beholdt-andel ≥ 60 % i intern test |
| M6 | Layout og redigering | Sidevekt, maler (helside, 2, 3, 4, serie-oppslag), rytmeregler, kapitler, bildetekster, redigering, forsidevelger. Kommentarer på utkastet, oppskarping, kryptert sikkerhetskopi | Hele albumet kan finpusses i appen |
| M7 | Trykk | Trykk-PDF etter trykkeriets krav, omslag med rygg, bestilling (kun PDF sendes) med kunde-ID som nøkkelpar og minimale opplysninger hos oss | Første fysiske prøvetrykk |
| M8 | Beta | 10–20 familier, innsamling av tilbakemelding (opt-in, uten bilder), kalibrering av vekter | Beholdt-andel ≥ 70 %, «alle er med» |
