import { useState } from "react";
import type { Forslag, KildeType } from "../api";
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
}: {
  onPickSource: (kilde: KildeType) => void;
  onFindSuggestions: () => void;
  onAddSuggestion: (f: Forslag) => void;
  /** `null` til brukeren har bedt om forslag (samtykke). */
  suggestions: Forslag[] | null;
}) {
  const [added, setAdded] = useState<Set<string>>(new Set());
  return (
    <section className="start" aria-labelledby="start-tittel">
      <h1 id="start-tittel" className="start__title">
        {tekster.start.tittel}
      </h1>
      <p className="start__lead">{tekster.start.ingress}</p>
      <Kildekort onPickSource={onPickSource} />

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
