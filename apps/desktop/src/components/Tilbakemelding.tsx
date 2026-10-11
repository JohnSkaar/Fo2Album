import { useEffect, useRef, useState } from "react";
import type { AppInfo } from "../api";
import { tekster } from "../tekster";

const t = tekster.tilbakemelding;
type Slag = keyof typeof t.slag;

/** Teksten som sendes, nøyaktig slik kunden ser den. */
export function meldingstekst(
  slag: Slag,
  tekst: string,
  info: AppInfo | null,
  medInfo: boolean,
): { emne: string; tekst: string } {
  const linjer = [tekst.trim(), "", "—", `${t.typeEtikett}: ${t.slag[slag]}`];
  if (medInfo && info) {
    linjer.push(`${t.versjonEtikett}: ${info.version}`, `${t.systemEtikett}: ${info.os}`);
  }
  return { emne: t.emne(t.slag[slag]), tekst: linjer.join("\n") };
}

/**
 * «Gi oss tilbakemelding»: kunden skriver selv og ser hva som sendes. Meldingen åpnes i
 * kundens eget e-postprogram; appen sender ingenting selv, og ingen bilder er med.
 */
export function Tilbakemelding({
  info,
  onSend,
  onClose,
}: {
  info: AppInfo | null;
  onSend: (emne: string, tekst: string) => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const [slag, setSlag] = useState<Slag>("forslag");
  const [tekst, setTekst] = useState("");
  const [medInfo, setMedInfo] = useState(true);
  const [kopiert, setKopiert] = useState(false);
  useEffect(() => {
    const d = ref.current;
    if (!d || d.open) return;
    if (d.showModal) d.showModal();
    else d.setAttribute("open", "");
  }, []);
  const melding = meldingstekst(slag, tekst, info, medInfo);
  const tom = tekst.trim() === "";
  return (
    <dialog
      ref={ref}
      className="dialog tilbake"
      aria-labelledby="tilbake-tittel"
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
    >
      <h2 id="tilbake-tittel" className="dialog__title">
        {t.tittel}
      </h2>
      <p>{t.ingress}</p>
      <fieldset className="tilbake__slag">
        <legend>{t.hvaGjelder}</legend>
        {(Object.keys(t.slag) as Slag[]).map((k) => (
          <label key={k}>
            <input
              type="radio"
              name="tilbake-slag"
              checked={slag === k}
              onChange={() => setSlag(k)}
            />
            {t.slag[k]}
          </label>
        ))}
      </fieldset>
      <div className="field">
        <label htmlFor="tilbake-tekst">{t.tekstEtikett}</label>
        <textarea
          id="tilbake-tekst"
          className="input input--area"
          rows={6}
          value={tekst}
          onChange={(e) => setTekst(e.target.value)}
        />
        <p className="field__help">{t.ikkeBilder}</p>
      </div>
      <label className="tilbake__info">
        <input type="checkbox" checked={medInfo} onChange={(e) => setMedInfo(e.target.checked)} />
        {t.medInfo}
      </label>
      <details className="tilbake__vis">
        <summary>{t.seHva(info?.feedbackEmail ?? "")}</summary>
        <pre>{`${melding.emne}\n\n${melding.tekst}`}</pre>
      </details>
      {kopiert && (
        <p className="field__help" role="status">
          {t.kopiert(info?.feedbackEmail ?? "")}
        </p>
      )}
      <div className="dialog__actions">
        <button type="button" className="btn btn--secondary" onClick={onClose}>
          {tekster.trykk.avbryt}
        </button>
        <button
          type="button"
          className="btn btn--secondary"
          disabled={tom}
          onClick={() => {
            void navigator.clipboard
              ?.writeText(`${melding.emne}\n\n${melding.tekst}`)
              .then(() => setKopiert(true));
          }}
        >
          {t.kopier}
        </button>
        <button
          type="button"
          className="btn btn--primary"
          disabled={tom}
          onClick={() => onSend(melding.emne, melding.tekst)}
        >
          {t.send}
        </button>
      </div>
    </dialog>
  );
}
