import type { CSSProperties, DragEvent } from "react";
import type { Felt, Side as SideData, UtkastBilde } from "../api";

/** Siden er 21 × 28 cm; feltene er i prosent av bredden og høyden. */
export const SIDE_FORHOLD = 210 / 280;

type Utseende = Pick<UtkastBilde, "rotation" | "focus" | "whole">;

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
  if (!b.rotation) return felles;
  if (b.rotation % 180 === 0) return { ...felles, transform: `rotate(${b.rotation}deg)` };
  // Kvart omdreining: bildet er like bredt som feltet er høyt, og omvendt.
  return {
    ...felles,
    position: "absolute",
    left: "50%",
    top: "50%",
    width: `${(100 / forhold).toFixed(3)}%`,
    height: `${(100 * forhold).toFixed(3)}%`,
    transform: `translate(-50%, -50%) rotate(${b.rotation}deg)`,
  };
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
        }`;
        return onPhoto ? (
          <button
            key={id}
            type="button"
            className={klasse}
            style={stil}
            aria-pressed={valgt === id || !!markert?.has(id)}
            aria-label={`${etikett}, bilde ${i + 1}`}
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
