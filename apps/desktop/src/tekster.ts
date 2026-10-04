/**
 * All tekst i grensesnittet, på norsk bokmål (CLAUDE.md, «Språk og tekst»).
 * Du-form, varm og konkret, setningsstor bokstav. Si aldri «last opp bildene».
 */
import type { Analysefase, Begrunnelse, KildeType, Laerdom, Svar } from "./api";
import { GRUNNPRIS, KR_PER_SIDE, SIDETRINN } from "./pris";

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
  appNavn: "Fo2Album",
  nav: {
    album: "Album",
    alleBilder: "Alle bilder",
    albumutkast: "Albumutkast",
    ikkeLaget: "Ikke laget ennå",
    sider: (n: number) => `Omtrent ${antall(n, "side", "sider")}`,
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
    tittel: "Velkommen til Fo2Album.",
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
    tittel: (aar: number) => `Alle bilder fra ${aar}`,
    ingenAar: "Alle bilder",
    antall: (n: number) => antall(n, "bilde", "bilder"),
    aarEtikett: "Velg år",
    tom: "Ingen bilder her ennå. Legg til en mappe, eller velg et annet år.",
    leserInn: "Bildene dukker opp her etter hvert som de hentes fra maskinen.",
    ingenMiniatyr: "Kan ikke vises",
    usikkerDato: "Usikker dato",
    maaned: (m: number, aar: number) => `${stor(maaneder[m] ?? "")} ${aar}`,
    bildeEtikett: (dato: string) => `Bilde fra ${dato}`,
  },
  utkast: {
    tittel: (aar: number) => `Familiealbum ${aar}`,
    ingress:
      "Appen går gjennom alle bildene fra året, finner hendelsene og lager et komplett forslag til album. Etterpå ser du hva som er med og hva som ikke er med, og hvorfor. Du bytter der du er uenig.",
    tid: "Det tar litt tid, og alt skjer på denne maskinen.",
    flereKilder: "Har dere bilder flere steder?",
    flereKilderTekst:
      "Legg til alle kildene før du lager utkastet: mobilen til begge foreldrene, Dropbox, iCloud, Google Disk og mapper på PC-en. Appen blander dem og fjerner dubletter.",
    kilderLagtTil: "Lagt til",
    aar: "Album for",
    aarValg: (aar: number, n: number) => `${aar} (${antall(n, "bilde", "bilder")})`,
    lag: "Lag utkast",
    venterInnlesing:
      "Appen henter fortsatt bilder fra maskinen. Utkastet tar med alle bildene når den er ferdig.",
    ingenBilder: "Ingen bilder fra dette året ennå.",
    analyseTittel: "Lager utkastet …",
    faser: {
      venter: "Henter miniatyrbilder og måler kvalitet",
      henter: "Henter årets bilder",
      hendelser: "Finner hendelsene i året",
      serier: "Finner serier og bilder som ligner hverandre",
      velger: "Velger de beste bildene fra hver hendelse",
      begrunnelser: "Skriver en kort begrunnelse for hvert bilde",
      ferdig: "Ferdig",
    } satisfies Record<Analysefase, string>,
    utkastTittel: (aar: number) => `Utkast til Familiealbum ${aar}`,
    sammendrag: (bilder: number, sider: number, hendelser: number) =>
      `${antall(bilder, "bilde", "bilder")} på omtrent ${antall(sider, "side", "sider")}, fra ${antall(hendelser, "hendelse", "hendelser")}.`,
    hjelp:
      "Til venstre er bildene appen foreslår, til høyre de som ikke er med. Klikk et bilde for å se hvorfor, og for å ta det med, ta det bort eller bytte.",
    med: "Med i albumet",
    ikkeMed: "Ikke med",
    ingenIkkeMed: "Alle bildene herfra er med.",
    visAlle: (n: number) => `Vis alle ${fmt(n)}`,
    visFaerre: "Vis færre",
    hverdager: (m: number) => `Hverdager i ${maaneder[m]}`,
    hendelseInfo: (bilder: number, sider: number) =>
      `${antall(bilder, "bilde", "bilder")} · ${antall(sider, "side", "sider")}`,
    taMed: "Ta med",
    taBort: "Ta bort",
    byttMedLignende: "Bytt med bildet som er med",
    byttHjelp: "Eller klikk et bilde i den andre kolonnen for å bytte.",
    lukk: "Lukk",
    uskarpt: "Uskarpt?",
    uskarptHjelp: "Kan være uskarpt, sammenlignet med resten av årets bilder",
    bildeEtikett: (dato: string, med: boolean) => `Bilde fra ${dato}, ${med ? "med" : "ikke med"}`,
    nyttUtkast: "Lag nytt utkast med det du har lært appen",
    lagtTil: "Bildet er med",
    tattBort: "Bildet er tatt bort",
    byttet: "Bildene er byttet",
  },
  pris: {
    naa: (sider: number, kr: number) =>
      `Albumet er nå på ${antall(sider, "side", "sider")} og koster ${fmt(kr)} kr.`,
    juster:
      "Juster antall sider etter ønske. Færre sider: alle historiene krymper litt, men hver dag beholder minst én side. Flere sider: flere bilder og mer plass til historiene.",
    ned: (sider: number, kr: number) => `↓ ${fmt(sider)} sider · ${fmt(kr)} kr`,
    opp: (sider: number, kr: number) => `↑ ${fmt(sider)} sider · ${fmt(kr)} kr`,
    forslag: (sider: number, kr: number) => `Appens forslag: ${fmt(sider)} sider · ${fmt(kr)} kr`,
    eget: "Eget antall",
    lagPaaNytt: "Lag på nytt",
    modell: `Prisen (foreløpig): perm ${fmt(GRUNNPRIS)} kr + ${KR_PER_SIDE} kr per side, regnet i trinn på ${SIDETRINN} sider.`,
    etikett: "Velg størrelse på albumet",
  },
  sider: {
    tittel: "Slik blir sidene",
    helside: "Helside",
    luft: "Ett bilde med luft rundt",
    rutenett: (n: number) => `${antall(n, "bilde", "bilder")} på siden`,
    side: (n: number) => `Side ${fmt(n)}`,
    endret: "Sidene ordnes på nytt når du lager utkastet igjen.",
  },
  begrunnelse: (b: Begrunnelse, bilderIHendelsen: number): string => {
    const n = b.antall ?? 0;
    switch (b.kode) {
      case "valgt_av_deg":
        return "Du valgte dette";
      case "beste_fra_hendelsen":
        return "Det beste bildet herfra";
      case "eneste_fra_hendelsen":
        return bilderIHendelsen === 1
          ? "Det eneste bildet herfra"
          : `Det beste av ${antall(bilderIHendelsen, "bilde", "bilder")} herfra`;
      case "svakt_men_eneste":
        return "Litt uskarpt eller mørkt, men det beste herfra";
      case "beste_i_serie":
        return `Beste bilde i en serie på ${fmt(n)}`;
      case "annen_del_av_hendelsen":
        return "Viser en annen del av dagen";
      case "god_kvalitet":
        return "Skarpt og godt lys";
      case "stemningsbilde":
        return "Et flott stemningsbilde";
      case "fornoyd":
        return "På en side du er fornøyd med";
      case "valgt_bort_av_deg":
        return "Du tok det bort";
      case "samme_serie":
        return "Et bedre bilde fra samme øyeblikk er med";
      case "nesten_likt":
        return "Nesten likt et bilde som er med";
      case "uskarpt":
        return "Uskarpt";
      case "morkt_eller_utbrent":
        return "For mørkt eller for lyst";
      case "skjermbilde":
        return "Ser ut som et skjermbilde";
      case "gjenstand":
        return "Ingen personer, og ikke et spesielt flott bilde";
      case "en_stemning_holder":
        return "Ett stemningsbilde herfra er nok";
      case "ikke_plass":
        return `Ikke plass: ${antall(n, "bedre bilde", "bedre bilder")} herfra er med`;
    }
  },
  hvorfor: {
    taBort: "Hvorfor tok du det bort?",
    taMed: "Hva gjør dette bildet viktig?",
    bytt: "Hva var bedre med det nye bildet?",
    hjelp: "Valgfritt. Svaret hjelper appen å forstå hva dere bryr dere om.",
    hoppOver: "Hopp over",
    takk: "Takk! Appen husker det til neste utkast.",
    svar: {
      uskarpt: "Uskarpt",
      daarlig_lys: "Dårlig lys",
      for_likt: "For likt et annet",
      for_mange_herfra: "For mange herfra",
      liker_ikke: "Liker det ikke",
      privat: "Skal ikke i albumet",
      viktig_oyeblikk: "Viktig øyeblikk",
      viktig_person: "Viktig person",
      fint_bilde: "Fint bilde",
      mangler_herfra: "Mangler noe herfra",
      skarpere: "Skarpere",
      gjenstand: "Bare en ting, ingen personer",
      uviktig_dag: "Ikke viktig for oss",
      stemning: "Fin stemning",
    } satisfies Record<Svar, string>,
  },
  laert: {
    tittel: "Dette har appen lært om dere",
    ingenting:
      "Ingenting ennå. Når du tar med, tar bort eller bytter bilder, lærer appen litt etter litt hva dere liker.",
    grunnlag: (valg: number, svar: number) =>
      `Basert på ${antall(valg, "valg", "valg")} og ${antall(svar, "svar", "svar")}.`,
    laerdom: {
      skarphet_teller_mer: "Skarpe bilder betyr mye for dere.",
      lys_teller_mer: "Godt lys betyr mye for dere.",
      farger_teller_mer: "Dere liker fargerike bilder.",
      oyeblikk_fremfor_kvalitet: "Øyeblikket betyr mer enn om bildet er perfekt.",
      kvalitet_fremfor_oyeblikk: "Dere vil helst ha teknisk gode bilder.",
      faerre_like_bilder: "Dere vil ikke ha bilder som ligner hverandre.",
      flere_fra_hver_hendelse: "Dere vil ha flere bilder fra hver hendelse.",
      faerre_fra_hver_hendelse: "Dere vil ha færre bilder fra hver hendelse.",
      ting_bare_naar_flotte:
        "Bilder av ting uten personer skal bare med når de er virkelig flotte.",
      liker_stemningsbilder: "Dere liker stemningsbilder uten personer.",
    } satisfies Record<Laerdom, string>,
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
    nyere_versjon: "Dataene er laget av en nyere versjon av appen. Oppdater Fo2Album.",
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
