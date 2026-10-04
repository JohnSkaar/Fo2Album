/**
 * All tekst i grensesnittet, på norsk bokmål (CLAUDE.md, «Språk og tekst»).
 * Du-form, varm og konkret, setningsstor bokstav. Si aldri «last opp bildene».
 */
import type { KildeType } from "./api";

const tall = new Intl.NumberFormat("nb-NO");
/** 8412 → «8 412» */
export const fmt = (n: number) => tall.format(n);
/** «1 bilde», «2 bilder» */
export const antall = (n: number, en: string, flere: string) => `${fmt(n)} ${n === 1 ? en : flere}`;

export const maaneder = [
  "januar",
  "februar",
  "mars",
  "april",
  "mai",
  "juni",
  "juli",
  "august",
  "september",
  "oktober",
  "november",
  "desember",
] as const;

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
    fjernKilde: (navn: string) => `Fjern ${navn}`,
    antallBilder: (n: number) => antall(n, "bilde", "bilder"),
    kildeMangler: "Finnes ikke nå",
    innstillinger: "Innstillinger",
    slettAlleData: "Slett alle data",
  },
  lokalt: {
    tittel: "Bildene blir på denne maskinen.",
    tekst:
      "Også bilder fra Dropbox, iCloud og Google Disk hentes fra maskinen. Bare det ferdige albumet sendes til trykk.",
  },
  velkommen: {
    tittel: "Velkommen til Pho2Album.",
    ingress:
      "Minnene blir sterkere når dere blar i dem sammen. Appen finner de beste bildene fra året og lager et album dere kan holde i.",
    trygt:
      "Alt appen lærer om familien deres, som hvem som er på bildene og kommentarene dine, lagres kryptert på denne maskinen.",
    start: "Kom i gang",
  },
  gjenopprettingsnokkel: {
    tittel: "Skriv ned gjenopprettingsnøkkelen.",
    ingress:
      "Med denne nøkkelen kan dere åpne familiealbumene på en ny maskin. Den vises bare nå, og vi har ingen kopi.",
    tips: "Skriv den på papir og legg den et trygt sted, for eksempel sammen med andre viktige papirer.",
    kopier: "Kopier",
    kopiert: "Kopiert",
    bekreft: "Jeg har skrevet ned nøkkelen",
    fortsett: "Fortsett",
    etikett: "Gjenopprettingsnøkkel",
  },
  gjenopprett: {
    tittel: "Skriv inn gjenopprettingsnøkkelen.",
    ingress:
      "Appen finner familiealbumene deres, men nøkkelen til å åpne dem mangler på denne maskinen. Det skjer typisk på en ny maskin.",
    etikett: "Gjenopprettingsnøkkel",
    plassholder: "ABCD-EFGH-…",
    knapp: "Åpne",
    hjelp: "Nøkkelen er 28 tegn i grupper på fire. Store og små bokstaver er det samme.",
  },
  start: {
    tittel: "Velg bildemappene dere vil lage årets familiealbum av.",
    ingress:
      "Bildene ligger ofte spredt: på PC-en, i Dropbox, iCloud og Google Disk. Legg til alle kildene, så blander appen dem, fjerner dubletter, finner de beste bildene fra året og lager et utkast du kan finpusse.",
    trygt: "Alt skjer på din egen maskin. Bildene sendes ingen steder.",
    hjelp:
      "Dropbox og Google Disk synkroniserer til en vanlig mappe på maskinen. Velg den mappen, så hentes bildene derfra. På Mac henter appen iCloud-bildene fra Bilder-biblioteket.",
    kilderEtikett: "Velg en bildekilde",
    forslagKnapp: "Finn vanlige bildemapper",
    forslagHjelp:
      "Appen ser etter de vanlige mappene til Dropbox, iCloud og Google Disk. Ingenting leses før du velger.",
    forslagTittel: "Fant disse mappene",
    forslagIngen: "Fant ingen av de vanlige mappene. Velg mappen selv med kortene over.",
    leggTil: "Legg til",
    lagtTil: "Lagt til",
  },
  kilder: {
    pc: { navn: "Mappe på maskinen", hint: "F.eks. Bilder › 2025" },
    dropbox: { navn: "Dropbox", hint: "Dropbox-mappen på maskinen" },
    icloud: { navn: "iCloud Bilder", hint: "Bilder-biblioteket på Mac, iCloud-mappen på Windows" },
    google_disk: { navn: "Google Disk", hint: "Google Disk-mappen på maskinen" },
    apple_photos: { navn: "Apple Bilder", hint: "Bilder-biblioteket på Mac" },
  } satisfies Record<KildeType, { navn: string; hint: string }>,
  bilder: {
    tittel: (aar: number) => `Familiealbum ${aar}`,
    ingenAar: "Velg bilder",
    antall: (n: number) => antall(n, "bilde", "bilder"),
    aarEtikett: "Velg år",
    tom: "Ingen bilder her ennå. Legg til en mappe, eller velg et annet år.",
    leserInn: "Bildene dukker opp her etter hvert som de hentes fra maskinen.",
    ingenMiniatyr: "Kan ikke vises",
    usikkerDato: "Usikker dato",
    maaned: (m: number, aar: number) => `${stor(maaneder[m] ?? "")} ${aar}`,
    bildeEtikett: (dato: string) => `Bilde fra ${dato}`,
  },
  innlesing: {
    skanner: "Går gjennom mappene …",
    leser: (done: number, total: number) =>
      `Leser bilder fra maskinen … ${fmt(done)} av ${fmt(total)}`,
    dubletter: "Leter etter samme bilde i flere kilder …",
    avbryt: "Stopp",
    ferdig: (nye: number) =>
      nye === 0 ? "Alt er oppdatert" : `Fant ${antall(nye, "nytt bilde", "nye bilder")}`,
    stoppet: "Innlesingen er stoppet. Den fortsetter der den slapp neste gang.",
  },
  merknader: {
    dubletter: (n: number) =>
      `${antall(n, "dublett", "dubletter")} fra flere kilder er slått sammen, så hvert bilde bare kommer med én gang.`,
    bareISkyen: (n: number) =>
      `${antall(n, "bilde", "bilder")} ligger bare i skyen og er ikke lastet ned til maskinen. Last dem ned i Dropbox, iCloud eller Google Disk, så tar appen dem med neste gang.`,
    utenMiniatyr: (n: number) =>
      `${antall(n, "bilde", "bilder")} kan ikke vises på denne maskinen ennå, ofte iPhone-bilder i HEIC-format. På Windows hjelper det å installere «HEIF Image Extensions» fra Microsoft Store.`,
    usikreDatoer: (n: number) =>
      `${antall(n, "bilde", "bilder")} mangler opptaksdato, så appen bruker datoen filen sist ble endret. Den kan være feil.`,
    uleselige: (n: number) => `${antall(n, "fil", "filer")} kunne ikke leses og er hoppet over.`,
    manglerKilder: (navn: string[]) =>
      `Fant ikke ${navn.join(", ")}. Er disken koblet til? Bildene derfra er ikke fjernet.`,
  },
  slett: {
    tittel: "Slette alle data?",
    tekst:
      "Appen glemmer alt den har lært: mapper, personer, kommentarer og miniatyrer. Bildene dine i mappene blir liggende urørt.",
    bekreft: "Slett alle data",
    avbryt: "Avbryt",
    ferdig: "Alle data er slettet",
  },
  feil: {
    feil_nokkel: "Nøkkelen passer ikke. Sjekk at du har skrevet den riktig.",
    ugyldig_kode: "Det ser ut som det er en skrivefeil i nøkkelen. Sjekk tegnene og prøv igjen.",
    innlesing_pagar: "Vent til appen er ferdig med å lese bildene.",
    ikke_mappe: "Fant ikke mappen.",
    nyere_versjon: "Dataene er laget av en nyere versjon av appen. Oppdater Pho2Album.",
    ukjent: "Noe gikk galt. Prøv igjen.",
  } as Record<string, string>,
  melding: {
    kildeLagtTil: (navn: string) => `${navn} er lagt til`,
    kildeFjernet: (navn: string) => `${navn} er fjernet fra appen. Bildene ligger der de lå.`,
  },
} as const;

function stor(s: string) {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/** Norsk tekst for en feil fra Rust-kjernen. */
export function feiltekst(e: unknown): string {
  if (typeof e === "object" && e !== null && "kode" in e) {
    const kode = String((e as { kode: unknown }).kode);
    return tekster.feil[kode] ?? tekster.feil.ukjent ?? "";
  }
  return tekster.feil.ukjent ?? "";
}

/** «14. juli 2011» */
export function datoTekst(iso: string): string {
  const [y, m, d] = iso.slice(0, 10).split("-").map(Number);
  return `${d}. ${maaneder[(m ?? 1) - 1]} ${y}`;
}
