import type { Aar, Bilde, Fremdrift, Innlesingsrapport, Sammendrag } from "../api";
import { datoTekst, fmt, tekster } from "../tekster";

/** Grupperer bilder per måned (bildene kommer sortert på tid). */
function perMaaned(
  bilder: Bilde[],
): { key: string; year: number; month: number; items: Bilde[] }[] {
  const groups: { key: string; year: number; month: number; items: Bilde[] }[] = [];
  for (const b of bilder) {
    const year = Number(b.takenAt?.slice(0, 4));
    const month = Number(b.takenAt?.slice(5, 7)) - 1;
    const key = `${year}-${month}`;
    const last = groups[groups.length - 1];
    if (last && last.key === key) last.items.push(b);
    else groups.push({ key, year, month, items: [b] });
  }
  return groups;
}

function Merknader({ s, report }: { s: Sammendrag; report: Innlesingsrapport | null }) {
  const m = tekster.merknader;
  const notes = [
    s.exact_duplicates + s.near_duplicates > 0 &&
      m.dubletter(s.exact_duplicates + s.near_duplicates),
    s.cloud_only > 0 && m.bareISkyen(s.cloud_only),
    s.without_preview > 0 && m.utenMiniatyr(s.without_preview),
    s.uncertain_dates > 0 && m.usikreDatoer(s.uncertain_dates),
    s.unreadable > 0 && m.uleselige(s.unreadable),
    report && report.missing_sources.length > 0 && m.manglerKilder(report.missing_sources),
  ].filter((n): n is string => typeof n === "string");
  if (notes.length === 0) return null;
  return (
    <ul className="warn" aria-label="Merknader">
      {notes.map((n) => (
        <li key={n}>{n}</li>
      ))}
    </ul>
  );
}

function Progress({ p, onCancel }: { p: Fremdrift; onCancel: () => void }) {
  const text = tekster.innlesing.tekst(p);
  const pct = p.total > 0 ? Math.round((p.done / p.total) * 100) : 0;
  return (
    <div className="loadbox">
      <div className="loadbox__row">
        <span role="status">{text}</span>
        <button type="button" className="btn btn--ghost btn--sm" onClick={onCancel}>
          {tekster.innlesing.avbryt}
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
    </div>
  );
}

export function PickScreen({
  years,
  year,
  onYear,
  photos,
  summary,
  progress,
  report,
  onCancel,
  thumbUrl,
}: {
  years: Aar[];
  year: number | null;
  onYear: (y: number) => void;
  photos: Bilde[];
  summary: Sammendrag | null;
  progress: Fremdrift | null;
  report: Innlesingsrapport | null;
  onCancel: () => void;
  thumbUrl: (id: string) => string;
}) {
  return (
    <section className="pick" aria-labelledby="velg-tittel">
      <div className="head">
        <div>
          <h1 id="velg-tittel" className="head__title">
            {year ? tekster.bilder.tittel(year) : tekster.bilder.ingenAar}
          </h1>
        </div>
        <span className="count">{tekster.bilder.antall(photos.length)}</span>
      </div>

      {progress && <Progress p={progress} onCancel={onCancel} />}

      {years.length > 0 && (
        <div className="chips" role="group" aria-label={tekster.bilder.aarEtikett}>
          {years.map((y) => (
            <button
              key={y.year}
              type="button"
              className="chip"
              aria-pressed={y.year === year}
              onClick={() => onYear(y.year)}
            >
              {y.year} <span className="chip__count">{fmt(y.count)}</span>
            </button>
          ))}
        </div>
      )}

      {summary && <Merknader s={summary} report={report} />}

      {photos.length === 0 ? (
        <p className="empty">{progress ? tekster.bilder.leserInn : tekster.bilder.tom}</p>
      ) : (
        perMaaned(photos).map((g) => (
          <section key={g.key} aria-label={tekster.bilder.maaned(g.month, g.year)}>
            <h2 className="month-h">
              {tekster.bilder.maaned(g.month, g.year)}{" "}
              <span className="count">· {fmt(g.items.length)}</span>
            </h2>
            <ul className="grid">
              {g.items.map((b) => (
                <li key={b.id} className="th" title={b.takenAt ? datoTekst(b.takenAt) : undefined}>
                  {b.hasThumbnail ? (
                    <img
                      src={thumbUrl(b.id)}
                      alt={b.takenAt ? tekster.bilder.bildeEtikett(datoTekst(b.takenAt)) : ""}
                      loading="lazy"
                      decoding="async"
                    />
                  ) : (
                    <span className="th__none">{tekster.bilder.ingenMiniatyr}</span>
                  )}
                  {b.dateSource === "endringstid" && (
                    <span className="th__badge">{tekster.bilder.usikkerDato}</span>
                  )}
                </li>
              ))}
            </ul>
          </section>
        ))
      )}
    </section>
  );
}
