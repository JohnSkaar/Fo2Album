import { useState } from "react";
import type { Ansikt, Ansiktsgruppe, Person, Personer, Rolle } from "../api";
import { tekster } from "../tekster";

const t = tekster.personer;
const ROLLER = Object.keys(t.roller) as Rolle[];

/**
 * Et ansikt, skåret ut av miniatyren med CSS. Utsnittet er kvadratisk og litt større enn
 * ansiktet; det regnes ut når bildets størrelse er kjent.
 */
export function Ansiktsutsnitt({
  a,
  thumbUrl,
  label,
}: {
  a: Ansikt;
  thumbUrl: (id: string) => string;
  label?: string;
}) {
  const [nat, setNat] = useState<{ w: number; h: number } | null>(null);
  let style: React.CSSProperties = { opacity: 0 };
  if (nat) {
    const cx = (a.x + a.w / 2) * nat.w;
    const cy = (a.y + a.h / 2) * nat.h;
    const side = Math.max(a.w * nat.w, a.h * nat.h) * 1.6;
    style = {
      width: `${(nat.w / side) * 100}%`,
      left: `${(-(cx - side / 2) / side) * 100}%`,
      top: `${(-(cy - side / 2) / side) * 100}%`,
    };
  }
  return (
    <span className="face" role={label ? "img" : undefined} aria-label={label}>
      <img
        src={thumbUrl(a.id)}
        alt=""
        style={style}
        onLoad={(e) =>
          setNat({ w: e.currentTarget.naturalWidth, h: e.currentTarget.naturalHeight })
        }
      />
    </span>
  );
}

function Gruppe({
  g,
  persons,
  thumbUrl,
  onName,
  onIgnore,
  onMove,
}: {
  g: Ansiktsgruppe;
  persons: Person[];
  thumbUrl: (id: string) => string;
  onName: (g: Ansiktsgruppe, name: string, role: Rolle, personId: number | null) => void;
  onIgnore: (g: Ansiktsgruppe) => void;
  onMove: (faceId: number) => void;
}) {
  const [endrer, setEndrer] = useState(g.personId === null);
  const [navn, setNavn] = useState(g.name ?? "");
  const [rolle, setRolle] = useState<Rolle>(g.role ?? "kjernefamilie");
  const [finnes, setFinnes] = useState<string>("");
  const id = `gruppe-${g.group}`;
  const andre = persons.filter((p) => p.id !== g.personId);
  return (
    <li className="pgroup" aria-labelledby={id}>
      <ul className="pgroup__faces">
        {g.samples.map((a) => (
          <li key={a.faceId} className="pgroup__face">
            <Ansiktsutsnitt a={a} thumbUrl={thumbUrl} />
            {g.name && g.samples.length > 1 && (
              <button
                type="button"
                className="pgroup__out"
                title={t.ikkeSammeInfo}
                aria-label={t.ikkeSamme(g.name)}
                onClick={() => onMove(a.faceId)}
              >
                ×
              </button>
            )}
          </li>
        ))}
      </ul>
      <div className="pgroup__body">
        <h3 id={id} className="pgroup__name">
          {g.name ?? t.ukjent}
          {g.role && <span className="pgroup__role">{t.roller[g.role]}</span>}
        </h3>
        <span className="count">{t.antall(g.faces, g.photos)}</span>
        {endrer ? (
          <form
            className="pgroup__form"
            onSubmit={(e) => {
              e.preventDefault();
              const valgt = finnes ? Number(finnes) : null;
              onName(g, navn, rolle, valgt ?? g.personId);
            }}
          >
            {g.personId === null && <p className="people__hint">{t.ukjentInfo}</p>}
            <div className="field">
              <label htmlFor={`${id}-navn`}>{t.navn}</label>
              <input
                id={`${id}-navn`}
                className="input"
                value={navn}
                onChange={(e) => setNavn(e.target.value)}
                autoComplete="off"
              />
            </div>
            <div className="field">
              <label htmlFor={`${id}-rolle`}>{t.rolle}</label>
              <select
                id={`${id}-rolle`}
                className="input"
                value={rolle}
                onChange={(e) => setRolle(e.target.value as Rolle)}
              >
                {ROLLER.map((r) => (
                  <option key={r} value={r}>
                    {t.roller[r]}
                  </option>
                ))}
              </select>
            </div>
            {g.personId === null && andre.length > 0 && (
              <div className="field">
                <label htmlFor={`${id}-finnes`}>{t.finnes}</label>
                <select
                  id={`${id}-finnes`}
                  className="input"
                  value={finnes}
                  onChange={(e) => setFinnes(e.target.value)}
                >
                  <option value="">{t.velgPerson}</option>
                  {andre.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </select>
              </div>
            )}
            <div className="pgroup__actions">
              <button type="submit" className="btn btn--primary btn--sm">
                {t.lagre}
              </button>
              {g.personId === null && (
                <button
                  type="button"
                  className="btn btn--ghost btn--sm"
                  title={t.ikkeViktigInfo}
                  onClick={() => onIgnore(g)}
                >
                  {t.ikkeViktig}
                </button>
              )}
            </div>
          </form>
        ) : (
          <div className="pgroup__actions">
            <button
              type="button"
              className="btn btn--secondary btn--sm"
              onClick={() => setEndrer(true)}
            >
              {t.endre}
            </button>
          </div>
        )}
      </div>
    </li>
  );
}

/**
 * Gruppenumrene for personer uten navn lages på nytt hver gang ansiktene grupperes, så
 * nøkkelen bygger på det beste ansiktet i gruppen (ellers følger skjemaet med til feil kort).
 */
const nokkel = (g: Ansiktsgruppe) =>
  g.personId !== null ? `p${g.personId}` : `a${g.samples[0]?.faceId ?? g.group}`;

/** «Hvem er med?»: navngi personene appen har funnet, og si hvem som er barna. */
export function PeopleScreen({
  people,
  thumbUrl,
  onName,
  onIgnore,
  onMove,
}: {
  people: Personer | null;
  thumbUrl: (id: string) => string;
  onName: (g: Ansiktsgruppe, name: string, role: Rolle, personId: number | null) => void;
  onIgnore: (g: Ansiktsgruppe) => void;
  onMove: (faceId: number) => void;
}) {
  const named = people?.groups.filter((g) => g.personId !== null) ?? [];
  const unknown = people?.groups.filter((g) => g.personId === null) ?? [];
  const props = { persons: people?.persons ?? [], thumbUrl, onName, onIgnore, onMove };
  return (
    <section className="people" aria-labelledby="personer-tittel">
      <header className="people__head">
        <h1 id="personer-tittel" className="head__title">
          {t.tittel}
        </h1>
        <p className="people__lead">{t.ingress}</p>
        <p className="people__hint">{t.trygt}</p>
      </header>
      {people && people.pending > 0 && (
        <p className="people__hint" role="status">
          {t.venter(people.pending)}
        </p>
      )}
      {people && people.groups.length === 0 && <p className="empty">{t.ingen}</p>}
      {unknown.length > 0 && (
        <ul className="pgroups">
          {unknown.map((g) => (
            <Gruppe key={nokkel(g)} g={g} {...props} />
          ))}
        </ul>
      )}
      {named.length > 0 && (
        <>
          <h2 className="people__h">{t.navngitt}</h2>
          <ul className="pgroups">
            {named.map((g) => (
              <Gruppe key={nokkel(g)} g={g} {...props} />
            ))}
          </ul>
        </>
      )}
    </section>
  );
}
