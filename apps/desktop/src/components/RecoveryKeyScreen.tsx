import { useState } from "react";
import { tekster } from "../tekster";
import { Icon } from "./Icon";

/** Viser gjenopprettingsnøkkelen én gang, rett etter første oppstart. */
export function RecoveryKeyScreen({ code, onDone }: { code: string; onDone: () => void }) {
  const [confirmed, setConfirmed] = useState(false);
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
    } catch {
      // Utklippstavlen er ikke tilgjengelig; brukeren kan skrive av nøkkelen.
    }
  };
  return (
    <section className="start" aria-labelledby="nokkel-tittel">
      <h1 id="nokkel-tittel" className="start__title">
        {tekster.gjenopprettingsnokkel.tittel}
      </h1>
      <p className="start__lead">{tekster.gjenopprettingsnokkel.ingress}</p>
      <div className="keybox">
        <Icon name="key" />
        <code aria-label={tekster.gjenopprettingsnokkel.etikett} className="keybox__code">
          {code}
        </code>
        <button type="button" className="btn btn--ghost" onClick={copy}>
          {copied ? tekster.gjenopprettingsnokkel.kopiert : tekster.gjenopprettingsnokkel.kopier}
        </button>
      </div>
      <p className="start__help">{tekster.gjenopprettingsnokkel.tips}</p>
      <label className="check">
        <input
          type="checkbox"
          checked={confirmed}
          onChange={(e) => setConfirmed(e.target.checked)}
        />
        {tekster.gjenopprettingsnokkel.bekreft}
      </label>
      <button
        type="button"
        className="btn btn--primary btn--lg"
        disabled={!confirmed}
        onClick={onDone}
      >
        {tekster.gjenopprettingsnokkel.fortsett}
      </button>
    </section>
  );
}
