import { tekster, type KildeId } from "../tekster";
import { Icon, type IconName } from "./Icon";

const kildeIkon: Record<KildeId, IconName> = {
  pc: "computer",
  dropbox: "dropbox",
  icloud: "cloud",
  googleDisk: "drive",
};

export function StartScreen({ onPickSource }: { onPickSource: (kilde: KildeId) => void }) {
  const kilder = Object.keys(tekster.kilder) as KildeId[];
  return (
    <section className="start" aria-labelledby="start-tittel">
      <h1 id="start-tittel" className="start__title">
        {tekster.start.tittel}
      </h1>
      <p className="start__lead">{tekster.start.ingress}</p>
      <ul className="sources" aria-label={tekster.start.kilderEtikett}>
        {kilder.map((id) => (
          <li key={id}>
            <button type="button" className="src" onClick={() => onPickSource(id)}>
              <span className="src__icon">
                <Icon name={kildeIkon[id]} />
              </span>
              <span>
                <b>{tekster.kilder[id].navn}</b>
                <span>{tekster.kilder[id].hint}</span>
              </span>
            </button>
          </li>
        ))}
      </ul>
      <div className="note">
        <Icon name="shield" />
        <span>{tekster.start.trygt}</span>
      </div>
      <p className="start__help">{tekster.start.hjelp}</p>
    </section>
  );
}
