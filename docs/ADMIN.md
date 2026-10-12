# Adminsiden (plan)

Eierens ønske (11. oktober 2026): én side for å ha oversikt over løsningen, leverandøroppfølging, fakturering, nedlastinger av appen, kundetilbakemeldinger og forbedringslogg.

Dette er en plan, ikke bygget ennå. Adminsiden trengs først når bestilling (M7) og beta (M8) er i gang. Planen er laget nå så de valgene vi gjør i appen, passer med den.

## Ufravikelig: adminsiden ser aldri familiens bilder

Adminsiden viser bare det som allerede finnes hos oss etter ARCHITECTURE.md, «Hos oss (minimalt)»:

- Ordrer per kunde-ID (offentlig nøkkel, ingen konto), navn og leveringsadresse til levering, produkt, pris og betalingsreferanse.
- Trykk-PDF-en ligger hos oss bare til albumet er levert og reklamasjonsfristen er ute. Adminsiden viser **at** den finnes og når den slettes. Den kan ikke åpne den, utenom et logget unntak ved reklamasjon som kunden selv ber om.
- Tilbakemeldinger kunden selv har valgt å sende: tekst, appversjon og operativsystem. Aldri bilder, filstier, navn fra familieprofilen eller ansiktsdata.
- Nedlastingstall fra nedlastingssiden og oppdateringstjenesten. Ingen sporing inne i appen (telemetri er av som standard, og bare uten innhold).

## Moduler

| Modul | Hva den viser | Kilde |
|---|---|---|
| **Oversikt** | Ordrer i dag/uke/måned, omsetning, ordrer som venter på trykkeri eller er forsinket, nye tilbakemeldinger, nedlastinger siste 30 dager, siste versjon | De andre modulene |
| **Bestillinger** | Hver ordre: status (betalt → sendt til trykkeri → trykket → sendt → levert), sporing, sletting av PDF (dato), reklamasjoner | Ordretjenesten (`p2a-order` i appen sender hit) |
| **Fakturering og regnskap** | Betalinger og refusjoner (Vipps og Stripe), kvitteringer, overføring til Fiken, mva-oppsummering | Betalingsleverandøren, regnskapssystemet |
| **Leverandører** | Trykkerier og andre leverandører: avtaler (inkl. databehandleravtale), priser, produksjonstid, avvik og reklamasjoner per leverandør, fakturaer fra dem, kontaktpersoner | Ordrer + manuelle notater |
| **Apper og versjoner** | Nedlastinger per plattform og versjon, hvilke versjoner som er i bruk (fra oppdateringssjekken, bare versjonsnummer), signering og utløpsdatoer for sertifikater | Nedlastingssiden, oppdateringstjenesten |
| **Tilbakemeldinger** | Meldinger fra appen («Gi oss tilbakemelding»), e-post og support, med status (ny, besvart, løst) og kobling til forbedringer | Appen (opt-in), e-post |
| **Forbedringslogg** | Hva som er gjort og planlagt, koblet til tilbakemeldingene som ba om det, og til evalueringsresultatene (`eval/RESULTS.md`) for endringer i utvalget | Repoet (commits, `docs/OVERLEVERING.md`), tilbakemeldinger |

## Teknisk forslag

- **Egen nettjeneste, adskilt fra appen.** Appens analysekode har ingen nettverkstilgang (`p2a-policy`). Bare bestillingen (`p2a-order`) og en valgfri tilbakemelding snakker med tjenesten.
- **Lagring i EU/EØS** (GDPR), kryptert. Postgres hos en europeisk leverandør, eller tilsvarende.
- **Innlogging for admin** med tofaktor, få personer, roller (eier, support, regnskap). All tilgang til ordrer logges.
- **Grensesnitt** med samme designsystem som appen (`design/tokens.css`), norsk bokmål.
- **Integrasjoner:** betaling (Vipps/Stripe webhooks), trykkeriets API (status og sporing), regnskap (eksport eller API), nedlastingssiden (tellere).

## Rekkefølge

1. **A1, sammen med første bestilling (M7):** Bestillinger, Leverandører (ett trykkeri), enkel Fakturering (betalinger og eksport).
2. **A2, sammen med beta (M8):** Tilbakemeldinger og Forbedringslogg.
3. **A3:** Oversikt med nøkkeltall, Apper og versjoner.

## Det som er gjort i appen allerede

- «Gi oss tilbakemelding» i appen (se `docs/OVERLEVERING.md`): kunden skriver selv, ser nøyaktig hva som sendes, og det går som e-post fra kundens eget e-postprogram. Ingen bilder, ingen server ennå. Når adminsiden finnes, kan meldingene hentes inn derfra.

## Eierens valg (12. oktober 2026)

- **Betaling:** både Vipps og Stripe.
- **Regnskap:** Fiken (eksport eller API fra Fakturering).
- **E-post:** `plussdelta@fo2album.no` (tilbakemeldinger og kontakt på nettsiden).
- **Trykkeri:** ikke valgt ennå. Eieren lager først nettside, e-post og signatur, og henvender seg så til trykkerier med forespørselen i `docs/LEVERANDOR-FORESPORSEL.md`.
