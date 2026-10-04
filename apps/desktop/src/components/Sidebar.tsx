import { tekster } from "../tekster";
import { Icon } from "./Icon";
import { Logo } from "./Logo";

export function Sidebar({ onAddFolder }: { onAddFolder: () => void }) {
  return (
    <aside className="side">
      <Logo />
      <nav aria-label={tekster.nav.hovedmeny} className="side__nav">
        <div className="grp">
          <h2 className="grp-h">{tekster.nav.album}</h2>
          <button type="button" className="item" aria-current="page">
            <Icon name="image" />
            <span className="grow">{tekster.nav.velgBilder}</span>
          </button>
          <button type="button" className="item" disabled>
            <Icon name="album" />
            <span className="grow">
              {tekster.nav.albumutkast}
              <small>{tekster.nav.ikkeLaget}</small>
            </span>
          </button>
        </div>
        <div className="grp">
          <h2 className="grp-h">{tekster.nav.bildekilder}</h2>
          <button type="button" className="item" onClick={onAddFolder}>
            <Icon name="plus" />
            <span className="grow">{tekster.nav.leggTilMappe}</span>
          </button>
        </div>
      </nav>
      <div className="local" role="note">
        <Icon name="lock" />
        <span>
          <b>{tekster.lokalt.tittel}</b> {tekster.lokalt.tekst}
        </span>
      </div>
    </aside>
  );
}
