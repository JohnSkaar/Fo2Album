/**
 * Alle kall til Rust-kjernen. Grensesnittet gjør aldri nettverkskall; alt går via IPC.
 * Testene erstatter denne modulen (vi.mock), så komponentene kan testes uten Tauri.
 */
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

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
  phase: "skanner" | "leser" | "dubletter";
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
};
