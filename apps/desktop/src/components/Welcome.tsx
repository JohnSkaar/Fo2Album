import { tekster } from "../tekster";
import { Icon } from "./Icon";

export function Welcome({ onStart, busy }: { onStart: () => void; busy: boolean }) {
  return (
    <section className="start" aria-labelledby="velkommen-tittel">
      <h1 id="velkommen-tittel" className="start__title">
        {tekster.velkommen.tittel}
      </h1>
      <p className="start__lead">{tekster.velkommen.ingress}</p>
      <div className="note">
        <Icon name="shield" />
        <span>{tekster.velkommen.trygt}</span>
      </div>
      <button type="button" className="btn btn--primary btn--lg" onClick={onStart} disabled={busy}>
        {tekster.velkommen.start}
      </button>
    </section>
  );
}
