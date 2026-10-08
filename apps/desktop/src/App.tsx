import { useCallback, useEffect, useRef, useState } from "react";
import {
  api,
  erKommandofeil,
  type Aar,
  type Analysefase,
  type Bilde,
  type Forslag,
  type Fremdrift,
  type Innlesingsrapport,
  type Kilde,
  type KildeType,
  type Handling,
  type Albumtekst,
  type Albumvalg,
  type Ansiktsgruppe,
  type Personer,
  type Ramme,
  type Rolle,
  type Sammendrag,
  type Svar,
  type Utkast,
} from "./api";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { DraftScreen } from "./components/DraftScreen";
import { PeopleScreen } from "./components/PeopleScreen";
import { PrintDialog, type Trykkstatus } from "./components/PrintDialog";
import { PickScreen } from "./components/PickScreen";
import { RecoverScreen } from "./components/RecoverScreen";
import { RecoveryKeyScreen } from "./components/RecoveryKeyScreen";
import { Sidebar, type Visning } from "./components/Sidebar";
import { StartScreen } from "./components/StartScreen";
import { Toast } from "./components/Toast";
import { Welcome } from "./components/Welcome";
import { feiltekst, tekster } from "./tekster";
import { brukValg } from "./utkast";

const TOAST_MS = 3200;
/** Hvor ofte rutenettet oppdateres mens bilder leses inn. */
const LIVE_REFRESH_MS = 3000;

type Fase =
  | { type: "laster" }
  | { type: "velkommen" }
  | { type: "nokkel"; kode: string }
  | { type: "gjenopprett"; feil: string | null }
  | { type: "klar" }
  | { type: "feil"; melding: string };

/** Standardår: forrige kalenderår, ellers året med flest bilder (PRODUCT.md, steg 2). */
export function standardAar(aar: Aar[], naa = new Date()): number | null {
  if (aar.length === 0) return null;
  const forrige = naa.getFullYear() - 1;
  if (aar.some((a) => a.year === forrige)) return forrige;
  return aar.reduce((best, a) => (a.count > best.count ? a : best)).year;
}

export function App() {
  const [fase, setFase] = useState<Fase>({ type: "laster" });
  const [busy, setBusy] = useState(false);
  const [toast, setToast] = useState<string | null>(null);
  const toastTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const [kilder, setKilder] = useState<Kilde[]>([]);
  const [forslag, setForslag] = useState<Forslag[] | null>(null);
  const [aar, setAar] = useState<Aar[]>([]);
  const [valgtAar, setValgtAar] = useState<number | null>(null);
  const [bilder, setBilder] = useState<Bilde[]>([]);
  const [sammendrag, setSammendrag] = useState<Sammendrag | null>(null);
  const [fremdrift, setFremdrift] = useState<Fremdrift | null>(null);
  const [rapport, setRapport] = useState<Innlesingsrapport | null>(null);
  const [bekreftSlett, setBekreftSlett] = useState(false);
  // Albumutkastet er hovedvisningen; «Alle bilder» er for den som vil se alt.
  const [visning, setVisning] = useState<Visning>("utkast");
  const [personer, setPersoner] = useState<Personer | null>(null);
  const [trykk, setTrykk] = useState<{ tekst: Albumtekst | null; status: Trykkstatus } | null>(
    null,
  );
  const [utkast, setUtkast] = useState<Utkast | null>(null);
  const [rammer, setRammer] = useState<Ramme[]>([]);
  /** Hvor mange endringer i utkastet som kan angres. */
  const [kanAngre, setKanAngre] = useState(0);
  // Rammene brukeren kan velge for en side, hentet én gang når utkastet vises.
  const harUtkast = utkast !== null;
  useEffect(() => {
    if (!harUtkast || rammer.length > 0) return;
    api.rammer().then(setRammer, () => {});
  }, [harUtkast, rammer.length]);
  const [analyse, setAnalyse] = useState<Analysefase | null>(null);
  // Valgt år leses av lastKatalog, som ikke skal lages på nytt hver gang året endres.
  const valgtAarRef = useRef<number | null>(null);
  useEffect(() => {
    valgtAarRef.current = valgtAar;
  }, [valgtAar]);

  const visMelding = useCallback((m: string) => {
    setToast(m);
    clearTimeout(toastTimer.current);
    toastTimer.current = setTimeout(() => setToast(null), TOAST_MS);
  }, []);
  useEffect(() => () => clearTimeout(toastTimer.current), []);

  // ---------- Oppstart ----------
  useEffect(() => {
    let avbrutt = false;
    (async () => {
      try {
        const status = await api.lagringStatus();
        if (avbrutt) return;
        if (status === "tom") return setFase({ type: "velkommen" });
        if (status === "trenger_gjenoppretting")
          return setFase({ type: "gjenopprett", feil: null });
        if (status === "laast") await api.aapneLagring();
        if (!avbrutt) setFase({ type: "klar" });
      } catch (e) {
        if (avbrutt) return;
        if (erKommandofeil(e) && e.kode === "trenger_gjenoppretting") {
          setFase({ type: "gjenopprett", feil: null });
        } else {
          setFase({ type: "feil", melding: feiltekst(e) });
        }
      }
    })();
    return () => {
      avbrutt = true;
    };
  }, []);

  // ---------- Katalog ----------
  const lastKatalog = useCallback(async () => {
    const [k, s, a] = await Promise.all([api.kilder(), api.sammendrag(), api.aar()]);
    setKilder(k);
    setSammendrag(s);
    setAar(a);
    const year =
      valgtAarRef.current !== null && a.some((x) => x.year === valgtAarRef.current)
        ? valgtAarRef.current
        : standardAar(a);
    setValgtAar(year);
    setBilder(year === null ? [] : await api.bilderIAar(year));
  }, []);

  const startInnlesing = useCallback(async () => {
    try {
      await api.startInnlesing();
    } catch (e) {
      visMelding(feiltekst(e));
    }
  }, [visMelding]);

  useEffect(() => {
    if (fase.type !== "klar") return;
    let sistOppdatert = 0;
    const lyttere = [
      api.paFremdrift((p) => {
        setFremdrift(p);
        const naa = Date.now();
        if (p.phase === "leser" && naa - sistOppdatert > LIVE_REFRESH_MS) {
          sistOppdatert = naa;
          void lastKatalog();
        }
      }),
      api.paFerdig((r) => {
        setFremdrift(null);
        setRapport(r.report);
        if (r.report?.cancelled) visMelding(tekster.innlesing.stoppet);
        else if (r.report) visMelding(tekster.innlesing.ferdig(r.report.new_photos));
        else visMelding(tekster.feil.ukjent ?? "");
        void lastKatalog();
        void api.personer().then(setPersoner, () => undefined);
      }),
    ];
    (async () => {
      await lastKatalog();
      // Ved oppstart: les inn det som er endret i mappene siden sist (raskt, inkrementelt).
      if ((await api.kilder()).length > 0 && !(await api.innlesingPagar())) await startInnlesing();
    })().catch((e) => visMelding(feiltekst(e)));
    return () => {
      lyttere.forEach((l) => void l.then((stopp) => stopp()));
    };
  }, [fase.type, lastKatalog, startInnlesing, visMelding]);

  const velgAar = async (y: number) => {
    setValgtAar(y);
    if (utkast && utkast.year !== y) setUtkast(null);
    setBilder(await api.bilderIAar(y));
  };

  // ---------- Utkast ----------
  const lagUtkast = async (sidetak: number | null = utkast?.pageCap ?? null) => {
    if (valgtAar === null || analyse) return;
    setAnalyse("henter");
    const stopp = api.paAnalyse((f) => setAnalyse(f));
    try {
      setUtkast(await api.lagUtkast(valgtAar, sidetak));
      setVisning("utkast");
    } catch (e) {
      visMelding(feiltekst(e));
    } finally {
      void stopp.then((s) => s());
      setAnalyse(null);
    }
  };

  /** Et valg for albumet: lagres, og utkastet lages på nytt uten gjennomgangen på skjermen. */
  const endreAlbum = async (change: Albumvalg | Albumvalg[], melding?: string) => {
    if (!utkast) return;
    try {
      await api.albumvalg(utkast.year, change);
      setUtkast(await api.lagUtkast(utkast.year, utkast.pageCap, true));
      setKanAngre(await api.angreAntall(utkast.year));
      visMelding(melding ?? tekster.utkast.oppdatert);
    } catch (e) {
      visMelding(feiltekst(e));
    }
  };

  const velgBilde = async (action: Handling, id: string, other?: string) => {
    if (!utkast) return null;
    try {
      const feedbackId = await api.velgBilde(utkast.year, action, id, other);
      setUtkast((u) => (u ? brukValg(u, action, id, other) : u));
      setKanAngre((n) => n + 1);
      visMelding(
        action === "bytt"
          ? tekster.utkast.byttet
          : action === "ta_med"
            ? tekster.utkast.lagtTil
            : tekster.utkast.tattBort,
      );
      return feedbackId;
    } catch (e) {
      visMelding(feiltekst(e));
      return null;
    }
  };

  /** Tar med eller tar bort mange markerte bilder på en gang. */
  const velgFlere = async (action: "ta_med" | "ta_bort", ids: string[]) => {
    if (!utkast || ids.length === 0) return;
    try {
      await api.velgFlere(utkast.year, action, ids);
      setUtkast((u) => (u ? ids.reduce((x, id) => brukValg(x, action, id), u) : u));
      setKanAngre((n) => n + 1);
      visMelding(
        action === "ta_med"
          ? tekster.redigering.lagtTil(ids.length)
          : tekster.redigering.tattBort(ids.length),
      );
    } catch (e) {
      visMelding(feiltekst(e));
    }
  };

  /** Angrer siste endring og lager utkastet på nytt. */
  const angre = async () => {
    if (!utkast) return;
    try {
      const igjen = await api.angre(utkast.year);
      if (igjen === null) {
        setKanAngre(0);
        return;
      }
      setUtkast(await api.lagUtkast(utkast.year, utkast.pageCap, true));
      setKanAngre(igjen);
      visMelding(tekster.redigering.angret);
    } catch (e) {
      visMelding(feiltekst(e));
    }
  };

  const lagreTekst = async (tekst: Albumtekst) => {
    if (!utkast) return;
    try {
      await api.lagreTekst(utkast.year, tekst);
      visMelding(tekster.redigering.tekstLagret);
    } catch (e) {
      visMelding(feiltekst(e));
    }
  };

  const svarHvorfor = async (feedbackId: number, reason: Svar | null) => {
    try {
      const learned = await api.svarHvorfor(feedbackId, reason);
      setUtkast((u) => (u ? { ...u, learned } : u));
    } catch (e) {
      visMelding(feiltekst(e));
    }
  };

  // ---------- Handlinger ----------
  const opprett = async () => {
    setBusy(true);
    try {
      setFase({ type: "nokkel", kode: await api.opprettLagring() });
    } catch (e) {
      visMelding(feiltekst(e));
    } finally {
      setBusy(false);
    }
  };

  const gjenopprett = async (kode: string) => {
    setBusy(true);
    try {
      await api.gjenopprett(kode);
      setFase({ type: "klar" });
    } catch (e) {
      setFase({ type: "gjenopprett", feil: feiltekst(e) });
    } finally {
      setBusy(false);
    }
  };

  const leggTil = async (path: string, kind?: KildeType) => {
    try {
      await api.leggTilKilde(path, kind);
      const navn = path.split(/[\\/]/).filter(Boolean).pop() ?? path;
      visMelding(tekster.melding.kildeLagtTil(navn));
      await lastKatalog();
      await startInnlesing();
    } catch (e) {
      visMelding(feiltekst(e));
    }
  };

  const velgMappe = async (kind?: KildeType) => {
    const path = await api.velgMappe();
    if (path) await leggTil(path, kind);
  };

  const finnForslag = async () => {
    try {
      setForslag(await api.forslag());
    } catch (e) {
      visMelding(feiltekst(e));
    }
  };

  const fjernKilde = async (k: Kilde) => {
    try {
      await api.fjernKilde(k.id);
      visMelding(tekster.melding.kildeFjernet(k.label));
      await lastKatalog();
    } catch (e) {
      visMelding(feiltekst(e));
    }
  };

  const slettAlt = async () => {
    setBekreftSlett(false);
    try {
      await api.slettAlleData();
      setKilder([]);
      setAar([]);
      setBilder([]);
      setSammendrag(null);
      setRapport(null);
      setForslag(null);
      setValgtAar(null);
      setUtkast(null);
      setFase({ type: "velkommen" });
      visMelding(tekster.slett.ferdig);
    } catch (e) {
      visMelding(feiltekst(e));
    }
  };

  // ---------- Trykkfil ----------
  const visTrykk = async () => {
    if (!utkast) return;
    setTrykk({ tekst: null, status: { type: "skriver" } });
    try {
      const tekst = await api.albumtekst(utkast.year);
      setTrykk({ tekst, status: { type: "skriver" } });
    } catch (e) {
      setTrykk(null);
      visMelding(feiltekst(e));
    }
  };
  const lagTrykk = async (tekst: Albumtekst) => {
    if (!utkast) return;
    const sti = await api.velgTrykkfil(tekster.trykk.filnavn(utkast.year));
    if (!sti) return;
    setTrykk({ tekst, status: { type: "lager", p: null } });
    const stopp = await api.paTrykk((p) => setTrykk({ tekst, status: { type: "lager", p } }));
    try {
      const fil = await api.lagTrykkfil(utkast.year, utkast.pageCap, tekst, sti);
      setTrykk({ tekst, status: { type: "ferdig", fil } });
    } catch (e) {
      setTrykk({ tekst, status: { type: "skriver" } });
      visMelding(feiltekst(e));
    } finally {
      stopp();
    }
  };

  // ---------- Hvem er med? ----------
  useEffect(() => {
    if (visning !== "personer" || fase.type !== "klar") return;
    api.personer().then(setPersoner, (e) => visMelding(feiltekst(e)));
  }, [visning, fase.type, visMelding]);

  const navngi = async (g: Ansiktsgruppe, navn: string, rolle: Rolle, personId: number | null) => {
    if (!navn.trim() && personId === null) return visMelding(tekster.personer.mangler);
    try {
      setPersoner(await api.navngiGruppe(g.group, navn, rolle, personId));
      const kjent = personer?.persons.find((p) => p.id === personId)?.name;
      visMelding(tekster.personer.lagret(navn.trim() || kjent || ""));
    } catch (e) {
      visMelding(feiltekst(e));
    }
  };
  const ignorer = async (g: Ansiktsgruppe) => {
    try {
      setPersoner(await api.ignorerGruppe(g.group));
      visMelding(tekster.personer.ignorert);
    } catch (e) {
      visMelding(feiltekst(e));
    }
  };
  const flytt = async (faceId: number) => {
    try {
      setPersoner(await api.flyttAnsikt(faceId, null));
      visMelding(tekster.personer.flyttet);
    } catch (e) {
      visMelding(feiltekst(e));
    }
  };

  // ---------- Visning ----------
  let innhold: React.ReactNode = null;
  switch (fase.type) {
    case "velkommen":
      innhold = <Welcome onStart={opprett} busy={busy} />;
      break;
    case "nokkel":
      innhold = <RecoveryKeyScreen code={fase.kode} onDone={() => setFase({ type: "klar" })} />;
      break;
    case "gjenopprett":
      innhold = <RecoverScreen onSubmit={gjenopprett} error={fase.feil} busy={busy} />;
      break;
    case "feil":
      innhold = (
        <p className="empty" role="alert">
          {fase.melding}
        </p>
      );
      break;
    case "klar":
      innhold =
        // Startsiden forblir kildesiden til «Lag utkast» trykkes; albumutkastet vises først
        // når gjennomgangen er i gang.
        kilder.length === 0 || (visning === "utkast" && !utkast && !analyse) ? (
          <StartScreen
            onPickSource={(k) => void velgMappe(k)}
            onFindSuggestions={() => void finnForslag()}
            onAddSuggestion={(f) => void leggTil(f.path, f.kind)}
            suggestions={forslag}
            sources={kilder}
            years={aar}
            year={valgtAar}
            onYear={(y) => void velgAar(y)}
            ingest={fremdrift}
            onMake={() => void lagUtkast()}
          />
        ) : visning === "personer" ? (
          <PeopleScreen
            people={personer}
            thumbUrl={api.miniatyrUrl}
            onName={(g, n, r, p) => void navngi(g, n, r, p)}
            onIgnore={(g) => void ignorer(g)}
            onMove={(f) => void flytt(f)}
          />
        ) : visning === "utkast" ? (
          <DraftScreen
            year={valgtAar}
            draft={utkast}
            phase={analyse}
            onMake={() => void lagUtkast()}
            onPrint={() => void visTrykk()}
            onChange={(c, m) => void endreAlbum(c, m)}
            onSize={(sidetak) => void lagUtkast(sidetak)}
            onChoose={velgBilde}
            onChooseMany={(a, ids) => void velgFlere(a, ids)}
            canUndo={kanAngre > 0}
            onUndo={() => void angre()}
            onSaveText={(x) => void lagreTekst(x)}
            onAnswer={(id, r) => void svarHvorfor(id, r)}
            thumbUrl={api.miniatyrUrl}
            stortUrl={api.stortUrl}
            hentTekst={api.albumtekst}
            rammer={rammer}
          />
        ) : (
          <PickScreen
            years={aar}
            year={valgtAar}
            onYear={(y) => void velgAar(y)}
            photos={bilder}
            summary={sammendrag}
            progress={fremdrift}
            report={rapport}
            onCancel={() => void api.avbrytInnlesing()}
            thumbUrl={api.miniatyrUrl}
          />
        );
      break;
  }

  return (
    <div className="app">
      <Sidebar
        sources={kilder}
        locked={fase.type !== "klar"}
        view={kilder.length === 0 ? null : visning}
        onView={setVisning}
        draftPages={utkast?.pages ?? null}
        onAddFolder={() => void velgMappe()}
        onRemoveSource={(k) => void fjernKilde(k)}
        onDeleteAll={() => setBekreftSlett(true)}
      />
      <main className="main">{innhold}</main>
      <ConfirmDialog
        open={bekreftSlett}
        title={tekster.slett.tittel}
        text={tekster.slett.tekst}
        confirmLabel={tekster.slett.bekreft}
        cancelLabel={tekster.slett.avbryt}
        onConfirm={() => void slettAlt()}
        onCancel={() => setBekreftSlett(false)}
      />
      {utkast && (
        <PrintDialog
          open={trykk !== null}
          year={utkast.year}
          text={trykk?.tekst ?? null}
          status={trykk?.status ?? { type: "skriver" }}
          onMake={(t) => void lagTrykk(t)}
          onClose={() => setTrykk(null)}
        />
      )}
      <Toast message={toast} />
    </div>
  );
}
