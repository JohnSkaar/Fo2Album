import { tekster } from "../tekster";
import { Icon } from "./Icon";

const t = tekster.sky;

/** Fremdriften mens bilder hentes fra skyen. */
export interface Skyfremdrift {
  done: number;
  total: number;
  failed: number;
}

/**
 * «n bilder fra året ligger bare i skyen»: knappen som ber Dropbox, iCloud eller OneDrive
 * laste dem ned til maskinen, og fremdriften mens det skjer.
 */
export function Skyhenting({
  year,
  count,
  progress,
  onFetch,
  onCancel,
}: {
  year: number;
  count: number;
  progress: Skyfremdrift | null;
  onFetch: () => void;
  onCancel: () => void;
}) {
  if (count === 0 && !progress) return null;
  const pct =
    progress && progress.total > 0 ? Math.round((progress.done / progress.total) * 100) : 0;
  return (
    <section className="sky" aria-label={t.tittel(count, year)}>
      <Icon name="cloud" />
      <div className="sky__tekst">
        <b>{t.tittel(count, year)}</b>
        <p>{t.forklaring}</p>
        {progress ? (
          <>
            <div className="loadbox__row">
              <span role="status">{t.henter(progress.done, progress.total)}</span>
              <button type="button" className="btn btn--ghost btn--sm" onClick={onCancel}>
                {t.stopp}
              </button>
            </div>
            <div
              className="progress"
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={pct}
            >
              <i style={{ width: `${pct}%` }} />
            </div>
          </>
        ) : (
          <button type="button" className="btn btn--secondary" onClick={onFetch}>
            {t.hent(count)}
          </button>
        )}
      </div>
    </section>
  );
}
