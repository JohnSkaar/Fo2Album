import { useEffect, useRef, useState } from "react";
import type { Albumtekst, Trykkfil, TrykkFremdrift } from "../api";
import { tekster } from "../tekster";

const t = tekster.trykk;

export type Trykkstatus =
  | { type: "skriver" }
  | { type: "lager"; p: TrykkFremdrift | null }
  | { type: "ferdig"; fil: Trykkfil };

/**
 * «Lag trykkfil»: tekstene i albumet (forside, første side, bakside), så «lagre som».
 * Filen lages på maskinen og sendes ingen steder.
 */
export function PrintDialog({
  open,
  year,
  text,
  status,
  checks = 0,
  onMake,
  onClose,
}: {
  open: boolean;
  year: number;
  /** Lagrede tekster eller forslaget; `null` mens de hentes. */
  text: Albumtekst | null;
  status: Trykkstatus;
  /** Antall funn i kvalitetssjekken før trykk. */
  checks?: number;
  onMake: (text: Albumtekst) => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const d = ref.current;
    if (!d) return;
    if (open && !d.open) d.showModal?.();
    if (!open && d.open) d.close?.();
  }, [open]);
  if (!open) return null;

  return (
    <dialog
      ref={ref}
      className="dialog dialog--wide"
      aria-labelledby="trykk-tittel"
      onCancel={(e) => {
        if (status.type === "lager") e.preventDefault();
        else onClose();
      }}
      open
    >
      <h2 id="trykk-tittel" className="dialog__title">
        {t.tittel}
      </h2>
      {status.type === "ferdig" ? (
        <div className="print__done" role="status">
          <p>{t.ferdig(status.fil.pages, status.fil.photos, status.fil.megabytes)}</p>
          {status.fil.missing > 0 && <p className="warn">{t.mangler(status.fil.missing)}</p>}
          {status.fil.lowResolution > 0 && (
            <p className="field__help">{t.lavOppl(status.fil.lowResolution)}</p>
          )}
          <div className="dialog__actions">
            <button type="button" className="btn btn--primary" onClick={onClose} autoFocus>
              {t.lukk}
            </button>
          </div>
        </div>
      ) : (
        text && (
          <Skjema
            key={JSON.stringify(text)}
            year={year}
            initial={text}
            busy={status.type === "lager" ? status : null}
            checks={checks}
            onMake={onMake}
            onClose={onClose}
          />
        )
      )}
    </dialog>
  );
}

function Skjema({
  year,
  initial,
  busy,
  checks,
  onMake,
  onClose,
}: {
  year: number;
  initial: Albumtekst;
  busy: { p: TrykkFremdrift | null } | null;
  checks: number;
  onMake: (text: Albumtekst) => void;
  onClose: () => void;
}) {
  const [tekst, setTekst] = useState<Albumtekst>(initial);
  const felt = (key: keyof Albumtekst, label: string, hjelp?: string, lang = false) => (
    <div className="field">
      <label htmlFor={`trykk-${key}`}>{label}</label>
      {lang ? (
        <textarea
          id={`trykk-${key}`}
          className="input input--area"
          rows={key === "intro" ? 5 : 3}
          value={tekst[key]}
          onChange={(e) => setTekst({ ...tekst, [key]: e.target.value })}
        />
      ) : (
        <input
          id={`trykk-${key}`}
          className="input"
          value={tekst[key]}
          onChange={(e) => setTekst({ ...tekst, [key]: e.target.value })}
        />
      )}
      {hjelp && <span className="field__help">{hjelp}</span>}
    </div>
  );
  return (
    <form
      className="print__form"
      onSubmit={(e) => {
        e.preventDefault();
        onMake(tekst);
      }}
    >
      <p>{t.ingress}</p>
      {checks > 0 && (
        <p className="warn" role="note">
          {tekster.sjekk.trykkOppsummering(checks)}
        </p>
      )}
      {felt("title", t.tittelFelt)}
      {felt("subtitle", t.undertittel, t.undertittelHjelp)}
      {felt("intro", t.intro, t.introHjelp(year), true)}
      {felt("back", t.bakside, t.baksideHjelp, true)}
      <p className="field__help">{t.trygt}</p>
      {busy && <p role="status">{busy.p ? t.lager(busy.p.done, busy.p.total) : "…"}</p>}
      <div className="dialog__actions">
        <button type="button" className="btn btn--secondary" onClick={onClose} disabled={!!busy}>
          {t.avbryt}
        </button>
        <button type="submit" className="btn btn--primary" disabled={!!busy}>
          {t.lag}
        </button>
      </div>
    </form>
  );
}
