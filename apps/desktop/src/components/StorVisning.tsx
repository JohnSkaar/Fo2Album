import { useEffect, useRef, useState } from "react";
import type { Albumvalg, Felt, UtkastBilde } from "../api";
import { datoTekst, tekster } from "../tekster";
import { Lukk } from "./Lukk";
import { Lupe } from "./Lupe";
import { bildeStil, draUtsnitt, SIDE_FORHOLD } from "./Side";

const t = tekster.redigering;

/** Formen på bildet slik det vises (etter brukerens rotering). */
export function bildeForhold(b: Pick<UtkastBilde, "width" | "height" | "rotation">): number {
  const a = b.width && b.height ? b.width / b.height : 4 / 3;
  return b.rotation % 180 === 90 ? 1 / a : a;
}

const klem = (v: number) => Math.min(1, Math.max(0, v));

/**
 * Nytt utsnitt når brukeren klikker i rammen: punktet brukeren klikket på, flyttes så nær midten
 * av rammen som mulig. `bilde` og `ramme` er bredde/høyde, `klikk` andeler av rammen.
 */
export function nyttUtsnitt(
  bilde: number,
  ramme: number,
  utsnitt: [number, number],
  klikk: [number, number],
  zoom = 1,
): [number, number] {
  // Hvor stor del av bildet som synes i hver retning når det fyller rammen.
  const synligX = (bilde > ramme ? ramme / bilde : 1) / zoom;
  const synligY = (bilde > ramme ? 1 : bilde / ramme) / zoom;
  const akse = (synlig: number, fokus: number, k: number) => {
    if (synlig >= 1) return fokus;
    const start = (1 - synlig) * fokus;
    const punkt = start + k * synlig;
    return klem((punkt - synlig / 2) / (1 - synlig));
  };
  return [akse(synligX, utsnitt[0], klikk[0]), akse(synligY, utsnitt[1], klikk[1])];
}

/** Glidebryter for hvor stort bildet er i feltet (100–400 %). */
export function Storrelse({
  zoom,
  onChange,
  onCommit,
}: {
  zoom: number;
  onChange: (z: number) => void;
  /** Når brukeren slipper glidebryteren. */
  onCommit: (z: number) => void;
}) {
  return (
    <label className="zoomfelt">
      <span>{t.storrelseIFelt}</span>
      <input
        type="range"
        min={100}
        max={400}
        step={5}
        value={Math.round(zoom * 100)}
        aria-valuetext={`${Math.round(zoom * 100)} %`}
        onChange={(e) => onChange(Number(e.target.value) / 100)}
        onPointerUp={(e) => onCommit(Number(e.currentTarget.value) / 100)}
        onKeyUp={(e) => onCommit(Number(e.currentTarget.value) / 100)}
      />
      <output>{Math.round(zoom * 100)} %</output>
    </label>
  );
}

/** Stor visning av ett bilde, med rotering og utsnitt for rammen det står i. */
export function StorVisning({
  b,
  felt,
  startUtsnitt,
  stortUrl,
  thumbUrl,
  fulltUrl,
  onOpenOriginal,
  onChange,
  onClose,
}: {
  b: UtkastBilde;
  /** Feltet bildet står i på siden, hvis det er med. */
  felt: Felt | null;
  startUtsnitt?: boolean;
  stortUrl: (id: string) => string;
  thumbUrl: (id: string) => string;
  /** Bildet i full oppløsning (til «Full størrelse»). */
  fulltUrl?: (id: string) => string;
  onOpenOriginal?: (id: string) => void;
  onChange: (c: Albumvalg, melding?: string) => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const [utsnitt, setUtsnitt] = useState(!!startUtsnitt);
  const [fokus, setFokus] = useState<[number, number]>(b.focus);
  const [zoom, setZoom] = useState(b.zoom ?? 1);
  const [stortFeilet, setStortFeilet] = useState(false);
  const [lupe, setLupe] = useState(false);
  useEffect(() => {
    const d = ref.current;
    if (!d || d.open) return;
    if (d.showModal) d.showModal();
    else d.setAttribute("open", "");
  }, []);

  const kanUtsnitt = !!felt && (felt.fill || b.whole);
  const rammeForhold = felt ? (felt.w / felt.h) * SIDE_FORHOLD : 1;
  const src = stortFeilet ? thumbUrl(b.id) : stortUrl(b.id);
  const lagre = (focus: [number, number] | null, whole: boolean, z = zoom) =>
    onChange(
      { type: "utsnitt", photo: b.id, focus, whole, zoom: whole ? null : Math.round(z * 100) },
      t.utsnittLagret,
    );
  const somProsent = (f: [number, number]): [number, number] => [
    Math.round(f[0] * 100),
    Math.round(f[1] * 100),
  ];

  return (
    <dialog
      ref={ref}
      className="lb"
      aria-label={t.visStort}
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
    >
      <Lukk onClick={onClose} fast />
      <div className="lb__img">
        {utsnitt && felt ? (
          <button
            type="button"
            className="lb__crop"
            style={{ aspectRatio: rammeForhold.toFixed(4) }}
            aria-label={t.utsnittHjelp}
            onPointerDown={
              b.whole
                ? undefined
                : draUtsnitt(
                    fokus,
                    { bilde: bildeForhold(b), ramme: rammeForhold, zoom },
                    setFokus,
                    (f) => lagre(somProsent(f), false),
                  )
            }
            onClick={(e) => {
              if (b.whole) return;
              const r = e.currentTarget.getBoundingClientRect();
              if (!r.width || !r.height) return;
              const ny = nyttUtsnitt(
                bildeForhold(b),
                rammeForhold,
                fokus,
                [klem((e.clientX - r.left) / r.width), klem((e.clientY - r.top) / r.height)],
                zoom,
              );
              setFokus(ny);
              lagre(somProsent(ny), false);
            }}
          >
            <img
              src={src}
              alt=""
              draggable={false}
              onError={() => setStortFeilet(true)}
              style={bildeStil({ ...b, focus: fokus, zoom }, !b.whole, rammeForhold)}
            />
          </button>
        ) : fulltUrl ? (
          <button
            type="button"
            className="lb__whole lb__whole--zoom"
            style={{ aspectRatio: bildeForhold(b).toFixed(4) }}
            aria-label={tekster.lupe.seFullt}
            title={tekster.lupe.seFulltHjelp}
            onClick={() => setLupe(true)}
          >
            <img
              src={src}
              alt=""
              onError={() => setStortFeilet(true)}
              style={bildeStil(b, false, bildeForhold(b))}
            />
          </button>
        ) : (
          <span className="lb__whole" style={{ aspectRatio: bildeForhold(b).toFixed(4) }}>
            <img
              src={src}
              alt=""
              onError={() => setStortFeilet(true)}
              style={bildeStil(b, false, bildeForhold(b))}
            />
          </span>
        )}
      </div>
      <div className="lb__side">
        <b>{datoTekst(b.takenAt)}</b>
        {utsnitt ? (
          <>
            <p className="dock__help">{t.utsnittHjelp}</p>
            {!b.whole && (
              <Storrelse
                zoom={zoom}
                onChange={setZoom}
                onCommit={(z) => lagre(somProsent(fokus), false, z)}
              />
            )}
            <div className="lb__acts">
              <button
                type="button"
                className="btn btn--secondary btn--sm"
                aria-pressed={!b.whole}
                onClick={() => lagre(null, false)}
              >
                {t.fyll}
              </button>
              <button
                type="button"
                className="btn btn--secondary btn--sm"
                aria-pressed={b.whole}
                onClick={() => lagre(null, true)}
              >
                {t.hele}
              </button>
              <button type="button" className="btn btn--primary btn--sm" onClick={onClose}>
                {t.ferdig}
              </button>
            </div>
          </>
        ) : (
          <div className="lb__acts">
            <button
              type="button"
              className="btn btn--secondary btn--sm"
              onClick={() => onChange({ type: "roter", photo: b.id })}
            >
              ↻ {tekster.utkast.roter}
            </button>
            {fulltUrl && (
              <button
                type="button"
                className="btn btn--secondary btn--sm"
                title={tekster.lupe.seFulltHjelp}
                onClick={() => setLupe(true)}
              >
                🔍 {tekster.lupe.seFullt}
              </button>
            )}
            {kanUtsnitt && (
              <button
                type="button"
                className="btn btn--secondary btn--sm"
                onClick={() => setUtsnitt(true)}
              >
                {t.utsnitt}
              </button>
            )}
          </div>
        )}
        <p className="lb__note">{t.forhaand}</p>
      </div>
      {lupe && fulltUrl && (
        <Lupe
          src={fulltUrl(b.id)}
          reserve={thumbUrl(b.id)}
          rotation={b.rotation}
          tittel={`${tekster.lupe.tittel}, ${datoTekst(b.takenAt)}`}
          onOpenOriginal={onOpenOriginal && (() => onOpenOriginal(b.id))}
          onClose={() => setLupe(false)}
        />
      )}
    </dialog>
  );
}
