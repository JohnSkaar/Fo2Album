import { useState } from "react";
import type { Aar, Forslag, Fremdrift, Kilde, KildeType } from "../api";
import { tekster } from "../tekster";
import { Icon, type IconName } from "./Icon";

const kort: { id: KildeType; ikon: IconName }[] = [
  { id: "pc", ikon: "computer" },
  { id: "dropbox", ikon: "dropbox" },
  { id: "icloud", ikon: "cloud" },
  { id: "google_disk", ikon: "drive" },
];

/** Kortene for å velge en bildekilde. Brukes også på utkastsiden før utkastet er laget. */
export function Kildekort({ onPickSource }: { onPickSource: (kilde: KildeType) => void }) {
  return (
    <ul className="sources" aria-label={tekster.start.kilderEtikett}>
      {kort.map(({ id, ikon }) => (
        <li key={id}>
          <button type="button" className="src" onClick={() => onPickSource(id)}>
            <span className="src__icon">
              <Icon name={ikon} />
            </span>
            <span>
              <b>{tekster.kilder[id].navn}</b>
              <span>{tekster.kilder[id].hint}</span>
            </span>
          </button>
        </li>
      ))}
    </ul>
  );
}

export function StartScreen({
  onPickSource,
  onFindSuggestions,
  onAddSuggestion,
  suggestions,
  sources = [],
  years = [],
  year = null,
  onYear,
  ingest = null,
  onMake,
}: {
  onPickSource: (kilde: KildeType) => void;
  onFindSuggestions: () => void;
  onAddSuggestion: (f: Forslag) => void;
  /** `null` til brukeren har bedt om forslag (samtykke). */
  suggestions: Forslag[] | null;
  /** Kildene som er lagt til. Når det finnes noen, vises «Lag utkast» under kortene. */
  sources?: Kilde[];
  years?: Aar[];
  year?: number | null;
  onYear?: (y: number) => void;
  ingest?: Fremdrift | null;
  onMake?: () => void;
}) {
  const [added, setAdded] = useState<Set<string>>(new Set());
  return (
    <section className="start" aria-labelledby="start-tittel">
      <h1 id="start-tittel" className="start__title">
        {tekster.start.tittel}
      </h1>
      <p className="start__lead">{tekster.start.ingress}</p>
      <Kildekort onPickSource={onPickSource} />

      {sources.length > 0 && (
        <div className="added" aria-live="polite">
          <b>{tekster.utkast.kilderLagtTil}</b>
          <ul className="intro__sources" aria-label={tekster.utkast.kilderLagtTil}>
            {sources.map((k) => (
              <li key={k.id}>
                {k.label} <span className="count">· {tekster.kilder[k.kind].navn}</span>
              </li>
            ))}
          </ul>
          {ingest && <p className="start__help">{tekster.utkast.venterInnlesing}</p>}
          <p className="start__help">
            <b>{tekster.utkast.flereKilder}</b> {tekster.utkast.flereKilderTekst}
          </p>
          <div className="added__row">
            {years.length > 1 && onYear && (
              <label className="added__year">
                {tekster.utkast.aar}
                <select
                  className="input"
                  value={year ?? ""}
                  onChange={(e) => onYear(Number(e.target.value))}
                >
                  {years.map((y) => (
                    <option key={y.year} value={y.year}>
                      {tekster.utkast.aarValg(y.year, y.count)}
                    </option>
                  ))}
                </select>
              </label>
            )}
            <button
              type="button"
              className="btn btn--primary btn--lg"
              onClick={onMake}
              disabled={year === null}
            >
              {tekster.utkast.lag}
            </button>
          </div>
        </div>
      )}

      {suggestions === null ? (
        <div className="suggest">
          <button type="button" className="btn btn--secondary" onClick={onFindSuggestions}>
            {tekster.start.forslagKnapp}
          </button>
          <p className="start__help">{tekster.start.forslagHjelp}</p>
        </div>
      ) : (
        <div className="suggest" aria-live="polite">
          <h2 className="suggest__title">{tekster.start.forslagTittel}</h2>
          {suggestions.length === 0 ? (
            <p className="start__help">{tekster.start.forslagIngen}</p>
          ) : (
            <ul className="suggest__list">
              {suggestions.map((f) => (
                <li key={f.path}>
                  <Icon name="folder" />
                  <span className="grow">
                    {f.label}
                    <small>
                      {tekster.kilder[f.kind].navn} · {f.path}
                    </small>
                  </span>
                  <button
                    type="button"
                    className="btn btn--ghost"
                    disabled={added.has(f.path)}
                    onClick={() => {
                      setAdded(new Set(added).add(f.path));
                      onAddSuggestion(f);
                    }}
                  >
                    {added.has(f.path) ? tekster.start.lagtTil : tekster.start.leggTil}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}

      <div className="note">
        <Icon name="shield" />
        <span>{tekster.start.trygt}</span>
      </div>
      <p className="start__help">{tekster.start.hjelp}</p>
    </section>
  );
}
