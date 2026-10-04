/**
 * Rene hjelpefunksjoner for utkastet: oppdatering etter brukerens valg, og når appen skal
 * spørre «Hvorfor?». Holdt utenfor komponentene så de kan testes direkte.
 */
import type { Handling, Side, Svar, Utkast, UtkastBilde } from "./api";
import { maaneder } from "./tekster";

/** Svarene appen tilbyr på «Hvorfor?», per handling. Teksten ligger i tekster.ts. */
export const HVORFOR_VALG: Record<Handling, Svar[]> = {
  ta_bort: [
    "gjenstand",
    "uskarpt",
    "daarlig_lys",
    "for_likt",
    "for_mange_herfra",
    "liker_ikke",
    "privat",
  ],
  ta_med: ["viktig_oyeblikk", "viktig_person", "fint_bilde", "stemning", "mangler_herfra"],
  bytt: ["skarpere", "viktig_oyeblikk", "viktig_person", "fint_bilde", "for_likt", "liker_ikke"],
};

/** Oppdaterer utkastet lokalt etter et valg, uten å lage det på nytt. */
export function brukValg(u: Utkast, action: Handling, id: string, other?: string): Utkast {
  const endret = new Map<string, Partial<UtkastBilde>>();
  if (action === "ta_med" || action === "bytt")
    endret.set(id, { included: true, reason: { kode: "valgt_av_deg" } });
  if (action === "ta_bort")
    endret.set(id, { included: false, reason: { kode: "valgt_bort_av_deg" } });
  if (action === "bytt" && other)
    endret.set(other, { included: false, reason: { kode: "valgt_bort_av_deg" } });
  const photos = u.photos.map((p) => {
    const e = endret.get(p.id);
    return e ? { ...p, ...e } : p;
  });
  const events = telt(u, photos);
  return { ...u, photos, events, pages: events.reduce((sum, e) => sum + e.pages, 0) };
}

function telt(u: Utkast, photos: UtkastBilde[]) {
  const med = new Array<number>(u.events.length).fill(0);
  for (const p of photos) if (p.included) med[p.event] = (med[p.event] ?? 0) + 1;
  return u.events.map((e, i) => {
    const included = med[i] ?? 0;
    return { ...e, included, pages: included === 0 ? 0 : Math.max(1, Math.ceil(included / 4)) };
  });
}

/**
 * Sidene etter brukerens valg, før utkastet lages på nytt: et bilde som er byttet inn tar
 * plassen til et som er tatt bort, og bilder som er lagt til havner på en side til slutt.
 */
export function oppdaterSider(sider: Side[], med: string[]): { sider: Side[]; endret: boolean } {
  const erMed = new Set(med);
  const iSider = new Set(sider.flatMap((s) => s.photos));
  const nye = med.filter((id) => !iSider.has(id));
  let endret = nye.length > 0;
  const ut: Side[] = [];
  for (const s of sider) {
    const photos: string[] = [];
    for (const id of s.photos) {
      if (erMed.has(id)) photos.push(id);
      else {
        endret = true;
        const inn = nye.shift();
        if (inn) photos.push(inn);
      }
    }
    if (photos.length > 0) ut.push({ ...s, photos });
  }
  if (nye.length > 0) {
    ut.push({ kind: "rutenett", kolonner: nye.length > 4 ? 3 : 2, photos: nye });
  }
  return { sider: ut, endret };
}

/**
 * Spør forsiktig: de tre første gangene, deretter hver tredje handling. Har brukeren hoppet
 * over tre ganger på rad, spør appen ikke mer før neste gang utkastet åpnes.
 */
export function spoerOmHvorfor(handlinger: number, hoppetOverPaRad: number): boolean {
  if (hoppetOverPaRad >= 3) return false;
  return handlinger <= 3 || handlinger % 3 === 0;
}

/** «14. juli», «14.–16. juli» eller «30. juni–2. juli». */
export function datoSpenn(start: string, slutt: string): string {
  const [, m1, d1] = start.slice(0, 10).split("-").map(Number);
  const [, m2, d2] = slutt.slice(0, 10).split("-").map(Number);
  const mnd = (m: number | undefined) => maaneder[(m ?? 1) - 1];
  if (m1 === m2 && d1 === d2) return `${d1}. ${mnd(m1)}`;
  if (m1 === m2) return `${d1}.–${d2}. ${mnd(m2)}`;
  return `${d1}. ${mnd(m1)}–${d2}. ${mnd(m2)}`;
}
