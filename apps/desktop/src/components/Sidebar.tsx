import type { Kilde } from "../api";
import { tekster } from "../tekster";
import { Icon } from "./Icon";
import { Logo } from "./Logo";

/** Hovedvisningene i menyen. */
export type Visning = "utkast" | "alle" | "personer";

export function Sidebar({
  sources,
  onAddFolder,
  onRemoveSource,
  onDeleteAll,
  locked,
  view,
  onView,
  draftPages,
}: {
  sources: Kilde[];
  onAddFolder: () => void;
  onRemoveSource: (k: Kilde) => void;
  onDeleteAll: () => void;
  /** Før lagringen er åpnet vises bare logo og låsmerknad. */
  locked: boolean;
  /** Valgt visning; `null` før det finnes bildekilder. */
  view: Visning | null;
  onView: (v: Visning) => void;
  /** Sider i utkastet, eller `null` før det er laget. */
  draftPages: number | null;
}) {
  return (
    <aside className="side">
      <Logo />
      {!locked && (
        <nav aria-label={tekster.nav.hovedmeny} className="side__nav">
          <div className="grp">
            <h2 className="grp-h">{tekster.nav.album}</h2>
            <button
              type="button"
              className="item"
              aria-current={view === "utkast" ? "page" : undefined}
              disabled={view === null}
              onClick={() => onView("utkast")}
            >
              <Icon name="album" />
              <span className="grow">
                {tekster.nav.albumutkast}
                <small>
                  {draftPages === null ? tekster.nav.ikkeLaget : tekster.nav.sider(draftPages)}
                </small>
              </span>
            </button>
            <button
              type="button"
              className="item"
              aria-current={view === "alle" ? "page" : undefined}
              disabled={view === null}
              onClick={() => onView("alle")}
            >
              <Icon name="image" />
              <span className="grow">{tekster.nav.alleBilder}</span>
            </button>
            <button
              type="button"
              className="item"
              aria-current={view === "personer" ? "page" : undefined}
              disabled={view === null}
              onClick={() => onView("personer")}
            >
              <Icon name="people" />
              <span className="grow">{tekster.nav.hvemErMed}</span>
            </button>
          </div>
          <div className="grp">
            <h2 className="grp-h">{tekster.nav.bildekilder}</h2>
            <ul className="sourcelist">
              {sources.map((s) => (
                <li key={s.id} className="item item--static">
                  <Icon name="folder" />
                  <span className="grow">
                    {s.label}
                    <small>
                      {tekster.kilder[s.kind].navn}
                      {!s.finnes && ` · ${tekster.nav.kildeMangler}`}
                    </small>
                  </span>
                  <button
                    type="button"
                    className="iconbtn"
                    aria-label={tekster.nav.fjernKilde(s.label)}
                    onClick={() => onRemoveSource(s)}
                  >
                    <Icon name="close" />
                  </button>
                </li>
              ))}
            </ul>
            <button type="button" className="item" onClick={onAddFolder}>
              <Icon name="plus" />
              <span className="grow">{tekster.nav.leggTilMappe}</span>
            </button>
          </div>
        </nav>
      )}
      <div className="side__bottom">
        <div className="local" role="note">
          <Icon name="lock" />
          <span>
            <b>{tekster.lokalt.tittel}</b> {tekster.lokalt.tekst}
          </span>
        </div>
        {!locked && (
          <button type="button" className="linkbtn" onClick={onDeleteAll}>
            {tekster.nav.slettAlleData}
          </button>
        )}
      </div>
    </aside>
  );
}
