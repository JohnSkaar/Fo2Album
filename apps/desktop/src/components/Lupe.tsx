import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { tekster } from "../tekster";
import { Lukk } from "./Lukk";

const t = tekster.lupe;

/** Et område i bildet som andeler (0–1) av bredden og høyden, f.eks. et ansikt. */
export interface Boks {
  x: number;
  y: number;
  w: number;
  h: number;
}

const MIN = 0.05;
const MAX = 4;
const TRINN = [0.1, 0.25, 0.5, 0.75, 1, 1.5, 2, 3, 4];

/** Neste zoomtrinn opp eller ned fra `z`. */
export function nesteZoom(z: number, retning: 1 | -1): number {
  if (retning === 1) return TRINN.find((s) => s > z + 1e-6) ?? MAX;
  return [...TRINN].reverse().find((s) => s < z - 1e-6) ?? MIN;
}

/**
 * Zoom som får plass til `del` av bildet (andeler, i bildets punkter `w × h`) i et vindu på
 * `vw × vh` punkter, høyst `maks` (100 %).
 */
export function zoomFor(w: number, h: number, vw: number, vh: number, del = 1, maks = 1): number {
  if (!w || !h || !vw || !vh) return 1;
  return Math.min(maks, vw / (w * del), vh / (h * del));
}

/**
 * Bildet i full oppløsning, så brukeren kan se om det er skarpt og hvem som er med. Starter
 * tilpasset skjermen (eller zoomet inn på `boks`), og kan vises i 100 % eller større. Bildet
 * leses fra originalfilen på maskinen og sendes ingen steder.
 */
export function Lupe({
  src,
  reserve,
  rotation = 0,
  boks,
  tittel,
  onOpenOriginal,
  onClose,
}: {
  /** Bildet i full størrelse. */
  src: string;
  /** Vises mens det store bildet hentes, og om det ikke finnes. */
  reserve: string;
  /** Brukerens rotering med klokka (0, 90, 180, 270). */
  rotation?: number;
  boks?: Boks;
  tittel: string;
  onOpenOriginal?: () => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const vindu = useRef<HTMLDivElement>(null);
  const [nat, setNat] = useState<{ w: number; h: number } | null>(null);
  const [feil, setFeil] = useState(false);
  const [zoom, setZoom] = useState<number | null>(null);
  // Punktet (andeler av bildet) som skal stå midt i vinduet etter neste zoom.
  const midt = useRef<[number, number]>(
    boks ? [boks.x + boks.w / 2, boks.y + boks.h / 2] : [0.5, 0.5],
  );
  useEffect(() => {
    const d = ref.current;
    if (!d || d.open) return;
    if (d.showModal) d.showModal();
    else d.setAttribute("open", "");
  }, []);

  const kvart = rotation % 180 !== 0;
  // Bildets mål slik det vises (etter rotering).
  const vis = useMemo(() => (nat ? (kvart ? { w: nat.h, h: nat.w } : nat) : null), [nat, kvart]);

  const tilpass = useCallback(() => {
    const v = vindu.current;
    if (!vis || !v) return;
    midt.current = [0.5, 0.5];
    setZoom(zoomFor(vis.w, vis.h, v.clientWidth - 32, v.clientHeight - 32));
  }, [vis]);

  // Første visning: tilpasset skjermen, eller ansiktet med god plass rundt.
  useEffect(() => {
    const v = vindu.current;
    if (!vis || !v || zoom !== null) return;
    const del = boks ? Math.max(boks.w, boks.h) * 3.5 : 1;
    // Et lite ansikt kan forstørres litt, så det er lett å se hvem det er.
    const maks = boks ? 2 : 1;
    setZoom(
      Math.max(MIN, zoomFor(vis.w, vis.h, v.clientWidth - 32, v.clientHeight - 32, del, maks)),
    );
  }, [vis, zoom, boks]);

  // Etter zoom: rull så punktet brukeren så på, står midt i vinduet.
  useLayoutEffect(() => {
    const v = vindu.current;
    if (!v || !vis || zoom === null) return;
    const [mx, my] = midt.current;
    v.scrollLeft = mx * vis.w * zoom + 16 - v.clientWidth / 2;
    v.scrollTop = my * vis.h * zoom + 16 - v.clientHeight / 2;
  }, [zoom, vis]);

  const endre = (z: number) => {
    const v = vindu.current;
    if (v && vis && zoom) {
      midt.current = [
        (v.scrollLeft + v.clientWidth / 2 - 16) / (vis.w * zoom),
        (v.scrollTop + v.clientHeight / 2 - 16) / (vis.h * zoom),
      ];
    }
    setZoom(Math.min(MAX, Math.max(MIN, z)));
  };

  const z = zoom ?? 0;
  const bredde = vis ? vis.w * z : 0;
  const hoyde = vis ? vis.h * z : 0;
  return (
    <dialog
      ref={ref}
      className="lupe"
      aria-label={tittel}
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
      onKeyDown={(e) => {
        if (e.key === "+" || e.key === "=") endre(nesteZoom(z, 1));
        if (e.key === "-") endre(nesteZoom(z, -1));
        if (e.key === "0") tilpass();
        if (e.key === "1") endre(1);
      }}
    >
      <div className="lupe__top">
        <Lukk onClick={onClose} />
        <span className="lupe__info" role="status">
          {nat
            ? `${t.punkter(nat.w, nat.h)} · ${t.zoom(Math.round(z * 100))}`
            : feil
              ? t.feil
              : t.henter}
        </span>
        <div className="lupe__knapper">
          <button
            type="button"
            className="btn btn--secondary btn--sm"
            aria-label={t.mindre}
            disabled={!nat}
            onClick={() => endre(nesteZoom(z, -1))}
          >
            −
          </button>
          <button
            type="button"
            className="btn btn--secondary btn--sm"
            aria-label={t.storre}
            disabled={!nat}
            onClick={() => endre(nesteZoom(z, 1))}
          >
            +
          </button>
          <button
            type="button"
            className="btn btn--secondary btn--sm"
            disabled={!nat}
            onClick={tilpass}
          >
            {t.tilpass}
          </button>
          <button
            type="button"
            className="btn btn--secondary btn--sm"
            aria-pressed={z === 1}
            disabled={!nat}
            onClick={() => endre(1)}
          >
            {t.full}
          </button>
          {onOpenOriginal && (
            <button
              type="button"
              className="btn btn--secondary btn--sm"
              title={t.aapneHjelp}
              onClick={onOpenOriginal}
            >
              {t.aapne}
            </button>
          )}
        </div>
      </div>
      <div ref={vindu} className="lupe__vindu">
        {!nat && <img className="lupe__reserve" src={reserve} alt="" />}
        <div
          className="lupe__lerret"
          style={vis && zoom !== null ? { width: bredde, height: hoyde } : { display: "none" }}
          onClick={(e) => {
            if (!vis || !zoom) return;
            const r = e.currentTarget.getBoundingClientRect();
            const p: [number, number] = [
              (e.clientX - r.left) / r.width,
              (e.clientY - r.top) / r.height,
            ];
            // Klikk: 100 % rundt punktet, eller tilbake til hele bildet.
            if (z < 1) {
              midt.current = p;
              setZoom(1);
            } else tilpass();
          }}
        >
          <img
            src={src}
            alt=""
            draggable={false}
            onLoad={(e) =>
              setNat({ w: e.currentTarget.naturalWidth, h: e.currentTarget.naturalHeight })
            }
            onError={() => setFeil(true)}
            style={
              nat
                ? {
                    width: nat.w * z,
                    height: nat.h * z,
                    transform: `translate(-50%, -50%) rotate(${rotation}deg)`,
                  }
                : undefined
            }
          />
          {boks && (
            <span
              className="lupe__boks"
              aria-hidden="true"
              style={{
                left: `${boks.x * 100}%`,
                top: `${boks.y * 100}%`,
                width: `${boks.w * 100}%`,
                height: `${boks.h * 100}%`,
              }}
            />
          )}
        </div>
      </div>
      {nat && <p className="lupe__hjelp">{t.hjelp}</p>}
    </dialog>
  );
}
