/**
 * All tekst i grensesnittet, på norsk bokmål (CLAUDE.md, «Språk og tekst»).
 * Du-form, varm og konkret, setningsstor bokstav. Si aldri «last opp bildene».
 */
import type { Analysefase, Begrunnelse, KildeType, Laerdom, Rolle, Svar } from "./api";
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
    hvemErMed: "Hvem er med?",
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
    roter: "Roter",
    roterHjelp: "Roter en kvart omdreining med klokka",
    brukForside: "Bruk som forside",
    brukBakside: "Bruk som bakside",
    turSporsmaal:
      "Ser ut som en tur over flere dager. Var det en tur uten barn (gutte- eller jentetur, jobbtur)? Da får den færre sider, med et bilde av hver person.",
    turGjettet:
      "Ingen av barna er på bildene, så appen har gjort dette til en tur uten barn: færre sider og et bilde av hver person.",
    turJa: "Ja, tur uten barn",
    turNei: "Nei, familietur",
    turUtenBarn: "Tur uten barn",
    presenter: "Presenter på",
    sider: (n: number) => (n === 1 ? "side" : "sider"),
    farreSider: "Færre sider",
    flereSider: "Flere sider",
    lagForslag: "Lag forslag",
    appensForslag: "Appens forslag",
    delOpp: "Del opp igjen",
    slaaSammenForslag: (dager: string) =>
      `${dager} er korte dager tett etter hverandre. Vil du slå dem sammen til én historie?`,
    slaaSammen: "Slå sammen",
    neiTakk: "Nei takk",
    omslag: "Forside og bakside",
    omslagHjelp:
      "Forslag fra de beste bildene gjennom året. Du kan også velge et hvilket som helst bilde: klikk det og trykk «Bruk som forside».",
    forside: "Forside",
    bakside: "Bakside",
    valgtAvDeg: "Valgt av deg",
    forslag: "Forslag",
    medPersoner: "Med personer",
    oversikt: "Oversiktsbilder",
    oppdatert: "Utkastet er oppdatert",
    fornoydMelding: "Siden beholdes som den er i neste utkast",
    ikkeFornoydMelding: "Siden kan endres igjen i neste utkast",
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
    fornoyd: "Fornøyd",
    fornoydHjelp: "Siden beholdes som den er når utkastet lages på nytt",
    erFornoyd: "✓ Fornøyd",
    rammer: "Rammer",
    rammerHjelp: "Velg rammer for siden",
    rammerTittel: "Rammer for siden",
    rammerIngress:
      "Automatisk viser hele bildene. Faste rammer fyller feltene, og du kan flytte utsnittet. Siden merkes «Fornøyd» så den beholdes.",
    auto: "Automatisk",
    autoHjelp: "Hele bildene, ingen beskjæring",
    mal: {
      "1-full": "Hele siden",
      "1-kvadrat": "Ett kvadratisk",
      "1-landskap": "Ett liggende",
      "1-portrett": "Ett stående",
      "2-over": "To over hverandre",
      "2-side": "To ved siden av hverandre",
      "3-topp": "Ett stort og to små",
      "3-venstre": "Ett høyt og to ved siden",
      "3-rader": "Tre liggende",
      "4-kvadrat": "Fire kvadratiske",
      "4-landskap": "Fire liggende",
      "6": "Seks kvadratiske",
      "9": "Ni kvadratiske",
      "12": "Tolv kvadratiske",
    } as Record<string, string>,
    ingenMaler: "Det finnes ingen faste rammer for så mange bilder. Siden er automatisk.",
    malValgt: "Rammene er valgt, og siden er merket «Fornøyd»",
    bildePaaSiden: (dato: string) => `Bilde fra ${dato} på siden`,
  },
  redigering: {
    storre: "Større",
    mindre: "Mindre",
    storreHjelp: "Større på siden, så egen side, til slutt hele siden",
    mindreHjelp: "Mindre: tilbake til samme størrelse som de andre, så litt mindre",
    storrelse: (s: number | null, alene: boolean, helside: boolean) =>
      alene
        ? helside
          ? "Bildet tar hele siden"
          : "Bildet har en egen side, med luft rundt"
        : s === null
          ? "Appen velger hvor stort bildet er på siden"
          : s > 0
            ? `Bildet er større på siden (trinn ${s} av 4)`
            : s < 0
              ? "Bildet er mindre enn de andre på siden"
              : "Bildet er like stort som de andre på siden",
    visStort: "Vis stort",
    utsnitt: "Endre utsnitt",
    utsnittHjelp: "Klikk i bildet for å velge hva som skal være i midten av rammen.",
    fyll: "Fyll rammen",
    hele: "Vis hele bildet",
    ferdig: "Ferdig",
    utsnittLagret: "Utsnittet er lagret",
    forhaand:
      "Dette er en forhåndsvisning på skjermen. Trykket bruker originalfilen i full oppløsning.",
    markerHjelp: "Marker flere (eller hold Ctrl og klikk)",
    marker: "Marker",
    markert: (n: number) => `${antall(n, "bilde", "bilder")} markert`,
    samleSide: "Sett på én side",
    samleSideHjelp: "Bildene samles på en ny side, merket «Fornøyd»",
    egenHistorie: "Egen historie",
    egenHistorieHjelp: "Bildene blir en egen historie med egne sider",
    fjernMarkering: "Fjern markering",
    samlet: (n: number) =>
      n > 12
        ? `${fmt(n)} bilder er samlet på ${fmt(Math.ceil(n / 12))} sider, merket «Fornøyd».`
        : `${antall(n, "bilde er", "bilder er")} samlet på én side, merket «Fornøyd». Velg gjerne rammer for siden.`,
    historieLaget: (n: number) => `${antall(n, "bilde er", "bilder er")} nå en egen historie.`,
    rotert: (n: number) => `${antall(n, "bilde er", "bilder er")} rotert`,
    lagtTil: (n: number) => `${antall(n, "bilde er", "bilder er")} tatt med`,
    tattBort: (n: number) => `${antall(n, "bilde er", "bilder er")} tatt bort`,
    egenHistorieMerke: "Egen historie",
    leggTilbake: "Legg tilbake",
    lagtTilbake: "Bildene er lagt tilbake der de hørte til",
    bla: "Bla i albumet",
    blaTittel: (aar: number) => `Familiealbum ${aar}`,
    forrige: "Forrige oppslag",
    neste: "Neste oppslag",
    oppslag: (n: number, av: number) => `Oppslag ${fmt(n)} av ${fmt(av)}`,
    forsideSide: "Forsiden",
    baksideSide: "Baksiden",
    introSide: "Første side: teksten om året",
    blaHjelp: "Bruk piltastene for å bla. Esc lukker.",
    angre: "Angre",
    angreHjelp: "Angre siste endring (Ctrl+Z)",
    angret: "Endringen er angret",
    flyttHjelp: "Dra bildet til en annen side, eller til et annet sted på samme side",
    flyttet: "Bildet er flyttet, og sidene er merket «Fornøyd»",
    forrigeSide: "← Forrige side",
    nesteSide: "Neste side →",
    endreTekst: "Skriv tekst",
    tekstLagret: "Teksten er lagret",
    lagreTekst: "Lagre teksten",
    tekstHjelp: "Teksten kommer med i trykkfilen.",
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
  trykk: {
    knapp: "Lag trykkfil",
    tittel: "Lag trykkfilen",
    ingress:
      "Trykkfilen er albumet som PDF, med bildene i full oppløsning, slik det skal trykkes. Skriv tekstene først; de lagres til neste gang.",
    trygt:
      "Filen lagres der du velger, på denne maskinen. Den sendes ikke noe sted før du selv bestiller.",
    tittelFelt: "Tittel på forsiden",
    undertittel: "Undertittel",
    undertittelHjelp: "For eksempel navnene i familien.",
    intro: "Første side inne i albumet",
    introHjelp: (aar: number) =>
      `Skriv litt om ${aar}: hva som skjedde, hvem som kom til, hva dere husker best.`,
    bakside: "Baksiden",
    baksideHjelp: "En hilsen, et sitat eller et minne fra året.",
    avbryt: "Avbryt",
    lag: "Velg hvor filen skal lagres",
    filnavn: (aar: number) => `Fo2Album ${aar}.pdf`,
    lager: (done: number, total: number) =>
      `Legger inn bildene i full oppløsning … ${fmt(done)} av ${fmt(total)}`,
    ferdig: (sider: number, bilder: number, mb: number) =>
      `Trykkfilen er lagret: ${antall(sider, "side", "sider")} og ${antall(bilder, "bilde", "bilder")} (${mb.toLocaleString("nb-NO", { maximumFractionDigits: 0 })} MB).`,
    mangler: (n: number) =>
      `${antall(n, "bilde", "bilder")} kunne ikke leses og står som grå felt. Sjekk at bildene finnes på maskinen (ikke bare i skyen), og lag filen på nytt.`,
    lavOppl: (n: number) =>
      `${antall(n, "bilde", "bilder")} har lav oppløsning for størrelsen i albumet og kan bli litt uskarpe på trykk.`,
    lukk: "Lukk",
  },
  sjekk: {
    tittel: (n: number) =>
      n === 0 ? "Før trykk: alt ser bra ut" : `Før trykk: ${antall(n, "ting", "ting")} å se på`,
    ingress:
      "Appen sjekker bildene slik de blir trykt: oppløsningen for størrelsen de får, at originalfilen finnes på maskinen, og uskarpe bilder som står stort.",
    ingenting:
      "Ingen bilder har for lav oppløsning, alle finnes på maskinen, og ingen uskarpe bilder står stort.",
    lavOpplosning: (ppi: number) =>
      `Lav oppløsning for størrelsen (${fmt(ppi)} ppi, bør være minst 150). Kan bli uskarpt på trykk. Gjør bildet mindre, eller bytt det ut.`,
    mangler:
      "Originalfilen finnes ikke på maskinen (flyttet, slettet eller bare i skyen). Hent bildet til maskinen, eller bytt det ut.",
    uskarptStort: "Ser uskarpt ut og står stort. Gjør det mindre, eller bytt det ut.",
    forside: "Forsiden",
    bakside: "Baksiden",
    side: (historie: string, side: number) => `${historie}, side ${fmt(side)}`,
    vis: "Vis",
    merke: "Se på før trykk",
    trykkOppsummering: (n: number) =>
      `${antall(n, "ting", "ting")} bør ses på før trykk (lav oppløsning, manglende eller uskarpe bilder). Se «Før trykk» i utkastet. Du kan likevel lage filen.`,
  },
  personer: {
    tittel: "Hvem er med?",
    ingress:
      "Appen har funnet de samme ansiktene i mange bilder. Skriv hvem de er, så får de rett plass i albumet: barna får omtrent like mange bilder i hver historie, og turer uten barn kjennes igjen.",
    trygt: "Ansiktene og navnene lagres kryptert på denne maskinen og sendes aldri noe sted.",
    venter: (n: number) =>
      `Appen leter fortsatt etter ansikter i ${antall(n, "bilde", "bilder")}. Flere personer kommer etter hvert.`,
    ingen:
      "Appen har ikke funnet noen personer i flere bilder ennå. Velg bildemapper først, så leter den etter ansikter mens bildene hentes inn.",
    navngitt: "Personer dere har navngitt",
    ukjent: "Hvem er dette?",
    ukjentInfo: "Skriv navnet, eller velg en person dere allerede har navngitt.",
    antall: (faces: number, photos: number) =>
      `${antall(faces, "ansikt", "ansikter")} i ${antall(photos, "bilde", "bilder")}`,
    navn: "Navn",
    rolle: "Rolle",
    finnes: "Eller samme person som",
    velgPerson: "Velg person",
    lagre: "Lagre",
    ikkeViktig: "Ikke viktig",
    ikkeViktigInfo: "Fremmede eller folk i bakgrunnen: telles ikke i albumet.",
    ikkeSamme: (navn: string) => `Ikke ${navn}`,
    ikkeSammeInfo: "Ta dette ansiktet ut av gruppen",
    endre: "Endre",
    lagret: (navn: string) => `Lagret. Appen kjenner nå igjen ${navn}.`,
    ignorert: "Gruppen telles ikke lenger i albumet.",
    flyttet: "Ansiktet er tatt ut av gruppen.",
    mangler: "Skriv et navn først.",
    roller: {
      barn: "Barn i familien",
      kjernefamilie: "Forelder",
      besteforeldre: "Besteforelder",
      naer_familie: "Nær familie",
      venn: "Venn",
      annen: "Annen",
    } satisfies Record<Rolle, string>,
  },
  innlesing: {
    skanner: "Går gjennom mappene …",
    leser: (done: number, total: number) =>
      `Leser bilder fra maskinen … ${fmt(done)} av ${fmt(total)}`,
    dubletter: "Leter etter samme bilde i flere kilder …",
    ansikter: (done: number, total: number) =>
      `Finner ansiktene i bildene … ${fmt(done)} av ${fmt(total)}`,
    personer: "Kjenner igjen de samme personene på tvers av bildene …",
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
