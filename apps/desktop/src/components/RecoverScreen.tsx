import { useState, type FormEvent } from "react";
import { tekster } from "../tekster";

/** Ny maskin: brukeren skriver inn gjenopprettingsnøkkelen. */
export function RecoverScreen({
  onSubmit,
  error,
  busy,
}: {
  onSubmit: (code: string) => void;
  error: string | null;
  busy: boolean;
}) {
  const [code, setCode] = useState("");
  const submit = (e: FormEvent) => {
    e.preventDefault();
    if (code.trim()) onSubmit(code);
  };
  return (
    <section className="start" aria-labelledby="gjenopprett-tittel">
      <h1 id="gjenopprett-tittel" className="start__title">
        {tekster.gjenopprett.tittel}
      </h1>
      <p className="start__lead">{tekster.gjenopprett.ingress}</p>
      <form className="field field--wide" onSubmit={submit}>
        <label htmlFor="gjenopprett-kode">{tekster.gjenopprett.etikett}</label>
        <input
          id="gjenopprett-kode"
          className="input input--code"
          value={code}
          onChange={(e) => setCode(e.target.value)}
          placeholder={tekster.gjenopprett.plassholder}
          autoComplete="off"
          autoCapitalize="characters"
          spellCheck={false}
          aria-invalid={error ? true : undefined}
          aria-describedby="gjenopprett-hjelp"
        />
        <p
          id="gjenopprett-hjelp"
          className={error ? "field__error" : "field__help"}
          role={error ? "alert" : undefined}
        >
          {error ?? tekster.gjenopprett.hjelp}
        </p>
        <button type="submit" className="btn btn--primary btn--lg" disabled={busy || !code.trim()}>
          {tekster.gjenopprett.knapp}
        </button>
      </form>
    </section>
  );
}
