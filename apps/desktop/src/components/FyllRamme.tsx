import { type DragEvent, useEffect, useRef, useState } from "react";
import type { Ramme, UtkastBilde } from "../api";
import { datoTekst, tekster } from "../tekster";
import { bildeStil, RammeIkon, SIDE_FORHOLD } from "./Side";

const t = tekster.fyll;

/** Et bilde brukeren kan sette i rammen, med hvor det står nå («Side 3», «Ikke med»). */
export interface Kandidat {
  id: string;
  hvor: string;
}

/** Hva som skjer med bildene på siden som ikke får plass i rammen. */
export type Rest = "ta_med" | "ta_bort";

/**
 * Plasserer `id` i felt `i`. Står bildet alt i et annet felt, bytter de to feltene plass; et
 * bilde som var i felt `i`, går ellers ut av rammen.
 */
export function plasser(felt: (string | null)[], i: number, id: string): (string | null)[] {
  const ut = [...felt];
  const j = ut.indexOf(id);
  if (j >= 0) ut[j] = ut[i] ?? null;
  ut[i] = id;
  return ut;
}

/** Bytter innholdet i to felt. */
export function bytt(felt: (string | null)[], a: number, b: number): (string | null)[] {
  const ut = [...felt];
  [ut[a], ut[b]] = [ut[b] ?? null, ut[a] ?? null];
  return ut;
}

const pst = (v: number) => `${v.toFixed(3)}%`;
const DRA = "application/x-fo2album-felt";

/**
 * Fyll en fast ramme: velg hvilke bilder som skal stå i den og i hvilket felt. Bildene kan
 * hentes fra resten av historien, også bilder som ikke er med ennå.
 */
export function FyllRamme({
  ramme,
  start,
  kandidater,
  bilder,
  thumbUrl,
  onDone,
  onClose,
}: {
  ramme: Ramme;
  /** Bildene på siden nå, i rekkefølge. */
  start: string[];
  kandidater: Kandidat[];
  bilder: Map<string, UtkastBilde>;
  thumbUrl: (id: string) => string;
  /** Bildene i feltene, i rammens rekkefølge, og hva som skjer med bildene som ikke fikk plass. */
  onDone: (photos: string[], ute: string[], rest: Rest) => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const n = ramme.photos;
  const [felt, setFelt] = useState<(string | null)[]>(() =>
    Array.from({ length: n }, (_, i) => start[i] ?? null),
  );
  const [valgt, setValgt] = useState<number | null>(null);
  const [rest, setRest] = useState<Rest>("ta_med");
  const [melding, setMelding] = useState<string | null>(null);
  useEffect(() => {
    const d = ref.current;
    if (!d || d.open) return;
    if (d.showModal) d.showModal();
    else d.setAttribute("open", "");
  }, []);

  const fylt = felt.filter((x): x is string => !!x);
  const ute = start.filter((id) => !fylt.includes(id));
  const ledig = felt.indexOf(null);

  const velgBilde = (id: string) => {
    setMelding(null);
    if (valgt !== null) {
      setFelt(plasser(felt, valgt, id));
      setValgt(null);
    } else if (felt.includes(id)) {
      setFelt(felt.map((x) => (x === id ? null : x)));
    } else if (ledig >= 0) {
      setFelt(plasser(felt, ledig, id));
    } else {
      setMelding(t.fullt);
    }
  };
  const velgFelt = (i: number) => {
    setMelding(null);
    if (valgt === null) setValgt(i);
    else if (valgt === i) setValgt(null);
    else {
      setFelt(bytt(felt, valgt, i));
      setValgt(null);
    }
  };
  const slipp = (i: number) => (e: DragEvent) => {
    e.preventDefault();
    const fra = e.dataTransfer.getData(DRA);
    const id = e.dataTransfer.getData("text/plain");
    if (fra !== "") setFelt(bytt(felt, Number(fra), i));
    else if (id) setFelt(plasser(felt, i, id));
    setValgt(null);
  };

  return (
    <dialog
      ref={ref}
      className="fyll"
      aria-label={t.tittel(n)}
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
    >
      <div className="fyll__side">
        <div className="fyll__ark" style={{ aspectRatio: SIDE_FORHOLD.toFixed(4) }}>
          {ramme.frames.map((f, i) => {
            const id = felt[i] ?? null;
            const b = id ? bilder.get(id) : undefined;
            const forhold = (f.w / f.h) * SIDE_FORHOLD;
            return (
              <button
                key={i}
                type="button"
                className={`fyll__felt${valgt === i ? " fyll__felt--valgt" : ""}${
                  id ? "" : " fyll__felt--tomt"
                }`}
                style={{
                  left: pst(Math.max(0, f.x)),
                  top: pst(Math.max(0, f.y)),
                  width: pst(Math.min(100, f.x + f.w) - Math.max(0, f.x)),
                  height: pst(Math.min(100, f.y + f.h) - Math.max(0, f.y)),
                }}
                aria-pressed={valgt === i}
                aria-label={t.felt(i + 1, n, b ? datoTekst(b.takenAt) : null)}
                draggable={!!id}
                onDragStart={(e) => e.dataTransfer.setData(DRA, String(i))}
                onDragOver={(e) => e.preventDefault()}
                onDrop={slipp(i)}
                onClick={() => velgFelt(i)}
              >
                {id ? (
                  <img
                    src={thumbUrl(id)}
                    alt=""
                    draggable={false}
                    style={bildeStil(
                      b ?? { rotation: 0, focus: [0.5, 0.5], whole: false },
                      true,
                      forhold,
                    )}
                  />
                ) : (
                  <span aria-hidden="true">{i + 1}</span>
                )}
              </button>
            );
          })}
        </div>
        <p className="dock__help" role="status">
          {melding ?? (valgt !== null ? t.feltValgt(valgt + 1) : t.hjelp)}
        </p>
        {valgt !== null && felt[valgt] && (
          <button
            type="button"
            className="btn btn--secondary btn--sm"
            onClick={() => {
              setFelt(felt.map((x, k) => (k === valgt ? null : x)));
              setValgt(null);
            }}
          >
            {t.taUt}
          </button>
        )}
      </div>
      <div className="fyll__valg">
        <div className="fyll__hode">
          <RammeIkon felt={ramme.frames} />
          <div>
            <h2 className="fyll__tittel">{t.tittel(n)}</h2>
            <p className="count">{t.fylt(fylt.length, n)}</p>
          </div>
        </div>
        <h3 className="ev__colh">{t.bildene}</h3>
        <ul className="fyll__liste">
          {kandidater.map((k) => {
            const b = bilder.get(k.id);
            const plass = felt.indexOf(k.id);
            return (
              <li key={k.id}>
                <button
                  type="button"
                  className={`fyll__bilde${plass >= 0 ? " fyll__bilde--med" : ""}`}
                  aria-pressed={plass >= 0}
                  aria-label={`${b ? datoTekst(b.takenAt) : ""}, ${k.hvor}${
                    plass >= 0 ? `, ${t.iFelt(plass + 1)}` : ""
                  }`}
                  draggable
                  onDragStart={(e) => e.dataTransfer.setData("text/plain", k.id)}
                  onClick={() => velgBilde(k.id)}
                >
                  <img src={thumbUrl(k.id)} alt="" draggable={false} />
                  <span className="fyll__hvor">{plass >= 0 ? `✓ ${plass + 1}` : k.hvor}</span>
                </button>
              </li>
            );
          })}
        </ul>
        {ute.length > 0 && (
          <fieldset className="fyll__rest">
            <legend>{t.restTittel(ute.length)}</legend>
            <label>
              <input
                type="radio"
                name="fyll-rest"
                checked={rest === "ta_med"}
                onChange={() => setRest("ta_med")}
              />
              {t.restMed}
            </label>
            <label>
              <input
                type="radio"
                name="fyll-rest"
                checked={rest === "ta_bort"}
                onChange={() => setRest("ta_bort")}
              />
              {t.restBort}
            </label>
          </fieldset>
        )}
        <div className="dialog__actions">
          <button type="button" className="btn btn--secondary" onClick={onClose}>
            {tekster.trykk.avbryt}
          </button>
          <button
            type="button"
            className="btn btn--primary"
            disabled={fylt.length < n}
            onClick={() => onDone(fylt, ute, rest)}
          >
            {t.bruk}
          </button>
        </div>
        {fylt.length < n && <p className="field__help">{t.mangler(n - fylt.length)}</p>}
      </div>
    </dialog>
  );
}
