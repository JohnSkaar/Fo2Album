import { useEffect, useRef, useState } from "react";
import type { Albumvalg, Felt, UtkastBilde } from "../api";
import { datoTekst, tekster } from "../tekster";
import { bildeStil, SIDE_FORHOLD } from "./Side";

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
): [number, number] {
  // Hvor stor del av bildet som synes i hver retning når det fyller rammen.
  const synligX = bilde > ramme ? ramme / bilde : 1;
  const synligY = bilde > ramme ? 1 : bilde / ramme;
  const akse = (synlig: number, fokus: number, k: number) => {
    if (synlig >= 1) return fokus;
    const start = (1 - synlig) * fokus;
    const punkt = start + k * synlig;
    return klem((punkt - synlig / 2) / (1 - synlig));
  };
  return [akse(synligX, utsnitt[0], klikk[0]), akse(synligY, utsnitt[1], klikk[1])];
}

/** Stor visning av ett bilde, med rotering og utsnitt for rammen det står i. */
export function StorVisning({
  b,
  felt,
  startUtsnitt,
  stortUrl,
  thumbUrl,
  onChange,
  onClose,
}: {
  b: UtkastBilde;
  /** Feltet bildet står i på siden, hvis det er med. */
  felt: Felt | null;
  startUtsnitt?: boolean;
  stortUrl: (id: string) => string;
  thumbUrl: (id: string) => string;
  onChange: (c: Albumvalg, melding?: string) => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const [utsnitt, setUtsnitt] = useState(!!startUtsnitt);
  const [fokus, setFokus] = useState<[number, number]>(b.focus);
  const [stortFeilet, setStortFeilet] = useState(false);
  useEffect(() => {
    const d = ref.current;
    if (!d || d.open) return;
    if (d.showModal) d.showModal();
    else d.setAttribute("open", "");
  }, []);

  const kanUtsnitt = !!felt && (felt.fill || b.whole);
  const rammeForhold = felt ? (felt.w / felt.h) * SIDE_FORHOLD : 1;
  const src = stortFeilet ? thumbUrl(b.id) : stortUrl(b.id);
  const lagre = (focus: [number, number] | null, whole: boolean) =>
    onChange({ type: "utsnitt", photo: b.id, focus, whole }, t.utsnittLagret);

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
      <div className="lb__img">
        {utsnitt && felt ? (
          <button
            type="button"
            className="lb__crop"
            style={{ aspectRatio: rammeForhold.toFixed(4) }}
            aria-label={t.utsnittHjelp}
            onClick={(e) => {
              if (b.whole) return;
              const r = e.currentTarget.getBoundingClientRect();
              if (!r.width || !r.height) return;
              const ny = nyttUtsnitt(bildeForhold(b), rammeForhold, fokus, [
                klem((e.clientX - r.left) / r.width),
                klem((e.clientY - r.top) / r.height),
              ]);
              setFokus(ny);
              lagre([Math.round(ny[0] * 100), Math.round(ny[1] * 100)], false);
            }}
          >
            <img
              src={src}
              alt=""
              onError={() => setStortFeilet(true)}
              style={bildeStil({ ...b, focus: fokus }, !b.whole, rammeForhold)}
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
        {!utsnitt && (
          <button type="button" className="btn btn--primary" onClick={onClose} autoFocus>
            {tekster.utkast.lukk}
          </button>
        )}
      </div>
    </dialog>
  );
}
