/**
 * Alle kall til Rust-kjernen. Grensesnittet gjør aldri nettverkskall; alt går via IPC.
 * Testene erstatter denne modulen (vi.mock), så komponentene kan testes uten Tauri.
 */
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";

export type KildeType = "pc" | "dropbox" | "icloud" | "google_disk" | "apple_photos";
export type LagringStatus = "tom" | "trenger_gjenoppretting" | "laast" | "klar";
export type Datokilde = "exif" | "filnavn" | "endringstid";

export interface Kilde {
  id: number;
  kind: KildeType;
  path: string;
  label: string;
  finnes: boolean;
}

export interface Forslag {
  kind: KildeType;
  path: string;
  label: string;
}

export interface Sammendrag {
  sources: number;
  files: number;
  cloud_only: number;
  unreadable: number;
  photos: number;
  exact_duplicates: number;
  near_duplicates: number;
  uncertain_dates: number;
  without_preview: number;
}

export interface Aar {
  year: number;
  count: number;
}

export interface Bilde {
  id: string;
  takenAt: string | null;
  dateSource: Datokilde | null;
  width: number | null;
  height: number | null;
  hasThumbnail: boolean;
  sources: KildeType[];
}

export interface Fremdrift {
  phase: "skanner" | "leser" | "dubletter" | "ansikter" | "personer";
  done: number;
  total: number;
}

export interface Innlesingsrapport {
  added: number;
  changed: number;
  removed: number;
  read: number;
  new_photos: number;
  unreadable: number;
  missing_sources: string[];
  cancelled: boolean;
  summary: Sammendrag;
}

export interface InnlesingFerdig {
  report: Innlesingsrapport | null;
  feil: string | null;
}

/** Begrunnelse for et bilde i utkastet. Teksten ligger i tekster.ts. */
export type Begrunnelseskode =
  | "valgt_av_deg"
  | "beste_fra_hendelsen"
  | "eneste_fra_hendelsen"
  | "svakt_men_eneste"
  | "beste_i_serie"
  | "annen_del_av_hendelsen"
  | "god_kvalitet"
  | "stemningsbilde"
  | "fornoyd"
  | "valgt_bort_av_deg"
  | "samme_serie"
  | "nesten_likt"
  | "uskarpt"
  | "morkt_eller_utbrent"
  | "skjermbilde"
  | "gjenstand"
  | "en_stemning_holder"
  | "ikke_plass";

export interface Begrunnelse {
  kode: Begrunnelseskode;
  antall?: number;
}

export interface UtkastBilde {
  id: string;
  takenAt: string;
  /** Indeks i `Utkast.events`. */
  event: number;
  included: boolean;
  reason: Begrunnelse;
  /** Uskarpt sammenlignet med resten av årets bilder. */
  blurry: boolean;
  /** Bildet dette henger sammen med (samme serie, likt, eller nærmest i tid). */
  related: string | null;
  hasThumbnail: boolean;
  width: number | null;
  height: number | null;
  /** Brukerens rotering med klokka: 0, 90, 180 eller 270. */
  rotation: number;
}

export interface UtkastHendelse {
  /** Et bilde i hendelsen; nøkkelen for valgene (tur, sider, slå sammen). */
  key: string;
  /** Dager brukeren har slått sammen. */
  merged: boolean;
  /** Brukerens «presenter på x sider», hvis satt. */
  pageTarget: number | null;
  /** Brukeren har svart på om dette var en tur uten barn. */
  tripAnswered: boolean;
  start: string;
  end: string;
  photos: number;
  included: number;
  pages: number;
  /** Små hendelser i en måned, samlet. */
  everyday: boolean;
  /** Ser ut som en tur over flere dager. */
  looksLikeTrip: boolean;
  /** Tur uten barn: brukeren har sagt det, eller ingen av barna er på bildene. */
  adultTrip: boolean;
  /** Appen så selv at ingen av barna er med (brukeren har ikke svart ennå). */
  adultTripGuess: boolean;
  /** Sidene historien får. */
  layout: Side[];
}

/** «helside» fyller hele rammen, «luft» er ett bilde med marg, «rutenett» flere bilder. */
export interface Side {
  kind: "helside" | "luft" | "rutenett";
  kolonner?: number;
  /** Brukeren er fornøyd med siden; den beholdes i neste utkast. */
  locked?: boolean;
  /** Bilde-id-er. */
  photos: string[];
}

export type Laerdom =
  | "skarphet_teller_mer"
  | "lys_teller_mer"
  | "farger_teller_mer"
  | "oyeblikk_fremfor_kvalitet"
  | "kvalitet_fremfor_oyeblikk"
  | "faerre_like_bilder"
  | "flere_fra_hver_hendelse"
  | "faerre_fra_hver_hendelse"
  | "ting_bare_naar_flotte"
  | "liker_stemningsbilder";

export interface Laert {
  lessons: Laerdom[];
  choices: number;
  answers: number;
}

export interface Utkast {
  year: number;
  /** Forslag om å slå sammen korte dager tett etter hverandre (indekser i `events`). */
  mergeSuggestions: number[][];
  pages: number;
  /** Sider uten sidetak (hele historien). */
  fullPages: number;
  pageCap: number | null;
  cover: Omslag;
  events: UtkastHendelse[];
  photos: UtkastBilde[];
  learned: Laert;
}

/** Forside og bakside: det som brukes nå (valgt eller foreslått) og forslagene. */
export interface Omslag {
  front: string | null;
  back: string | null;
  chosenFront: boolean;
  chosenBack: boolean;
  people: string[];
  overview: string[];
}

/** Et valg for albumet utover enkeltbilder. Lagres kryptert per år. */
export type Albumvalg =
  | {
      type: "fornoyd";
      page: { kind: Side["kind"]; kolonner: number | null; photos: string[] };
      on: boolean;
    }
  | { type: "tur_uten_barn"; photo: string; on: boolean }
  | { type: "slaa_sammen"; photos: string[] }
  | { type: "del_opp"; photo: string }
  | { type: "sider"; photo: string; pages: number | null }
  | { type: "forside"; photo: string | null }
  | { type: "bakside"; photo: string | null }
  | { type: "roter"; photo: string };

export type Analysefase =
  "venter" | "henter" | "hendelser" | "serier" | "velger" | "begrunnelser" | "ferdig";

export type Handling = "ta_med" | "ta_bort" | "bytt";

export type Svar =
  | "uskarpt"
  | "daarlig_lys"
  | "for_likt"
  | "for_mange_herfra"
  | "liker_ikke"
  | "privat"
  | "viktig_oyeblikk"
  | "viktig_person"
  | "fint_bilde"
  | "mangler_herfra"
  | "skarpere"
  | "gjenstand"
  | "uviktig_dag"
  | "stemning";

/** Roller i familieprofilen. */
export type Rolle = "barn" | "kjernefamilie" | "besteforeldre" | "naer_familie" | "venn" | "annen";

/** Et ansikt i et bilde: boksen er andeler (0–1) av bildet. */
export interface Ansikt {
  faceId: number;
  /** Bildet ansiktet er i. */
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

/** En gruppe ansikter appen mener er samme person. */
export interface Ansiktsgruppe {
  group: number;
  personId: number | null;
  name: string | null;
  role: Rolle | null;
  faces: number;
  photos: number;
  samples: Ansikt[];
}

export interface Person {
  id: number;
  name: string;
  role: Rolle;
}

export interface Personer {
  groups: Ansiktsgruppe[];
  persons: Person[];
  /** Bilder der ansiktene ikke er funnet ennå. */
  pending: number;
}

/** Tekstene i albumet: forsiden, første side inne og baksiden. */
export interface Albumtekst {
  title: string;
  subtitle: string;
  intro: string;
  back: string;
}

export interface TrykkFremdrift {
  done: number;
  total: number;
}

export interface Trykkfil {
  pages: number;
  photos: number;
  /** Bilder som ikke kunne leses (vises som grå felt). */
  missing: number;
  /** Bilder med for lav oppløsning for størrelsen i albumet. */
  lowResolution: number;
  megabytes: number;
}

/** Feil fra Rust-kjernen: en stabil kode og en teknisk melding. */
export interface Kommandofeil {
  kode: string;
  melding: string;
}

export function erKommandofeil(e: unknown): e is Kommandofeil {
  return typeof e === "object" && e !== null && "kode" in e;
}

export const api = {
  lagringStatus: () => invoke<LagringStatus>("vault_status"),
  opprettLagring: () => invoke<string>("vault_create"),
  aapneLagring: () => invoke<void>("vault_open"),
  gjenopprett: (kode: string) => invoke<void>("vault_recover", { kode }),
  slettAlleData: () => invoke<void>("delete_all_data"),

  kilder: () => invoke<Kilde[]>("list_sources"),
  forslag: () => invoke<Forslag[]>("suggested_sources"),
  leggTilKilde: (path: string, kind?: KildeType) => invoke<number>("add_source", { path, kind }),
  fjernKilde: (id: number) => invoke<void>("remove_source", { id }),
  velgMappe: async (): Promise<string | null> => {
    const valgt = await open({ directory: true, multiple: false });
    return typeof valgt === "string" ? valgt : null;
  },

  startInnlesing: () => invoke<void>("start_ingest"),
  avbrytInnlesing: () => invoke<void>("cancel_ingest"),
  innlesingPagar: () => invoke<boolean>("ingest_running"),
  paFremdrift: (cb: (p: Fremdrift) => void): Promise<UnlistenFn> =>
    listen<Fremdrift>("innlesing", (e) => cb(e.payload)),
  paFerdig: (cb: (r: InnlesingFerdig) => void): Promise<UnlistenFn> =>
    listen<InnlesingFerdig>("innlesing-ferdig", (e) => cb(e.payload)),

  sammendrag: () => invoke<Sammendrag>("catalog_summary"),
  aar: () => invoke<Aar[]>("list_years"),
  bilderIAar: (year: number) => invoke<Bilde[]>("photos_in_year", { year }),
  miniatyrUrl: (id: string) => convertFileSrc(id, "miniatyr"),

  /** `quiet`: lag utkastet på nytt uten gjennomgangen på skjermen (etter en endring). */
  lagUtkast: (year: number, pageCap: number | null = null, quiet = false) =>
    invoke<Utkast>("make_album_draft", { year, pageCap, quiet }),
  albumvalg: (year: number, change: Albumvalg) => invoke<void>("album_choice", { year, change }),
  paAnalyse: (cb: (fase: Analysefase) => void): Promise<UnlistenFn> =>
    listen<{ fase: Analysefase }>("analyse", (e) => cb(e.payload.fase)),
  /** Ved bytte er `id` bildet som tas med og `other` bildet som tas ut. */
  velgBilde: (year: number, action: Handling, id: string, other?: string) =>
    invoke<number>("choose_photo", { year, action, id, other: other ?? null }),
  svarHvorfor: (feedbackId: number, reason: Svar | null) =>
    invoke<Laert>("answer_why", { feedbackId, reason }),

  albumtekst: (year: number) => invoke<Albumtekst>("album_text", { year }),
  /** Spør hvor trykkfilen skal lagres. `null` hvis brukeren avbryter. */
  velgTrykkfil: async (navn: string): Promise<string | null> =>
    (await save({ defaultPath: navn, filters: [{ name: "PDF", extensions: ["pdf"] }] })) ?? null,
  lagTrykkfil: (year: number, pageCap: number | null, text: Albumtekst, path: string) =>
    invoke<Trykkfil>("export_album", { year, pageCap, text, path }),
  paTrykk: (cb: (p: TrykkFremdrift) => void): Promise<UnlistenFn> =>
    listen<TrykkFremdrift>("trykk", (e) => cb(e.payload)),

  personer: () => invoke<Personer>("face_groups"),
  /** Gir gruppen navn og rolle; med `personId` slås den inn i en person som finnes. */
  navngiGruppe: (group: number, name: string, role: Rolle, personId: number | null = null) =>
    invoke<Personer>("name_face_group", { group, name, role, personId }),
  ignorerGruppe: (group: number) => invoke<Personer>("ignore_face_group", { group }),
  /** Tar ett ansikt ut av gruppen (`personId` null) eller flytter det til en person. */
  flyttAnsikt: (faceId: number, personId: number | null) =>
    invoke<Personer>("move_face", { faceId, personId }),
};
