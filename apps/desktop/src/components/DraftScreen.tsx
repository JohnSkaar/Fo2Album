import { useEffect, useMemo, useRef, useState } from "react";
import type {
  Albumtekst,
  Albumvalg,
  Analysefase,
  Omslag,
  Handling,
  Ramme,
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
import { Bla } from "./Bla";
import { type Dra, RammeIkon, Sidebilde } from "./Side";
import { StorVisning } from "./StorVisning";

const t = tekster.utkast;
const r = tekster.redigering;
/** Så mange bilder som ikke er med, vises før «Vis alle». */
const VIS_IKKE_MED = 8;
const FASER: Analysefase[] = ["henter", "hendelser", "serier", "velger", "begrunnelser"];

type Valgt = { id: string; included: boolean; event: number };
type Endre = (c: Albumvalg | Albumvalg[], melding?: string) => void;

/** Siden som «Fornøyd»-valg, med nye bilder (rammene beholdes bare når antallet er likt). */
function somValgt(s: Side, photos: string[]): Albumvalg {
  const kind: Side["kind"] =
    photos.length > 1 ? "rutenett" : s.kind === "rutenett" ? "luft" : s.kind;
  return {
    type: "fornoyd",
    page: {
      kind,
      kolonner: kind === "rutenett" ? (s.kolonner ?? (photos.length > 4 ? 3 : 2)) : null,
      photos,
      mal: photos.length === s.photos.length ? (s.mal ?? null) : null,
    },
    on: true,
  };
}

/**
 * Flytter et bilde til en side, foran bildet `foran` (ellers sist), eller til et annet sted
 * på samme side. Sidene brukeren har ordnet, merkes «Fornøyd» så de beholdes. `fra` er siden
 * bildet står på nå (mangler for et bilde som ikke er med).
 */
export function flytt(
  fra: Side | undefined,
  til: Side,
  id: string,
  foran: string | null,
): Albumvalg[] {
  if (foran === id) return [];
  const uten = til.photos.filter((x) => x !== id);
  const k = foran ? uten.indexOf(foran) : -1;
  const nye = k < 0 ? [...uten, id] : [...uten.slice(0, k), id, ...uten.slice(k)];
  if (fra && fra.photos[0] === til.photos[0]) {
    return nye.join() === til.photos.join() ? [] : [somValgt(til, nye)];
  }
  const ut: Albumvalg[] = [somValgt(til, nye)];
  const igjen = fra?.photos.filter((x) => x !== id) ?? [];
  if (fra && igjen.length > 0) ut.push(somValgt(fra, igjen));
  return ut;
}

/** Neste størrelse når brukeren trykker «Større» eller «Mindre» (som prototypen). */
export function nyStorrelse(
  size: number | null,
  side: Side | undefined,
  retning: 1 | -1,
): number | null {
  const alene = side?.photos.length === 1;
  if (retning === 1) {
    if (alene) return 4;
    return Math.min(3, Math.max(0, size ?? 0) + 1);
  }
  if (alene) return side?.kind === "helside" ? 3 : 0;
  return Math.max(-1, (size ?? 0) - 1);
}

/** Brukerens rotering vist på miniatyren (stående bilder skaleres så de får plass). */
const rotert = (deg: number | undefined): React.CSSProperties | undefined =>
  deg ? { transform: `rotate(${deg}deg)${deg % 180 ? " scale(0.75)" : ""}` } : undefined;
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
  markert,
  liten,
  onClick,
  onMark,
  onDrag,
  thumbUrl,
}: {
  b: UtkastBilde;
  hendelse: UtkastHendelse;
  valgt: boolean;
  markert: boolean;
  liten?: boolean;
  onClick: () => void;
  onMark: () => void;
  onDrag?: () => void;
  thumbUrl: (id: string) => string;
}) {
  const hvorfor = tekster.begrunnelse(b.reason, hendelse.photos);
  return (
    <li className={`dcwrap${markert ? " dcwrap--markert" : ""}`}>
      <button
        type="button"
        className="dcard__mark"
        aria-pressed={markert}
        aria-label={`${r.marker}: ${t.bildeEtikett(datoTekst(b.takenAt), b.included)}`}
        title={r.markerHjelp}
        onClick={onMark}
      >
        ✓
      </button>
      <button
        type="button"
        className={`dcard${liten ? " dcard--small" : ""}`}
        aria-pressed={valgt}
        aria-label={`${t.bildeEtikett(datoTekst(b.takenAt), b.included)}. ${hvorfor}`}
        onClick={(e) => (e.ctrlKey || e.metaKey || e.shiftKey ? onMark() : onClick())}
        draggable={!!onDrag}
        onDragStart={(e) => {
          e.dataTransfer?.setData("text/plain", b.id);
          onDrag?.();
        }}
      >
        <span className="th">
          {b.hasThumbnail ? (
            <img
              src={thumbUrl(b.id)}
              alt=""
              loading="lazy"
              decoding="async"
              style={rotert(b.rotation)}
            />
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

/** «Rammer for siden»: automatisk eller faste rammer med like mange felt som bilder. */
function Rammevalg({ s, rammer, onChange }: { s: Side; rammer: Ramme[]; onChange: Endre }) {
  const passer = rammer.filter((r) => r.photos === s.photos.length);
  const velg = (mal: string | null) =>
    onChange(
      {
        type: "fornoyd",
        page: { kind: s.kind, kolonner: s.kolonner ?? null, photos: s.photos, mal },
        on: true,
      },
      mal ? tekster.sider.malValgt : t.fornoydMelding,
    );
  return (
    <div className="tplbar" role="group" aria-label={tekster.sider.rammerTittel}>
      <p className="dock__help">{tekster.sider.rammerIngress}</p>
      <div className="tpls">
        <button
          type="button"
          className="tpl"
          aria-pressed={!s.mal}
          title={tekster.sider.autoHjelp}
          onClick={() => velg(null)}
        >
          <span className="tpl__auto" aria-hidden="true">
            A
          </span>
          <span>{tekster.sider.auto}</span>
        </button>
        {passer.map((r) => (
          <button
            key={r.id}
            type="button"
            className="tpl"
            aria-pressed={s.mal === r.id}
            onClick={() => velg(r.id)}
          >
            <RammeIkon felt={r.frames} />
            <span>{tekster.sider.mal[r.id] ?? r.id}</span>
          </button>
        ))}
      </div>
      {passer.length === 0 && <p className="ev__none">{tekster.sider.ingenMaler}</p>}
    </div>
  );
}

function Sider({
  sider,
  med,
  bilder,
  valgt,
  markert,
  rammer,
  onPhoto,
  onChange,
  dra,
  thumbUrl,
}: {
  sider: Side[];
  /** Bildene som er med nå, i tidsrekkefølge. */
  med: string[];
  bilder: Map<string, UtkastBilde>;
  valgt: string | null;
  markert: Set<string>;
  rammer: Ramme[];
  onPhoto: (id: string, marker: boolean) => void;
  onChange: Endre;
  /** Dra og slipp til denne siden. */
  dra: (s: Side) => Dra;
  thumbUrl: (id: string) => string;
}) {
  const { sider: vis, endret } = oppdaterSider(sider, med);
  const [aapen, setAapen] = useState<string | null>(null);
  return (
    <div className="pages">
      <h4 className="ev__colh">{tekster.sider.tittel}</h4>
      <ol className="pages__list">
        {vis.map((s, i) => {
          const nokkel = s.photos[0]!;
          const etikett = `${tekster.sider.side(i + 1)}: ${
            s.kind === "rutenett" ? tekster.sider.rutenett(s.photos.length) : tekster.sider[s.kind]
          }`;
          return (
            <li key={nokkel} className={`pgwrap${aapen === nokkel ? " pgwrap--open" : ""}`}>
              <Sidebilde
                side={s}
                bilder={bilder}
                thumbUrl={thumbUrl}
                etikett={etikett}
                valgt={valgt}
                markert={markert}
                onPhoto={onPhoto}
                dra={endret ? undefined : dra(s)}
              />
              <span className="pg__btns">
                <button
                  type="button"
                  className="pg__lock"
                  aria-pressed={!!s.locked}
                  title={tekster.sider.fornoydHjelp}
                  aria-label={`${tekster.sider.side(i + 1)}: ${tekster.sider.fornoyd}`}
                  onClick={() =>
                    onChange(
                      {
                        type: "fornoyd",
                        page: {
                          kind: s.kind,
                          kolonner: s.kolonner ?? null,
                          photos: s.photos,
                          mal: s.mal ?? null,
                        },
                        on: !s.locked,
                      },
                      s.locked ? t.ikkeFornoydMelding : t.fornoydMelding,
                    )
                  }
                >
                  {s.locked ? tekster.sider.erFornoyd : tekster.sider.fornoyd}
                </button>
                <button
                  type="button"
                  className="pg__lock"
                  aria-expanded={aapen === nokkel}
                  title={tekster.sider.rammerHjelp}
                  aria-label={`${tekster.sider.side(i + 1)}: ${tekster.sider.rammer}`}
                  onClick={() => setAapen(aapen === nokkel ? null : nokkel)}
                >
                  {tekster.sider.rammer}
                </button>
              </span>
              {aapen === nokkel && <Rammevalg s={s} rammer={rammer} onChange={onChange} />}
            </li>
          );
        })}
      </ol>
      {endret && <p className="ev__none">{tekster.sider.endret}</p>}
    </div>
  );
}

/** Turspørsmålet, sider per historie og «del opp igjen». */
function Verktoy({ e, onChange }: { e: UtkastHendelse; onChange: Endre }) {
  const [n, setN] = useState(e.pageTarget ?? e.pages);
  const tur = e.looksLikeTrip && !e.everyday;
  return (
    <div className="ev__tools">
      {tur && e.adultTripGuess && !e.tripAnswered && (
        <p className="ev__ask">
          <span>{t.turGjettet}</span>
          <button
            type="button"
            className="btn btn--ghost btn--sm"
            onClick={() => onChange({ type: "tur_uten_barn", photo: e.key, on: false })}
          >
            {t.turNei}
          </button>
        </p>
      )}
      {tur && !e.adultTripGuess && !e.tripAnswered && (
        <p className="ev__ask">
          <span>{t.turSporsmaal}</span>
          <button
            type="button"
            className="btn btn--secondary btn--sm"
            onClick={() => onChange({ type: "tur_uten_barn", photo: e.key, on: true })}
          >
            {t.turJa}
          </button>
          <button
            type="button"
            className="btn btn--ghost btn--sm"
            onClick={() => onChange({ type: "tur_uten_barn", photo: e.key, on: false })}
          >
            {t.turNei}
          </button>
        </p>
      )}
      <div className="ev__row">
        {tur && e.tripAnswered && (
          <button
            type="button"
            className="chip"
            aria-pressed={e.adultTrip}
            onClick={() => onChange({ type: "tur_uten_barn", photo: e.key, on: !e.adultTrip })}
          >
            {e.adultTrip ? `✓ ${t.turUtenBarn}` : `${t.turUtenBarn}?`}
          </button>
        )}
        {!e.everyday && (
          <span className="ev__pages" role="group" aria-label={t.presenter}>
            {t.presenter}
            <button
              type="button"
              className="btn btn--ghost btn--sm"
              aria-label={t.farreSider}
              onClick={() => setN(Math.max(1, n - 1))}
            >
              −
            </button>
            <b aria-live="polite">{n}</b>
            <button
              type="button"
              className="btn btn--ghost btn--sm"
              aria-label={t.flereSider}
              onClick={() => setN(n + 1)}
            >
              +
            </button>
            {t.sider(n)}
            {n !== e.pages && (
              <button
                type="button"
                className="btn btn--secondary btn--sm"
                onClick={() => onChange({ type: "sider", photo: e.key, pages: n })}
              >
                {t.lagForslag}
              </button>
            )}
            {e.pageTarget !== null && (
              <button
                type="button"
                className="btn btn--ghost btn--sm"
                onClick={() => onChange({ type: "sider", photo: e.key, pages: null })}
              >
                {t.appensForslag}
              </button>
            )}
          </span>
        )}
        {e.merged && (
          <button
            type="button"
            className="btn btn--ghost btn--sm"
            onClick={() => onChange({ type: "del_opp", photo: e.key })}
          >
            {t.delOpp}
          </button>
        )}
      </div>
    </div>
  );
}

function Hendelse({
  e,
  index,
  bilder,
  valgt,
  markert,
  alleBilder,
  rammer,
  onPick,
  onMark,
  onChange,
  dra,
  thumbUrl,
}: {
  e: UtkastHendelse;
  index: number;
  bilder: UtkastBilde[];
  valgt: Valgt | null;
  markert: Set<string>;
  alleBilder: Map<string, UtkastBilde>;
  rammer: Ramme[];
  onPick: (b: UtkastBilde) => void;
  onMark: (id: string) => void;
  onChange: Endre;
  dra: { side: (s: Side) => Dra; start: (id: string) => void };
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
        {e.ownStory && (
          <span className="ev__own">
            <span className="ev__tag">{r.egenHistorieMerke}</span>
            <button
              type="button"
              className="btn btn--ghost btn--sm"
              onClick={() => onChange({ type: "legg_tilbake", photo: e.key }, r.lagtTilbake)}
            >
              {r.leggTilbake}
            </button>
          </span>
        )}
      </header>
      <Verktoy key={`${e.key}-${e.pages}`} e={e} onChange={onChange} />
      <Sider
        sider={e.layout}
        med={med.map((b) => b.id)}
        bilder={alleBilder}
        valgt={valgt?.id ?? null}
        markert={markert}
        rammer={rammer}
        onPhoto={(id, marker) => {
          const b = alleBilder.get(id);
          if (marker) onMark(id);
          else if (b) onPick(b);
        }}
        onChange={onChange}
        dra={dra.side}
        thumbUrl={thumbUrl}
      />
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
                markert={markert.has(b.id)}
                onClick={() => onPick(b)}
                onMark={() => onMark(b.id)}
                onDrag={() => dra.start(b.id)}
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
                  markert={markert.has(b.id)}
                  onClick={() => onPick(b)}
                  onMark={() => onMark(b.id)}
                  onDrag={() => dra.start(b.id)}
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

function Omslagsvalg({
  o,
  rotasjon,
  onChange,
  thumbUrl,
}: {
  o: Omslag;
  rotasjon: Map<string, number>;
  onChange: Endre;
  thumbUrl: (id: string) => string;
}) {
  const bilde = (id: string, merke?: string) => (
    <span className="cover__img">
      <img src={thumbUrl(id)} alt="" loading="lazy" style={rotert(rotasjon.get(id))} />
      {merke && <span className="cover__tag">{merke}</span>}
    </span>
  );
  const kandidat = (id: string) => (
    <li key={id} className="cover__cand">
      {bilde(id, id === o.front ? t.forside : id === o.back ? t.bakside : undefined)}
      <span className="cover__btns">
        <button
          type="button"
          className="chip"
          aria-pressed={id === o.front}
          onClick={() => onChange({ type: "forside", photo: id })}
        >
          {t.forside}
        </button>
        <button
          type="button"
          className="chip"
          aria-pressed={id === o.back}
          onClick={() => onChange({ type: "bakside", photo: id })}
        >
          {t.bakside}
        </button>
      </span>
    </li>
  );
  return (
    <details className="cover">
      <summary>{t.omslag}</summary>
      <p className="price__help">{t.omslagHjelp}</p>
      <div className="cover__now">
        {o.front && (
          <figure>
            {bilde(o.front)}
            <figcaption>
              {t.forside} · {o.chosenFront ? t.valgtAvDeg : t.forslag}
            </figcaption>
          </figure>
        )}
        {o.back && (
          <figure>
            {bilde(o.back)}
            <figcaption>
              {t.bakside} · {o.chosenBack ? t.valgtAvDeg : t.forslag}
            </figcaption>
          </figure>
        )}
      </div>
      {o.people.length > 0 && (
        <>
          <h4 className="ev__colh">{t.medPersoner}</h4>
          <ul className="cover__list">{o.people.map(kandidat)}</ul>
        </>
      )}
      {o.overview.length > 0 && (
        <>
          <h4 className="ev__colh">{t.oversikt}</h4>
          <ul className="cover__list">{o.overview.map(kandidat)}</ul>
        </>
      )}
    </details>
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
  rammer = [],
  onMake,
  onPrint,
  onChange,
  onSize,
  onChoose,
  onChooseMany,
  onAnswer,
  canUndo = false,
  onUndo,
  onSaveText,
  thumbUrl,
  stortUrl = thumbUrl,
  hentTekst,
}: {
  /** Året albumet lages for (til tittelen mens gjennomgangen pågår). */
  year: number | null;
  draft: Utkast | null;
  phase: Analysefase | null;
  /** Faste rammer brukeren kan velge for en side. */
  rammer?: Ramme[];
  onMake: () => void;
  /** «Lag trykkfil»: albumet som PDF. */
  onPrint: () => void;
  /** Valg for albumet (fornøyd, rammer, tur, sider, slå sammen, omslag, rotering, størrelse,
   * utsnitt, samle på én side, egen historie). Flere valg lagres samlet. */
  onChange: Endre;
  /** Lager utkastet på nytt med et sidetak (`null` = hele historien). */
  onSize: (sidetak: number | null) => void;
  /** Lagrer valget og returnerer id-en til loggføringen (til «Hvorfor?»), eller `null`. */
  onChoose: (action: Handling, id: string, other?: string) => Promise<number | null>;
  /** Tar med eller tar bort mange bilder på en gang (angres samlet). */
  onChooseMany?: (action: "ta_med" | "ta_bort", ids: string[]) => void;
  onAnswer: (feedbackId: number, reason: Svar | null) => void;
  /** Det finnes endringer som kan angres. */
  canUndo?: boolean;
  onUndo?: () => void;
  /** Lagrer tekstene skrevet i «Bla i albumet». */
  onSaveText?: (tekst: Albumtekst) => void;
  thumbUrl: (id: string) => string;
  /** Stor visning fra originalfilen (faller tilbake til miniatyren). */
  stortUrl?: (id: string) => string;
  /** Tekstene i albumet, til «Bla i albumet». */
  hentTekst?: (year: number) => Promise<Albumtekst>;
}) {
  const [valgt, setValgt] = useState<Valgt | null>(null);
  const [markert, setMarkert] = useState<Set<string>>(new Set());
  const [stor, setStor] = useState<{ id: string; utsnitt: boolean } | null>(null);
  const [bla, setBla] = useState(false);
  const [tekst, setTekst] = useState<Albumtekst | null>(null);
  const [spm, setSpm] = useState<Spørsmål | null>(null);
  const [handlinger, setHandlinger] = useState(0);
  const [hoppetOver, setHoppetOver] = useState(0);
  const [takk, setTakk] = useState(false);
  const [avslatt, setAvslatt] = useState<Set<string>>(new Set());

  const perHendelse = useMemo(() => {
    const m = new Map<number, UtkastBilde[]>();
    for (const b of draft?.photos ?? []) {
      const l = m.get(b.event);
      if (l) l.push(b);
      else m.set(b.event, [b]);
    }
    return m;
  }, [draft]);
  const alleBilder = useMemo(
    () => new Map<string, UtkastBilde>((draft?.photos ?? []).map((b) => [b.id, b])),
    [draft],
  );
  const draId = useRef<string | null>(null);

  // Ctrl+Z (Cmd+Z på Mac) angrer, utenom når brukeren skriver.
  const vises = !!draft && !phase;
  useEffect(() => {
    if (!vises || !onUndo) return;
    const tast = (e: KeyboardEvent) => {
      const el = e.target as HTMLElement | null;
      if (el?.closest("input, textarea, [contenteditable]")) return;
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key.toLowerCase() === "z") {
        e.preventDefault();
        onUndo();
      }
    };
    window.addEventListener("keydown", tast);
    return () => window.removeEventListener("keydown", tast);
  }, [vises, onUndo]);

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

  const marker = (id: string) => {
    const m = new Set(markert);
    if (m.has(id)) m.delete(id);
    else m.add(id);
    setMarkert(m);
    setValgt(null);
  };

  /** De markerte bildene i tidsrekkefølge. */
  const markerte = draft.photos.filter((b) => markert.has(b.id));
  const flere = async (action: "ta_med" | "ta_bort") => {
    const ids = markerte.filter((b) => b.included === (action === "ta_bort")).map((b) => b.id);
    if (onChooseMany) onChooseMany(action, ids);
    else for (const id of ids) await onChoose(action, id);
    setMarkert(new Set());
  };
  const samlet = (c: Albumvalg | Albumvalg[], melding: string) => {
    onChange(c, melding);
    setMarkert(new Set());
  };

  const alleSider = draft.events.flatMap((e) => e.layout);
  const sideMed = (id: string) => alleSider.find((s) => s.photos.includes(id));
  const flyttTil = (id: string, til: Side, foran: string | null) => {
    const c = flytt(sideMed(id), til, id, foran);
    if (c.length > 0) onChange(c, r.flyttet);
  };
  const dra = {
    start: (id: string) => {
      draId.current = id;
    },
    side: (s: Side): Dra => ({
      start: (id) => {
        draId.current = id;
      },
      slipp: (foran) => {
        const id = draId.current;
        draId.current = null;
        if (id) flyttTil(id, s, foran);
      },
    }),
  };
  const feltFor = (id: string) => {
    const s = sideMed(id);
    const i = s?.photos.indexOf(id) ?? -1;
    return s?.frames && i >= 0 ? (s.frames[i] ?? null) : null;
  };

  const valgtBilde = valgt ? alleBilder.get(valgt.id) : undefined;
  const valgtSide = valgtBilde ? sideMed(valgtBilde.id) : undefined;
  const valgtFelt = valgtBilde ? feltFor(valgtBilde.id) : null;
  const alene = valgtSide?.photos.length === 1;
  // Sidene før og etter i samme historie, for «Forrige side» og «Neste side».
  const historieSider = valgtBilde ? (draft.events[valgtBilde.event]?.layout ?? []) : [];
  const sideNr = valgtSide ? historieSider.indexOf(valgtSide) : -1;
  const forrigeSide = sideNr > 0 ? historieSider[sideNr - 1] : undefined;
  const nesteSide = sideNr >= 0 ? historieSider[sideNr + 1] : undefined;
  const lignende =
    valgtBilde && !valgtBilde.included && valgtBilde.related
      ? draft.photos.find((b) => b.id === valgtBilde.related && b.included)
      : undefined;
  const storrelse = (retning: 1 | -1) => {
    if (!valgtBilde) return;
    const size = nyStorrelse(valgtBilde.size, valgtSide, retning);
    onChange(
      { type: "storrelse", photo: valgtBilde.id, size },
      r.storrelse(size, size !== null && size >= 3, size === 4),
    );
  };
  const antallMed = draft.photos.filter((b) => b.included).length;
  const rotasjon = new Map(draft.photos.filter((b) => b.rotation).map((b) => [b.id, b.rotation]));
  const hendelserMed = draft.events.filter((e) => e.included > 0).length;
  const storBilde = stor ? alleBilder.get(stor.id) : undefined;

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
          {onUndo && (
            <button
              type="button"
              className="btn btn--secondary"
              disabled={!canUndo}
              title={r.angreHjelp}
              onClick={onUndo}
            >
              ↶ {r.angre}
            </button>
          )}
          <button type="button" className="btn btn--secondary" onClick={onMake}>
            {t.nyttUtkast}
          </button>
          <button
            type="button"
            className="btn btn--secondary"
            onClick={() => {
              setBla(true);
              hentTekst?.(draft.year).then(setTekst, () => {});
            }}
          >
            {r.bla}
          </button>
          <button type="button" className="btn btn--primary" onClick={onPrint}>
            {tekster.trykk.knapp}
          </button>
        </div>
      </div>
      <Pris draft={draft} onSize={onSize} />
      <Omslagsvalg o={draft.cover} rotasjon={rotasjon} onChange={onChange} thumbUrl={thumbUrl} />
      <p className="draft__help">{t.hjelp}</p>
      <Laerdommer l={draft.learned} />

      {draft.events.map((e, i) => {
        const maaned = e.start.slice(0, 7);
        const ny = i === 0 || draft.events[i - 1]?.start.slice(0, 7) !== maaned;
        // Forslag om å slå sammen korte dager som starter her.
        const forslag = draft.mergeSuggestions.find((m) => m[0] === i);
        const forslagNokkel = forslag?.map((k) => draft.events[k]?.key).join(",") ?? "";
        const forslagEvents = forslag
          ?.map((k) => draft.events[k])
          .filter((x): x is UtkastHendelse => !!x);
        return (
          <div key={e.start + i}>
            {ny && (
              <h2 className="month-h">
                {tekster.bilder.maaned(Number(maaned.slice(5, 7)) - 1, Number(maaned.slice(0, 4)))}
              </h2>
            )}
            {forslagEvents && forslagEvents.length > 1 && !avslatt.has(forslagNokkel) && (
              <p className="merge" role="note">
                <span>
                  {t.slaaSammenForslag(
                    datoSpenn(
                      forslagEvents[0]!.start,
                      forslagEvents[forslagEvents.length - 1]!.end,
                    ),
                  )}
                </span>
                <button
                  type="button"
                  className="btn btn--secondary btn--sm"
                  onClick={() =>
                    onChange({ type: "slaa_sammen", photos: forslagEvents.map((x) => x.key) })
                  }
                >
                  {t.slaaSammen}
                </button>
                <button
                  type="button"
                  className="btn btn--ghost btn--sm"
                  onClick={() => setAvslatt(new Set(avslatt).add(forslagNokkel))}
                >
                  {t.neiTakk}
                </button>
              </p>
            )}
            <Hendelse
              e={e}
              index={i}
              bilder={perHendelse.get(i) ?? []}
              valgt={valgt}
              markert={markert}
              alleBilder={alleBilder}
              rammer={rammer}
              onPick={velg}
              onMark={marker}
              onChange={onChange}
              dra={dra}
              thumbUrl={thumbUrl}
            />
          </div>
        );
      })}

      {(valgtBilde || spm || takk || markert.size > 0) && (
        <div className="dock" role="region" aria-label={tekster.nav.albumutkast}>
          {markert.size > 0 ? (
            <div className="dock__photo">
              <p className="dock__q" role="status">
                {r.markert(markert.size)}
              </p>
              <div className="dock__actions">
                <button
                  type="button"
                  className="btn btn--secondary"
                  title={r.samleSideHjelp}
                  onClick={() =>
                    samlet(
                      { type: "samle_side", photos: markerte.map((b) => b.id) },
                      r.samlet(markerte.length),
                    )
                  }
                >
                  {r.samleSide}
                </button>
                <button
                  type="button"
                  className="btn btn--secondary"
                  title={r.egenHistorieHjelp}
                  onClick={() =>
                    samlet(
                      { type: "egen_historie", photos: markerte.map((b) => b.id) },
                      r.historieLaget(markerte.length),
                    )
                  }
                >
                  {r.egenHistorie}
                </button>
                <button
                  type="button"
                  className="btn btn--secondary"
                  onClick={() =>
                    samlet(
                      markerte.map((b) => ({ type: "roter", photo: b.id }) as Albumvalg),
                      r.rotert(markerte.length),
                    )
                  }
                >
                  ↻ {t.roter}
                </button>
                {markerte.some((b) => !b.included) && (
                  <button
                    type="button"
                    className="btn btn--secondary"
                    onClick={() => void flere("ta_med")}
                  >
                    {t.taMed}
                  </button>
                )}
                {markerte.some((b) => b.included) && (
                  <button
                    type="button"
                    className="btn btn--secondary"
                    onClick={() => void flere("ta_bort")}
                  >
                    {t.taBort}
                  </button>
                )}
                <button
                  type="button"
                  className="btn btn--ghost"
                  onClick={() => setMarkert(new Set())}
                >
                  {r.fjernMarkering}
                </button>
              </div>
              <p className="dock__help">{r.markerHjelp}</p>
            </div>
          ) : spm ? (
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
                {HVORFOR_VALG[spm.action].map((x) => (
                  <button key={x} type="button" className="chip" onClick={() => svar(x)}>
                    {tekster.hvorfor.svar[x]}
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
              {valgtBilde.included && valgtSide && !valgtSide.mal && (
                <p className="dock__help" role="status">
                  {r.storrelse(valgtBilde.size, alene, valgtSide.kind === "helside")}
                </p>
              )}
              <div className="dock__actions">
                {valgtBilde.included ? (
                  <>
                    <button
                      type="button"
                      className="btn btn--secondary"
                      onClick={() => void handling("ta_bort", valgtBilde.id)}
                    >
                      {t.taBort}
                    </button>
                    <button
                      type="button"
                      className="btn btn--secondary"
                      title={r.storreHjelp}
                      disabled={alene && valgtSide?.kind === "helside"}
                      onClick={() => storrelse(1)}
                    >
                      {r.storre}
                    </button>
                    <button
                      type="button"
                      className="btn btn--secondary"
                      title={r.mindreHjelp}
                      disabled={!alene && valgtBilde.size === -1}
                      onClick={() => storrelse(-1)}
                    >
                      {r.mindre}
                    </button>
                  </>
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
                <button
                  type="button"
                  className="btn btn--secondary"
                  title={t.roterHjelp}
                  onClick={() => onChange({ type: "roter", photo: valgtBilde.id })}
                >
                  ↻ {t.roter}
                </button>
                {forrigeSide && (
                  <button
                    type="button"
                    className="btn btn--secondary"
                    title={r.flyttHjelp}
                    onClick={() => flyttTil(valgtBilde.id, forrigeSide, null)}
                  >
                    {r.forrigeSide}
                  </button>
                )}
                {nesteSide && (
                  <button
                    type="button"
                    className="btn btn--secondary"
                    title={r.flyttHjelp}
                    onClick={() => flyttTil(valgtBilde.id, nesteSide, nesteSide.photos[0] ?? null)}
                  >
                    {r.nesteSide}
                  </button>
                )}
                <button
                  type="button"
                  className="btn btn--secondary"
                  onClick={() => setStor({ id: valgtBilde.id, utsnitt: false })}
                >
                  {r.visStort}
                </button>
                {valgtFelt && (valgtFelt.fill || valgtBilde.whole) && (
                  <button
                    type="button"
                    className="btn btn--secondary"
                    onClick={() => setStor({ id: valgtBilde.id, utsnitt: true })}
                  >
                    {r.utsnitt}
                  </button>
                )}
                <button
                  type="button"
                  className="btn btn--secondary"
                  onClick={() => onChange({ type: "forside", photo: valgtBilde.id })}
                >
                  {t.brukForside}
                </button>
                <button
                  type="button"
                  className="btn btn--secondary"
                  onClick={() => onChange({ type: "bakside", photo: valgtBilde.id })}
                >
                  {t.brukBakside}
                </button>
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
      {storBilde && stor && (
        <StorVisning
          key={stor.id + String(stor.utsnitt)}
          b={storBilde}
          felt={feltFor(storBilde.id)}
          startUtsnitt={stor.utsnitt}
          stortUrl={stortUrl}
          thumbUrl={thumbUrl}
          onChange={onChange}
          onClose={() => setStor(null)}
        />
      )}
      {bla && (
        <Bla
          draft={draft}
          tekst={tekst}
          thumbUrl={thumbUrl}
          onClose={() => setBla(false)}
          onSaveText={
            onSaveText &&
            ((x) => {
              setTekst(x);
              onSaveText(x);
            })
          }
        />
      )}
    </section>
  );
}
