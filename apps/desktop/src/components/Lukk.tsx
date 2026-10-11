import { tekster } from "../tekster";

/**
 * Knappen for å gå ut av en stor visning. Den står alltid øverst til venstre, så brukeren
 * vet hvor veien ut er.
 */
export function Lukk({
  onClick,
  label = tekster.utkast.lukk,
  fast = false,
}: {
  onClick: () => void;
  label?: string;
  /** Ligger over innholdet, festet i hjørnet av visningen. */
  fast?: boolean;
}) {
  return (
    <button
      type="button"
      className={`btn btn--secondary btn--sm lukk${fast ? " lukk--fast" : ""}`}
      onClick={onClick}
    >
      <span aria-hidden="true">✕</span> {label}
    </button>
  );
}
