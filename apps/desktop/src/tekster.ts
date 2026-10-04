/**
 * All tekst i grensesnittet, på norsk bokmål (CLAUDE.md, «Språk og tekst»).
 * Du-form, varm og konkret, setningsstor bokstav. Si aldri «last opp bildene».
 */
export const tekster = {
  appNavn: "Pho2Album",
  nav: {
    album: "Album",
    velgBilder: "Velg bilder",
    albumutkast: "Albumutkast",
    ikkeLaget: "Ikke laget ennå",
    bildekilder: "Bildekilder",
    leggTilMappe: "Legg til mappe",
    hovedmeny: "Hovedmeny",
  },
  lokalt: {
    tittel: "Bildene blir på denne maskinen.",
    tekst:
      "Også bilder fra Dropbox, iCloud og Google Disk hentes fra maskinen. Bare det ferdige albumet sendes til trykk.",
  },
  start: {
    tittel: "Velg bildemappene dere vil lage årets familiealbum av.",
    ingress:
      "Bildene ligger ofte spredt: på PC-en, i Dropbox, iCloud og Google Disk. Legg til alle kildene, så blander appen dem, fjerner dubletter, finner de beste bildene fra året og lager et utkast du kan finpusse.",
    trygt: "Alt skjer på din egen maskin. Bildene sendes ingen steder.",
    hjelp:
      "Dropbox og Google Disk synkroniserer til en vanlig mappe på maskinen. Velg den mappen, så hentes bildene derfra. På Mac henter appen iCloud-bildene fra Bilder-biblioteket.",
    kilderEtikett: "Velg en bildekilde",
  },
  kilder: {
    pc: { navn: "Mappe på maskinen", hint: "F.eks. Bilder › 2025" },
    dropbox: { navn: "Dropbox", hint: "Dropbox-mappen på maskinen" },
    icloud: { navn: "iCloud Bilder", hint: "Bilder-biblioteket på Mac, iCloud-mappen på Windows" },
    googleDisk: { navn: "Google Disk", hint: "Google Disk-mappen på maskinen" },
  },
  melding: {
    kommerSnart: "Mappevalg kommer i neste versjon av appen.",
  },
} as const;

export type KildeId = keyof typeof tekster.kilder;
