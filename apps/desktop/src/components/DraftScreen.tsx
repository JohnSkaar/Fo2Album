import { useMemo, useState } from "react";
import type {
  Analysefase,
  Handling,
  Laert,
  Side,
  Svar,
  Utkast,
  UtkastBilde,
  UtkastHendelse,
} from "../api";
import { pris, prisvalg } from "../pris";
import { datoTekst, fmt, tekster } from "../tekster";
import { datoSpenn, HVORFOR_VALG, oppdaterSider, spoerOmHvorfor } from "../utkast";

const t = tekster.utkast;
/** Så mange bilder som ikke er med, vises før «Vis alle». */
const VIS_IKKE_MED = 8;
const FASER: Analysefase[] = ["henter", "hendelser", "serier", "velger", "begrunnelser"];

type Valgt = { id: string; included: boolean; event: number };
type Spørsmål = { feedbackId: number; action: Handling };

function Analyse({ fase }: { fase: Analysefase }) {
  const naa = fase === "venter" ? -1 : FASER.indexOf(fase);
  const pct = fase === "ferdig" ? 100 : Math.round((Math.max(0, naa) / FASER.length) * 100);
  return (
    <div className="analyse">
      <h2 className="analyse__title" role="status">
        {t.analyseTittel}
      </h2>
      <div
        className="progress"
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={pct}
      >
        <i style={{ width: `${pct}%` }} />
      </div>
      <ol className="analyse__steps">
        {fase === "venter" && <li aria-current="step">{t.faser.venter}</li>}
        {FASER.map((f, i) => (
          <li
            key={f}
            data-ferdig={fase === "ferdig" || i < naa}
            aria-current={i === naa ? "step" : undefined}
          >
            {t.faser[f]}
          </li>
        ))}
      </ol>
    </div>
  );
}

function Kort({
  b,
  hendelse,
  valgt,
  liten,
  onClick,
  thumbUrl,
}: {
  b: UtkastBilde;
  hendelse: UtkastHendelse;
  valgt: boolean;
  liten?: boolean;
  onClick: () => void;
  thumbUrl: (id: string) => string;
}) {
  const hvorfor = tekster.begrunnelse(b.reason, hendelse.photos);
  return (
    <li>
      <button
        type="button"
        className={`dcard${liten ? " dcard--small" : ""}`}
        aria-pressed={valgt}
        aria-label={`${t.bildeEtikett(datoTekst(b.takenAt), b.included)}. ${hvorfor}`}
        onClick={onClick}
      >
        <span className="th">
          {b.hasThumbnail ? (
            <img src={thumbUrl(b.id)} alt="" loading="lazy" decoding="async" />
          ) : (
            <span className="th__none">{tekster.bilder.ingenMiniatyr}</span>
          )}
          {b.blurry && (
            <span className="th__badge" title={t.uskarptHjelp}>
              {t.uskarpt}
            </span>
          )}
        </span>
        <span className="dcard__why">{hvorfor}</span>
      </button>
    </li>
  );
}

function Sider({
  sider,
  med,
  thumbUrl,
}: {
  sider: Side[];
  /** Bildene som er med nå, i tidsrekkefølge. */
  med: string[];
  thumbUrl: (id: string) => string;
}) {
  const { sider: vis, endret } = oppdaterSider(sider, med);
  return (
    <div className="pages">
      <h4 className="ev__colh">{tekster.sider.tittel}</h4>
      <ol className="pages__list">
        {vis.map((s, i) => (
          <li
            key={s.photos[0]}
            className={`pg pg--${s.kind}`}
            style={s.kolonner ? { gridTemplateColumns: `repeat(${s.kolonner}, 1fr)` } : undefined}
            aria-label={`${tekster.sider.side(i + 1)}: ${
              s.kind === "rutenett"
                ? tekster.sider.rutenett(s.photos.length)
                : tekster.sider[s.kind]
            }`}
          >
            {s.photos.map((id) => (
              <img key={id} src={thumbUrl(id)} alt="" loading="lazy" decoding="async" />
            ))}
          </li>
        ))}
      </ol>
      {endret && <p className="ev__none">{tekster.sider.endret}</p>}
    </div>
  );
}

function Hendelse({
  e,
  index,
  bilder,
  valgt,
  onPick,
  thumbUrl,
}: {
  e: UtkastHendelse;
  index: number;
  bilder: UtkastBilde[];
  valgt: Valgt | null;
  onPick: (b: UtkastBilde) => void;
  thumbUrl: (id: string) => string;
}) {
  const [visAlle, setVisAlle] = useState(false);
  const med = bilder.filter((b) => b.included);
  const ikkeMed = bilder.filter((b) => !b.included);
  const synlige = visAlle ? ikkeMed : ikkeMed.slice(0, VIS_IKKE_MED);
  const tittel = e.everyday
    ? t.hverdager(Number(e.start.slice(5, 7)) - 1)
    : datoSpenn(e.start, e.end);
  const id = `hendelse-${index}`;
  return (
    <section className="ev" aria-labelledby={id}>
      <header className="ev__head">
        <h3 id={id} className="ev__title">
          {tittel}
        </h3>
        <span className="count">{t.hendelseInfo(e.photos, e.pages)}</span>
      </header>
      <Sider sider={e.layout} med={med.map((b) => b.id)} thumbUrl={thumbUrl} />
      <div className="ev__cols">
        <div className="ev__col">
          <h4 className="ev__colh">
            {t.med} <span className="count">· {fmt(med.length)}</span>
          </h4>
          <ul className="grid grid--draft">
            {med.map((b) => (
              <Kort
                key={b.id}
                b={b}
                hendelse={e}
                valgt={valgt?.id === b.id}
                onClick={() => onPick(b)}
                thumbUrl={thumbUrl}
              />
            ))}
          </ul>
        </div>
        <div className="ev__col ev__col--out">
          <h4 className="ev__colh">
            {t.ikkeMed} <span className="count">· {fmt(ikkeMed.length)}</span>
          </h4>
          {ikkeMed.length === 0 ? (
            <p className="ev__none">{t.ingenIkkeMed}</p>
          ) : (
            <ul className="grid grid--draft grid--small">
              {synlige.map((b) => (
                <Kort
                  key={b.id}
                  b={b}
                  hendelse={e}
                  liten
                  valgt={valgt?.id === b.id}
                  onClick={() => onPick(b)}
                  thumbUrl={thumbUrl}
                />
              ))}
            </ul>
          )}
          {ikkeMed.length > VIS_IKKE_MED && (
            <button
              type="button"
              className="btn btn--ghost btn--sm"
              onClick={() => setVisAlle(!visAlle)}
            >
              {visAlle ? t.visFaerre : t.visAlle(ikkeMed.length)}
            </button>
          )}
        </div>
      </div>
    </section>
  );
}

function Pris({ draft, onSize }: { draft: Utkast; onSize: (sider: number | null) => void }) {
  const { ned, opp } = prisvalg(draft.pages);
  const [eget, setEget] = useState(String(draft.pages));
  return (
    <div className="price">
      <p className="price__now">{tekster.pris.naa(draft.pages, pris(draft.pages))}</p>
      <p className="price__help">{tekster.pris.juster}</p>
      <div className="chips price__sizes" role="group" aria-label={tekster.pris.etikett}>
        {ned.map((v) => (
          <button key={v.sider} type="button" className="chip" onClick={() => onSize(v.sider)}>
            {tekster.pris.ned(v.sider, v.pris)}
          </button>
        ))}
        <label className="price__own">
          {tekster.pris.eget}
          <input
            type="number"
            min={1}
            className="input"
            value={eget}
            onChange={(e) => setEget(e.target.value)}
          />
        </label>
        <button
          type="button"
          className="btn btn--secondary btn--sm"
          onClick={() => Number(eget) >= 1 && onSize(Math.round(Number(eget)))}
        >
          {tekster.pris.lagPaaNytt}
        </button>
        {opp.map((v) => (
          <button key={v.sider} type="button" className="chip" onClick={() => onSize(v.sider)}>
            {tekster.pris.opp(v.sider, v.pris)}
          </button>
        ))}
        {draft.pageCap !== null && (
          <button type="button" className="chip" onClick={() => onSize(null)}>
            {tekster.pris.forslag(draft.fullPages, pris(draft.fullPages))}
          </button>
        )}
      </div>
      <p className="price__help">{tekster.pris.modell}</p>
    </div>
  );
}

function Laerdommer({ l }: { l: Laert }) {
  return (
    <details className="learned">
      <summary>{tekster.laert.tittel}</summary>
      {l.lessons.length === 0 ? (
        <p>{tekster.laert.ingenting}</p>
      ) : (
        <ul>
          {l.lessons.map((x) => (
            <li key={x}>{tekster.laert.laerdom[x]}</li>
          ))}
        </ul>
      )}
      {l.choices > 0 && <p className="count">{tekster.laert.grunnlag(l.choices, l.answers)}</p>}
    </details>
  );
}

export function DraftScreen({
  year,
  draft,
  phase,
  onMake,
  onPrint,
  onSize,
  onChoose,
  onAnswer,
  thumbUrl,
}: {
  /** Året albumet lages for (til tittelen mens gjennomgangen pågår). */
  year: number | null;
  draft: Utkast | null;
  phase: Analysefase | null;
  onMake: () => void;
  /** «Lag trykkfil»: albumet som PDF. */
  onPrint: () => void;
  /** Lager utkastet på nytt med et sidetak (`null` = hele historien). */
  onSize: (sidetak: number | null) => void;
  /** Lagrer valget og returnerer id-en til loggføringen (til «Hvorfor?»), eller `null`. */
  onChoose: (action: Handling, id: string, other?: string) => Promise<number | null>;
  onAnswer: (feedbackId: number, reason: Svar | null) => void;
  thumbUrl: (id: string) => string;
}) {
  const [valgt, setValgt] = useState<Valgt | null>(null);
  const [spm, setSpm] = useState<Spørsmål | null>(null);
  const [handlinger, setHandlinger] = useState(0);
  const [hoppetOver, setHoppetOver] = useState(0);
  const [takk, setTakk] = useState(false);

  const perHendelse = useMemo(() => {
    const m = new Map<number, UtkastBilde[]>();
    for (const b of draft?.photos ?? []) {
      const l = m.get(b.event);
      if (l) l.push(b);
      else m.set(b.event, [b]);
    }
    return m;
  }, [draft]);

  if (phase) {
    return (
      <section className="pick" aria-labelledby="utkast-tittel">
        <h1 id="utkast-tittel" className="head__title">
          {year ? t.tittel(year) : t.lag}
        </h1>
        <Analyse fase={phase} />
      </section>
    );
  }

  // Albumutkastet vises bare når gjennomgangen er i gang eller ferdig (startsiden lager det).
  if (!draft) return null;

  const handling = async (action: Handling, id: string, other?: string) => {
    setValgt(null);
    setTakk(false);
    const feedbackId = await onChoose(action, id, other);
    const n = handlinger + 1;
    setHandlinger(n);
    if (feedbackId !== null && spoerOmHvorfor(n, hoppetOver)) setSpm({ feedbackId, action });
    else setSpm(null);
  };

  const svar = (reason: Svar | null) => {
    if (!spm) return;
    onAnswer(spm.feedbackId, reason);
    setHoppetOver(reason === null ? hoppetOver + 1 : 0);
    setTakk(reason !== null);
    setSpm(null);
  };

  const velg = (b: UtkastBilde) => {
    // Ett bilde fra hver kolonne i samme hendelse: bytt dem.
    if (valgt && valgt.event === b.event && valgt.included !== b.included) {
      const inn = b.included ? valgt.id : b.id;
      const ut = b.included ? b.id : valgt.id;
      void handling("bytt", inn, ut);
      return;
    }
    setValgt(valgt?.id === b.id ? null : { id: b.id, included: b.included, event: b.event });
  };

  const valgtBilde = valgt ? draft.photos.find((b) => b.id === valgt.id) : undefined;
  const lignende =
    valgtBilde && !valgtBilde.included && valgtBilde.related
      ? draft.photos.find((b) => b.id === valgtBilde.related && b.included)
      : undefined;
  const antallMed = draft.photos.filter((b) => b.included).length;
  const hendelserMed = draft.events.filter((e) => e.included > 0).length;

  return (
    <section className="pick draft" aria-labelledby="utkast-tittel">
      <div className="head">
        <div>
          <h1 id="utkast-tittel" className="head__title">
            {t.utkastTittel(draft.year)}
          </h1>
          <p className="draft__sum">{t.sammendrag(antallMed, draft.pages, hendelserMed)}</p>
        </div>
        <div className="draft__actions">
          <button type="button" className="btn btn--secondary" onClick={onMake}>
            {t.nyttUtkast}
          </button>
          <button type="button" className="btn btn--primary" onClick={onPrint}>
            {tekster.trykk.knapp}
          </button>
        </div>
      </div>
      <Pris draft={draft} onSize={onSize} />
      <p className="draft__help">{t.hjelp}</p>
      <Laerdommer l={draft.learned} />

      {draft.events.map((e, i) => {
        const maaned = e.start.slice(0, 7);
        const ny = i === 0 || draft.events[i - 1]?.start.slice(0, 7) !== maaned;
        return (
          <div key={e.start + i}>
            {ny && (
              <h2 className="month-h">
                {tekster.bilder.maaned(Number(maaned.slice(5, 7)) - 1, Number(maaned.slice(0, 4)))}
              </h2>
            )}
            <Hendelse
              e={e}
              index={i}
              bilder={perHendelse.get(i) ?? []}
              valgt={valgt}
              onPick={velg}
              thumbUrl={thumbUrl}
            />
          </div>
        );
      })}

      {(valgtBilde || spm || takk) && (
        <div className="dock" role="region" aria-label={tekster.nav.albumutkast}>
          {spm ? (
            <div className="dock__ask">
              <p className="dock__q">
                {
                  tekster.hvorfor[
                    spm.action === "ta_bort" ? "taBort" : spm.action === "ta_med" ? "taMed" : "bytt"
                  ]
                }
              </p>
              <p className="dock__help">{tekster.hvorfor.hjelp}</p>
              <div className="chips" role="group" aria-label={tekster.hvorfor.hjelp}>
                {HVORFOR_VALG[spm.action].map((r) => (
                  <button key={r} type="button" className="chip" onClick={() => svar(r)}>
                    {tekster.hvorfor.svar[r]}
                  </button>
                ))}
                <button type="button" className="btn btn--ghost btn--sm" onClick={() => svar(null)}>
                  {tekster.hvorfor.hoppOver}
                </button>
              </div>
            </div>
          ) : valgtBilde ? (
            <div className="dock__photo">
              <p className="dock__q">
                {tekster.begrunnelse(
                  valgtBilde.reason,
                  draft.events[valgtBilde.event]?.photos ?? 0,
                )}
              </p>
              <div className="dock__actions">
                {valgtBilde.included ? (
                  <button
                    type="button"
                    className="btn btn--secondary"
                    onClick={() => void handling("ta_bort", valgtBilde.id)}
                  >
                    {t.taBort}
                  </button>
                ) : (
                  <>
                    <button
                      type="button"
                      className="btn btn--secondary"
                      onClick={() => void handling("ta_med", valgtBilde.id)}
                    >
                      {t.taMed}
                    </button>
                    {lignende && (
                      <button
                        type="button"
                        className="btn btn--secondary"
                        onClick={() => void handling("bytt", valgtBilde.id, lignende.id)}
                      >
                        {t.byttMedLignende}
                      </button>
                    )}
                  </>
                )}
                <button type="button" className="btn btn--ghost" onClick={() => setValgt(null)}>
                  {t.lukk}
                </button>
              </div>
              <p className="dock__help">{t.byttHjelp}</p>
            </div>
          ) : (
            <p className="dock__q" role="status">
              {tekster.hvorfor.takk}
              <button
                type="button"
                className="btn btn--ghost btn--sm"
                onClick={() => setTakk(false)}
              >
                {t.lukk}
              </button>
            </p>
          )}
        </div>
      )}
    </section>
  );
}
