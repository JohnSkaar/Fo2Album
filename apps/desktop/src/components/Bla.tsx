import { useEffect, useRef, useState } from "react";
import type { Albumtekst, Side, Utkast, UtkastBilde } from "../api";
import { tekster } from "../tekster";
import { bildeStil, Sidebilde } from "./Side";

const t = tekster.redigering;
const tt = tekster.trykk;

type Tekstside = "forside" | "intro" | "bakside";

/** Tekstfeltene for én side i albumet. */
function Skriv({
  side,
  aar,
  start,
  onSave,
  onClose,
}: {
  side: Tekstside;
  aar: number;
  start: Albumtekst;
  onSave: (x: Albumtekst) => void;
  onClose: () => void;
}) {
  const [x, setX] = useState(start);
  const felt = (key: keyof Albumtekst, label: string, lang = false) => (
    <div className="field">
      <label htmlFor={`bla-${key}`}>{label}</label>
      {lang ? (
        <textarea
          id={`bla-${key}`}
          className="input input--area"
          rows={key === "intro" ? 6 : 3}
          value={x[key]}
          onChange={(e) => setX({ ...x, [key]: e.target.value })}
        />
      ) : (
        <input
          id={`bla-${key}`}
          className="input"
          value={x[key]}
          onChange={(e) => setX({ ...x, [key]: e.target.value })}
        />
      )}
    </div>
  );
  return (
    <form
      className="bla__skriv"
      onSubmit={(e) => {
        e.preventDefault();
        onSave(x);
        onClose();
      }}
    >
      {side === "forside" && (
        <>
          {felt("title", tt.tittelFelt)}
          {felt("subtitle", tt.undertittel)}
        </>
      )}
      {side === "intro" && (
        <>
          {felt("intro", tt.intro, true)}
          <p className="field__help">{tt.introHjelp(aar)}</p>
        </>
      )}
      {side === "bakside" && felt("back", tt.bakside, true)}
      <p className="field__help">{t.tekstHjelp}</p>
      <div className="dialog__actions">
        <button type="button" className="btn btn--secondary btn--sm" onClick={onClose}>
          {tt.avbryt}
        </button>
        <button type="submit" className="btn btn--primary btn--sm">
          {t.lagreTekst}
        </button>
      </div>
    </form>
  );
}

type Blad =
  | { type: "forside"; id: string | null }
  | { type: "bakside"; id: string | null }
  | { type: "intro" }
  | { type: "side"; side: Side; nr: number };

/**
 * Oppslagene i albumet, som i trykkfilen: forsiden alene, første side inne (teksten om året)
 * til høyre, så sidene to og to (venstre og høyre), og baksiden alene til slutt. `null` er en
 * tom halvdel.
 */
export function oppslag(u: Utkast): (Blad | null)[][] {
  const sider: Blad[] = u.events
    .flatMap((e) => e.layout)
    .filter((s) => s.photos.length > 0)
    .map((side, i) => ({ type: "side", side, nr: i + 1 }));
  const ut: (Blad | null)[][] = [
    [null, { type: "forside", id: u.cover.front }],
    [null, { type: "intro" }],
  ];
  for (let i = 0; i < sider.length; i += 2) ut.push([sider[i]!, sider[i + 1] ?? null]);
  ut.push([{ type: "bakside", id: u.cover.back }, null]);
  return ut;
}

/** Hele albumet på skjermen, oppslag for oppslag. */
export function Bla({
  draft,
  tekst,
  thumbUrl,
  onClose,
  onSaveText,
}: {
  draft: Utkast;
  /** Tekstene i albumet (forsiden, første side, baksiden), når de er hentet. */
  tekst: Albumtekst | null;
  thumbUrl: (id: string) => string;
  onClose: () => void;
  /** Lagrer tekstene brukeren skriver rett i albumet. */
  onSaveText?: (x: Albumtekst) => void;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const [n, setN] = useState(0);
  const [skriv, setSkriv] = useState<Tekstside | null>(null);
  const alle = oppslag(draft);
  const bilder = new Map<string, UtkastBilde>(draft.photos.map((b) => [b.id, b]));
  useEffect(() => {
    const d = ref.current;
    if (!d || d.open) return;
    if (d.showModal) d.showModal();
    else d.setAttribute("open", "");
  }, []);

  const blad = (b: Blad | null, k: number) => {
    if (!b) {
      // Tekstfeltene står i den tomme halvdelen, ved siden av siden de hører til.
      if (skriv && tekst && onSaveText) {
        return (
          <Skriv
            key={skriv}
            side={skriv}
            aar={draft.year}
            start={tekst}
            onSave={onSaveText}
            onClose={() => setSkriv(null)}
          />
        );
      }
      return <div key={k} className="bla__blank" />;
    }
    if (b.type === "side") {
      return (
        <Sidebilde
          key={k}
          side={b.side}
          bilder={bilder}
          thumbUrl={thumbUrl}
          etikett={tekster.sider.side(b.nr)}
        />
      );
    }
    if (b.type === "intro") {
      return (
        <div key={k} className="bla__intro" role="img" aria-label={t.introSide}>
          <div className="bla__introtekst">
            <span className="bla__aar">{draft.year}</span>
            {tekst?.intro && <p>{tekst.intro}</p>}
          </div>
        </div>
      );
    }
    const navn = b.type === "forside" ? t.forsideSide : t.baksideSide;
    const bilde = b.id ? bilder.get(b.id) : undefined;
    return (
      <div key={k} className={`bla__omslag bla__omslag--${b.type}`} role="img" aria-label={navn}>
        {b.id && (
          <span className="bla__omslagbilde">
            <img
              src={thumbUrl(b.id)}
              alt=""
              style={bildeStil(
                bilde ?? { rotation: 0, focus: [0.5, 0.5], whole: false },
                true,
                b.type === "forside" ? 1.07 : 1.25,
              )}
            />
          </span>
        )}
        {b.type === "forside" ? (
          <span className="bla__tittel">
            {tekst?.title ?? tekster.utkast.tittel(draft.year)}
            {tekst?.subtitle && <small>{tekst.subtitle}</small>}
          </span>
        ) : (
          tekst?.back && <span className="bla__hilsen">{tekst.back}</span>
        )}
      </div>
    );
  };

  const gaa = (d: number) => {
    setSkriv(null);
    setN((x) => Math.min(alle.length - 1, Math.max(0, x + d)));
  };
  // Siden i dette oppslaget som har tekst (forsiden, første side eller baksiden).
  const tekstside = (alle[n] ?? [])
    .map((b) => b?.type)
    .find((x): x is Tekstside => x === "forside" || x === "intro" || x === "bakside");
  return (
    <dialog
      ref={ref}
      className="bla"
      aria-label={t.blaTittel(draft.year)}
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
      onKeyDown={(e) => {
        if ((e.target as HTMLElement).closest("input, textarea")) return;
        if (e.key === "ArrowRight") gaa(1);
        if (e.key === "ArrowLeft") gaa(-1);
      }}
    >
      <div className="bla__top">
        <span role="status">{t.oppslag(n + 1, alle.length)}</span>
        <span className="bla__hjelp">{t.blaHjelp}</span>
        <button type="button" className="btn btn--secondary btn--sm" onClick={onClose}>
          {tekster.utkast.lukk}
        </button>
      </div>
      <div className="bla__opp">{(alle[n] ?? []).map(blad)}</div>
      {!skriv && tekstside && onSaveText && (
        <div className="bla__tekstknapp">
          <button
            type="button"
            className="btn btn--secondary btn--sm"
            disabled={!tekst}
            onClick={() => setSkriv(tekstside)}
          >
            ✎ {t.endreTekst}
          </button>
        </div>
      )}
      <div className="bla__nav">
        <button
          type="button"
          className="btn btn--secondary"
          disabled={n === 0}
          onClick={() => gaa(-1)}
        >
          ← {t.forrige}
        </button>
        <button
          type="button"
          className="btn btn--secondary"
          disabled={n >= alle.length - 1}
          onClick={() => gaa(1)}
          autoFocus
        >
          {t.neste} →
        </button>
      </div>
    </dialog>
  );
}
