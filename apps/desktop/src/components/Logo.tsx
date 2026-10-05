import { tekster } from "../tekster";
import { FOTOALBUM, FOTOALBUM_SKALA } from "./logoTekst";

/**
 * Logoen: familien ser inn i fotoalbumet sammen med oss, med «Fo2Album.no» under.
 * Samme tegning som design/logo/fo2album-logo.svg; fargene kommer fra tokens (--color-logo-*).
 */
export function LogoSymbol({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 200 172" className={className} aria-hidden="true">
      <path
        className="logo__heart"
        d="M100 26c-4.5-5.5-13-3.5-13 3.2 0 5.6 7.6 9.6 13 13.8 5.4-4.2 13-8.2 13-13.8 0-6.7-8.5-8.7-13-3.2z"
      />
      <path
        className="logo__cover"
        d="M100 60C78 49 46 47 16 54l-6 78c32-6 64-3 90 9 26-12 58-15 90-9l-6-78c-30-7-62-5-84 6z"
      />
      <path className="logo__page" d="M100 63C80 53 51 51 24 57l-5 69c29-5 57-2 81 9z" />
      <path className="logo__page-alt" d="M100 63c20-10 49-12 76-6l5 69c-29-5-57-2-81 9z" />
      <path className="logo__line" d="M100 63v72" strokeWidth="1.6" />
      <g transform="rotate(-5 58 86)">
        <rect className="logo__frame" x="36" y="68" width="45" height="37" rx="1.5" />
        <rect className="logo__sky" x="40" y="72" width="37" height="25" />
        <circle className="logo__sun" cx="68" cy="79" r="4" />
        <path className="logo__hill" d="M40 97v-7c7-7 13-7 19-2 5-5 11-6 18 0v9z" />
      </g>
      <path
        className="logo__ink"
        transform={`translate(109 84) rotate(-4) scale(${FOTOALBUM_SKALA})`}
        d={FOTOALBUM}
      />
      <path
        className="logo__line"
        d="M117 95c13-2 30-2 44-1M119 104c11-1.5 23-1.5 34-1"
        strokeWidth="2"
        strokeLinecap="round"
      />
      <g className="logo__people" strokeWidth="3.5">
        <path className="logo__adult" d="M18 172c0-21 15-33 38-33s38 12 38 33z" />
        <circle className="logo__adult" cx="56" cy="122" r="18" />
        <path className="logo__adult" d="M106 172c0-22 16-35 40-35s40 13 40 35z" />
        <circle className="logo__adult" cx="146" cy="119" r="19" />
        <path className="logo__child" d="M74 172c0-15 11-23 26-23s26 8 26 23z" />
        <circle className="logo__child" cx="100" cy="136" r="13" />
      </g>
    </svg>
  );
}

export function Logo() {
  return (
    <div className="logo" role="img" aria-label={`${tekster.appNavn}.no`}>
      <LogoSymbol className="logo__symbol" />
      <span className="logo__word" aria-hidden="true">
        <span className="logo__fo2">Fo2</span>Album<span className="logo__no">.no</span>
      </span>
    </div>
  );
}
