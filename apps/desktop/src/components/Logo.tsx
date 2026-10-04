import { tekster } from "../tekster";

/** Logoen. Fargene er en del av merket og følger tokens via CSS-klasser. */
export function Logo() {
  return (
    <div className="logo">
      <svg viewBox="0 0 32 32" aria-hidden="true">
        <rect
          className="logo__bg"
          x="2"
          y="2"
          width="28"
          height="28"
          rx="9"
          transform="rotate(-6 16 16)"
        />
        <g className="logo__mark">
          <path d="M10 9h9a3 3 0 0 1 3 3v11H13a3 3 0 0 1-3-3z" />
          <path d="M10 20a3 3 0 0 1 3-3h9" />
        </g>
      </svg>
      {tekster.appNavn}
    </div>
  );
}
