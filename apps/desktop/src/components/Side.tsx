import type { CSSProperties, DragEvent, PointerEvent as ReactPointerEvent } from "react";
import type { Felt, Side as SideData, UtkastBilde } from "../api";
import { tekster } from "../tekster";

/** Siden er 21 × 28 cm; feltene er i prosent av bredden og høyden. */
export const SIDE_FORHOLD = 210 / 280;

type Utseende = Pick<UtkastBilde, "rotation" | "focus" | "whole" | "zoom">;

/**
 * Utsnittet (midtpunktet slik bildet vises) som `object-position` på det uroterte bildet.
 * Bildet roteres med CSS, så punktet må regnes tilbake.
 */
export function utsnittFoerRotering(rot: number, [x, y]: [number, number]): [number, number] {
  switch (((rot % 360) + 360) % 360) {
    case 90:
      return [y, 1 - x];
    case 180:
      return [1 - x, 1 - y];
    case 270:
      return [1 - y, x];
    default:
      return [x, y];
  }
}

/**
 * Stilen til bildet i et felt: fyller feltet (med utsnitt) eller viser hele bildet, og
 * roterer det som brukeren har valgt. `forhold` er feltets bredde/høyde på skjermen.
 */
export function bildeStil(b: Utseende, fill: boolean, forhold: number): CSSProperties {
  const [x, y] = utsnittFoerRotering(b.rotation, b.focus);
  const felles: CSSProperties = {
    objectFit: fill ? "cover" : "contain",
    objectPosition: `${(x * 100).toFixed(1)}% ${(y * 100).toFixed(1)}%`,
  };
  const z = fill && !b.whole ? Math.min(4, Math.max(1, b.zoom ?? 1)) : 1;
  if (!b.rotation && z === 1) return felles;
  if (b.rotation % 180 === 0 && z === 1) {
    return { ...felles, transform: `rotate(${b.rotation}deg)` };
  }
  // Kvart omdreining: bildet er like bredt som feltet er høyt, og omvendt. Forstørret: boksen
  // er `z` ganger større og flyttes så utsnittet blir som i trykkfilen (der punktet `focus`
  // ligger like langt inn i feltet som i bildet).
  const kvart = b.rotation % 180 !== 0;
  const [fx, fy] = b.focus;
  return {
    ...felles,
    position: "absolute",
    left: `${(50 + (1 - z) * (fx - 0.5) * 100).toFixed(3)}%`,
    top: `${(50 + (1 - z) * (fy - 0.5) * 100).toFixed(3)}%`,
    width: `${((kvart ? 100 / forhold : 100) * z).toFixed(3)}%`,
    height: `${((kvart ? 100 * forhold : 100) * z).toFixed(3)}%`,
    maxWidth: "none",
    transform: `translate(-50%, -50%)${b.rotation ? ` rotate(${b.rotation}deg)` : ""}`,
  };
}

/**
 * Nytt utsnitt når brukeren drar bildet i et felt som fylles. `bilde` og `ramme` er
 * bredde/høyde, `zoom` forstørrelsen og `dra` hvor langt bildet er dratt, i andeler av feltet.
 * Bildet følger fingeren: dras det mot høyre, ser man mer av venstre side.
 */
export function flyttUtsnitt(
  bilde: number,
  ramme: number,
  zoom: number,
  focus: [number, number],
  dra: [number, number],
): [number, number] {
  // Hvor mange ganger bredere og høyere enn feltet bildet tegnes.
  const sx = Math.max(bilde / ramme, 1) * zoom;
  const sy = Math.max(ramme / bilde, 1) * zoom;
  const akse = (s: number, f: number, d: number) =>
    s <= 1.0001 ? f : Math.min(1, Math.max(0, f + d / (1 - s)));
  return [akse(sx, focus[0], dra[0]), akse(sy, focus[1], dra[1])];
}

const pst = (v: number) => `${v.toFixed(3)}%`;

/** Feltene stemmer med bildene (sider som er endret lokalt, mangler feltene til de lages). */
/** Dra og slipp mellom sidene. */
export interface Dra {
  start: (id: string) => void;
  slipp: (foran: string | null) => void;
}

/** Felles for alt som kan slippes på: godta slipp, og si hvor bildet havner. */
const slippMaal = (dra: Dra | undefined, foran: string | null) =>
  dra
    ? {
        onDragOver: (e: DragEvent) => e.preventDefault(),
        onDrop: (e: DragEvent) => {
          e.preventDefault();
          e.stopPropagation();
          dra.slipp(foran);
        },
      }
    : {};

export const harFelt = (s: SideData): s is SideData & { frames: Felt[] } =>
  !!s.frames && s.frames.length === s.photos.length;

/**
 * En side slik den blir trykt: bildene i feltene fra Rust (`p2a_print::layout`). Uten felt
 * (siden er endret lokalt) vises et enkelt rutenett til utkastet lages på nytt.
 */
export function Sidebilde({
  side,
  bilder,
  thumbUrl,
  etikett,
  valgt,
  markert,
  onPhoto,
  dra,
  varsel,
}: {
  side: SideData;
  bilder: Map<string, UtkastBilde>;
  thumbUrl: (id: string) => string;
  etikett: string;
  valgt?: string | null;
  markert?: Set<string>;
  /** Klikk på et bilde på siden (med Ctrl/Cmd/Shift: marker). */
  onPhoto?: (id: string, marker: boolean) => void;
  /** Dra og slipp: et bilde dras fra siden, eller slippes foran et bilde (`null` = sist). */
  dra?: Dra;
  /** Kvalitetssjekken: bildene på siden som bør ses på før trykk. */
  varsel?: Set<string>;
}) {
  const utseende = (id: string): Utseende =>
    bilder.get(id) ?? { rotation: 0, focus: [0.5, 0.5], whole: false };
  if (!harFelt(side)) {
    return (
      <div
        className={`pg pg--${side.kind}${side.locked ? " pg--locked" : ""}`}
        style={side.kolonner ? { gridTemplateColumns: `repeat(${side.kolonner}, 1fr)` } : undefined}
        role="img"
        aria-label={etikett}
        {...slippMaal(dra, null)}
      >
        {side.photos.map((id) => (
          <img
            key={id}
            src={thumbUrl(id)}
            alt=""
            loading="lazy"
            decoding="async"
            style={bildeStil(utseende(id), true, 1)}
          />
        ))}
      </div>
    );
  }
  return (
    <div
      className={`pg2${side.locked ? " pg2--locked" : ""}`}
      role={onPhoto ? "group" : "img"}
      aria-label={etikett}
      {...slippMaal(dra, null)}
    >
      {side.photos.map((id, i) => {
        const f = side.frames[i]!;
        const forhold = (f.w / f.h) * SIDE_FORHOLD;
        const stil: CSSProperties = {
          left: pst(f.x),
          top: pst(f.y),
          width: pst(f.w),
          height: pst(f.h),
        };
        const img = (
          <img
            src={thumbUrl(id)}
            alt=""
            loading="lazy"
            decoding="async"
            style={bildeStil(utseende(id), f.fill, forhold)}
          />
        );
        const klasse = `pg2__fr${valgt === id ? " pg2__fr--valgt" : ""}${
          markert?.has(id) ? " pg2__fr--markert" : ""
        }${varsel?.has(id) ? " pg2__fr--varsel" : ""}`;
        return onPhoto ? (
          <button
            key={id}
            type="button"
            className={klasse}
            style={stil}
            aria-pressed={valgt === id || !!markert?.has(id)}
            aria-label={`${etikett}, bilde ${i + 1}${varsel?.has(id) ? `, ${tekster.sjekk.merke}` : ""}`}
            title={varsel?.has(id) ? tekster.sjekk.merke : undefined}
            onClick={(e) => onPhoto(id, e.ctrlKey || e.metaKey || e.shiftKey)}
            draggable={!!dra}
            onDragStart={(e) => {
              e.dataTransfer?.setData("text/plain", id);
              dra?.start(id);
            }}
            {...slippMaal(dra, id)}
          >
            {img}
          </button>
        ) : (
          <span key={id} className={klasse} style={stil}>
            {img}
          </span>
        );
      })}
    </div>
  );
}

/** Liten tegning av en ramme (til rammevalget). */
export function RammeIkon({ felt }: { felt: Felt[] }) {
  return (
    <svg viewBox="0 0 30 40" aria-hidden="true" className="tpl__ic">
      <rect x=".5" y=".5" width="29" height="39" rx="1.5" fill="none" stroke="currentColor" />
      {felt.map((r, i) => {
        const x = Math.max(0, r.x) * 0.3;
        const y = Math.max(0, r.y) * 0.4;
        return (
          <rect
            key={i}
            x={x.toFixed(2)}
            y={y.toFixed(2)}
            width={(Math.min(100, r.x + r.w) * 0.3 - x).toFixed(2)}
            height={(Math.min(100, r.y + r.h) * 0.4 - y).toFixed(2)}
            fill="currentColor"
            opacity=".55"
          />
        );
      })}
    </svg>
  );
}

/**
 * Dra i et felt for å flytte bildet i det. Gir hendelsene feltet trenger; `onMove` kalles mens
 * brukeren drar, `onEnd` når bildet slippes (bare om det faktisk ble flyttet).
 */
export function draUtsnitt(
  start: [number, number],
  maal: { bilde: number; ramme: number; zoom: number },
  onMove: (f: [number, number]) => void,
  onEnd: (f: [number, number]) => void,
) {
  return (e: ReactPointerEvent<HTMLElement>) => {
    if (e.button !== 0) return;
    const el = e.currentTarget;
    const r = el.getBoundingClientRect();
    if (!r.width || !r.height) return;
    const [x0, y0] = [e.clientX, e.clientY];
    let siste = start;
    let flyttet = false;
    el.setPointerCapture?.(e.pointerId);
    const flytt = (ev: PointerEvent) => {
      const dx = (ev.clientX - x0) / r.width;
      const dy = (ev.clientY - y0) / r.height;
      if (!flyttet && Math.hypot(ev.clientX - x0, ev.clientY - y0) < 4) return;
      flyttet = true;
      siste = flyttUtsnitt(maal.bilde, maal.ramme, maal.zoom, start, [dx, dy]);
      onMove(siste);
    };
    const slipp = () => {
      el.removeEventListener("pointermove", flytt);
      el.removeEventListener("pointerup", slipp);
      el.removeEventListener("pointercancel", slipp);
      if (flyttet) {
        // Klikket som følger etter et drag, skal ikke telle.
        const stopp = (c: Event) => c.stopPropagation();
        el.addEventListener("click", stopp, { capture: true, once: true });
        setTimeout(() => el.removeEventListener("click", stopp, { capture: true }), 0);
        onEnd(siste);
      }
    };
    el.addEventListener("pointermove", flytt);
    el.addEventListener("pointerup", slipp);
    el.addEventListener("pointercancel", slipp);
  };
}
