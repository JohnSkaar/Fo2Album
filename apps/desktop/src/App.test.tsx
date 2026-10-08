import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Aar, Bilde, Sammendrag, Utkast } from "./api";
import { api } from "./api";
import { App, standardAar } from "./App";
import { tekster } from "./tekster";
import { spoerOmHvorfor } from "./utkast";

vi.mock("./api", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./api")>();
  const noop = () => Promise.resolve(() => {});
  return {
    ...actual,
    api: {
      lagringStatus: vi.fn(),
      opprettLagring: vi.fn(),
      aapneLagring: vi.fn(() => Promise.resolve()),
      gjenopprett: vi.fn(),
      slettAlleData: vi.fn(() => Promise.resolve()),
      kilder: vi.fn(() => Promise.resolve([])),
      forslag: vi.fn(() => Promise.resolve([])),
      leggTilKilde: vi.fn(() => Promise.resolve(1)),
      fjernKilde: vi.fn(() => Promise.resolve()),
      velgMappe: vi.fn(() => Promise.resolve(null)),
      startInnlesing: vi.fn(() => Promise.resolve()),
      avbrytInnlesing: vi.fn(() => Promise.resolve()),
      innlesingPagar: vi.fn(() => Promise.resolve(false)),
      paFremdrift: vi.fn(noop),
      paFerdig: vi.fn(noop),
      sammendrag: vi.fn(() => Promise.resolve(tomtSammendrag)),
      aar: vi.fn(() => Promise.resolve([])),
      bilderIAar: vi.fn(() => Promise.resolve([])),
      miniatyrUrl: (id: string) => `miniatyr://localhost/${id}`,
      lagUtkast: vi.fn(),
      paAnalyse: vi.fn(noop),
      velgBilde: vi.fn(() => Promise.resolve(7)),
      svarHvorfor: vi.fn(() =>
        Promise.resolve({ lessons: ["oyeblikk_fremfor_kvalitet"], choices: 1, answers: 1 }),
      ),
    },
  };
});

const tomtSammendrag: Sammendrag = {
  sources: 0,
  files: 0,
  cloud_only: 0,
  unreadable: 0,
  photos: 0,
  exact_duplicates: 0,
  near_duplicates: 0,
  uncertain_dates: 0,
  without_preview: 0,
};

const mock = api as unknown as { [K in keyof typeof api]: ReturnType<typeof vi.fn> };

function bilde(id: string, takenAt: string, extra: Partial<Bilde> = {}): Bilde {
  return {
    id,
    takenAt,
    dateSource: "exif",
    width: 4032,
    height: 3024,
    hasThumbnail: true,
    sources: ["icloud"],
    ...extra,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("første oppstart", () => {
  it("lager lagringen og viser gjenopprettingsnøkkelen før appen åpnes", async () => {
    mock.lagringStatus.mockResolvedValue("tom");
    mock.opprettLagring.mockResolvedValue("ABCD-EFGH-JKMN-PQRS-TVWX-YZ01-2345");
    render(<App />);

    await userEvent.click(await screen.findByRole("button", { name: tekster.velkommen.start }));
    expect(await screen.findByLabelText(tekster.gjenopprettingsnokkel.etikett)).toHaveTextContent(
      "ABCD-EFGH-JKMN-PQRS-TVWX-YZ01-2345",
    );

    const fortsett = screen.getByRole("button", { name: tekster.gjenopprettingsnokkel.fortsett });
    expect(fortsett).toBeDisabled();
    await userEvent.click(screen.getByLabelText(tekster.gjenopprettingsnokkel.bekreft));
    await userEvent.click(fortsett);

    expect(await screen.findByRole("heading", { name: tekster.start.tittel })).toBeInTheDocument();
    expect(screen.getByRole("note")).toHaveTextContent(tekster.lokalt.tittel);
  });
});

describe("ny maskin", () => {
  it("ber om gjenopprettingsnøkkelen og viser feil på norsk", async () => {
    mock.lagringStatus.mockResolvedValue("trenger_gjenoppretting");
    mock.gjenopprett
      .mockRejectedValueOnce({ kode: "ugyldig_kode", melding: "kontrollsum" })
      .mockResolvedValueOnce(undefined);
    render(<App />);

    const felt = await screen.findByLabelText(tekster.gjenopprett.etikett);
    await userEvent.type(felt, "abcd-efgh");
    await userEvent.click(screen.getByRole("button", { name: tekster.gjenopprett.knapp }));
    expect(await screen.findByRole("alert")).toHaveTextContent(tekster.feil.ugyldig_kode ?? "");

    await userEvent.click(screen.getByRole("button", { name: tekster.gjenopprett.knapp }));
    expect(await screen.findByRole("heading", { name: tekster.start.tittel })).toBeInTheDocument();
    expect(mock.gjenopprett).toHaveBeenLastCalledWith("abcd-efgh");
  });
});

describe("startskjermen", () => {
  it("leter bare etter vanlige mapper når brukeren ber om det", async () => {
    mock.lagringStatus.mockResolvedValue("klar");
    mock.forslag.mockResolvedValue([
      { kind: "dropbox", path: "/Users/kari/Dropbox/Camera Uploads", label: "Camera Uploads" },
    ]);
    render(<App />);

    await screen.findByRole("heading", { name: tekster.start.tittel });
    expect(mock.forslag).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: tekster.start.forslagKnapp }));
    expect(await screen.findByText("Camera Uploads")).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: tekster.start.leggTil }));
    expect(mock.leggTilKilde).toHaveBeenCalledWith("/Users/kari/Dropbox/Camera Uploads", "dropbox");
    await waitFor(() => expect(mock.startInnlesing).toHaveBeenCalled());
  });
});

describe("velg bilder", () => {
  it("viser bildene per måned med merknader, og starter innlesing ved oppstart", async () => {
    mock.lagringStatus.mockResolvedValue("laast");
    mock.kilder.mockResolvedValue([
      { id: 1, kind: "icloud", path: "/Bilder/iCloud", label: "iCloud Photos", finnes: true },
    ]);
    const aar: Aar[] = [
      { year: 2010, count: 1 },
      { year: 2011, count: 3 },
    ];
    mock.aar.mockResolvedValue(aar);
    mock.sammendrag.mockResolvedValue({
      ...tomtSammendrag,
      photos: 4,
      exact_duplicates: 2,
      cloud_only: 12,
      without_preview: 1,
    });
    mock.bilderIAar.mockResolvedValue([
      bilde("a".repeat(64), "2011-03-02T08:15:00"),
      bilde("b".repeat(64), "2011-03-05T10:00:00", { hasThumbnail: false }),
      bilde("c".repeat(64), "2011-07-14T12:34:56", { dateSource: "endringstid" }),
    ]);
    render(<App />);

    // Med kilder, men uten utkast: startsiden står, med kildene og «Lag utkast».
    expect(
      await screen.findByRole("heading", { level: 1, name: tekster.start.tittel }),
    ).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: tekster.utkast.lag })).toBeInTheDocument();
    expect(mock.aapneLagring).toHaveBeenCalled();
    await waitFor(() => expect(mock.startInnlesing).toHaveBeenCalled());

    // Albumutkastet er hovedvisningen; alle bildene ligger under «Alle bilder».
    await userEvent.click(screen.getByRole("button", { name: tekster.nav.alleBilder }));

    const mars = screen.getByRole("region", { name: "Mars 2011" });
    expect(within(mars).getAllByRole("listitem")).toHaveLength(2);
    expect(within(mars).getByText(tekster.bilder.ingenMiniatyr)).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Juli 2011" })).toHaveTextContent(
      tekster.bilder.usikkerDato,
    );
    expect(screen.getByRole("img", { name: "Bilde fra 2. mars 2011" })).toHaveAttribute(
      "src",
      `miniatyr://localhost/${"a".repeat(64)}`,
    );

    const merknader = screen.getByRole("list", { name: "Merknader" });
    expect(merknader).toHaveTextContent(/2\s?dubletter/);
    expect(merknader).toHaveTextContent(/12\s?bilder ligger bare i skyen/);
    expect(merknader).toHaveTextContent("HEIF Image Extensions");

    await userEvent.click(screen.getByRole("button", { name: /2010/ }));
    expect(mock.bilderIAar).toHaveBeenLastCalledWith(2010);
  });
});

describe("albumutkast", () => {
  const h = (c: string) => c.repeat(64);
  const utkast: Utkast = {
    year: 2011,
    mergeSuggestions: [],
    pages: 312,
    fullPages: 312,
    pageCap: null,
    cover: {
      front: null,
      back: null,
      chosenFront: false,
      chosenBack: false,
      people: [],
      overview: [],
    },
    events: [
      {
        start: "2011-06-05T11:00:00",
        end: "2011-06-05T15:00:00",
        photos: 3,
        included: 1,
        pages: 1,
        everyday: false,
        looksLikeTrip: false,
        adultTrip: false,
        adultTripGuess: false,
        key: h("a"),
        merged: false,
        pageTarget: null,
        tripAnswered: false,
        layout: [{ kind: "luft", photos: [h("a")] }],
      },
    ],
    photos: [
      {
        id: h("a"),
        takenAt: "2011-06-05T11:00:00",
        event: 0,
        included: true,
        reason: { kode: "beste_i_serie", antall: 4 },
        related: null,
        blurry: false,
        hasThumbnail: true,
        width: 4032,
        height: 3024,
        rotation: 0,
      },
      {
        id: h("b"),
        takenAt: "2011-06-05T11:00:05",
        event: 0,
        included: false,
        reason: { kode: "samme_serie", antall: 4 },
        related: h("a"),
        blurry: true,
        hasThumbnail: true,
        width: 4032,
        height: 3024,
        rotation: 0,
      },
      {
        id: h("c"),
        takenAt: "2011-06-05T15:00:00",
        event: 0,
        included: false,
        reason: { kode: "ikke_plass", antall: 1 },
        related: h("a"),
        blurry: true,
        hasThumbnail: true,
        width: 4032,
        height: 3024,
        rotation: 0,
      },
    ],
    learned: { lessons: [], choices: 0, answers: 0 },
  };

  beforeEach(() => {
    mock.lagringStatus.mockResolvedValue("klar");
    mock.kilder.mockResolvedValue([
      { id: 1, kind: "icloud", path: "/x", label: "Bilder", finnes: true },
    ]);
    mock.aar.mockResolvedValue([{ year: 2011, count: 3 } satisfies Aar]);
    mock.lagUtkast.mockResolvedValue(utkast);
  });

  it("lager et komplett forslag med begrunnelse for det som er med og ikke med", async () => {
    render(<App />);
    // Før utkastet: kildene og muligheten til å legge til flere, på samme side.
    expect(await screen.findByText(tekster.utkast.flereKilder)).toBeInTheDocument();
    expect(screen.getByRole("list", { name: tekster.utkast.kilderLagtTil })).toHaveTextContent(
      "Bilder",
    );
    await userEvent.click(await screen.findByRole("button", { name: tekster.utkast.lag }));
    expect(mock.lagUtkast).toHaveBeenCalledWith(2011, null);

    expect(
      await screen.findByRole("heading", { name: tekster.utkast.utkastTittel(2011) }),
    ).toBeInTheDocument();
    expect(screen.getByText("Beste bilde i en serie på 4")).toBeInTheDocument();
    expect(screen.getByText("Et bedre bilde fra samme øyeblikk er med")).toBeInTheDocument();
    expect(screen.getAllByText(tekster.utkast.uskarpt)).toHaveLength(2);
    expect(screen.getByText(/Albumet er nå på 312 sider og koster 1\s700 kr/)).toBeInTheDocument();

    // Færre eller flere sider etter gjennomkjøringen: utkastet lages på nytt.
    await userEvent.click(screen.getByRole("button", { name: tekster.pris.ned(300, 1500) }));
    expect(mock.lagUtkast).toHaveBeenLastCalledWith(2011, 300);
    await userEvent.click(screen.getByRole("button", { name: tekster.pris.opp(400, 1900) }));
    expect(mock.lagUtkast).toHaveBeenLastCalledWith(2011, 400);
  });

  it("bytter bilder og spør forsiktig hvorfor", async () => {
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: tekster.utkast.lag }));
    const ut = await screen.findByRole("button", { name: /med\. Beste bilde i en serie/ });
    const inn = screen.getByRole("button", { name: /ikke med\. Ikke plass/ });

    await userEvent.click(ut);
    await userEvent.click(inn);
    expect(mock.velgBilde).toHaveBeenCalledWith(2011, "bytt", h("c"), h("a"));
    expect(await screen.findByText(tekster.hvorfor.bytt)).toBeInTheDocument();

    await userEvent.click(
      screen.getByRole("button", { name: tekster.hvorfor.svar.viktig_oyeblikk }),
    );
    expect(mock.svarHvorfor).toHaveBeenCalledWith(7, "viktig_oyeblikk");
    expect(await screen.findByText(tekster.hvorfor.takk)).toBeInTheDocument();
    expect(screen.getAllByText("Du valgte dette").length).toBe(1);
  });
});

describe("spoerOmHvorfor", () => {
  it("spør de første gangene, så sjeldnere, og slutter når brukeren hopper over", () => {
    expect([1, 2, 3, 4, 5, 6].map((n) => spoerOmHvorfor(n, 0))).toEqual([
      true,
      true,
      true,
      false,
      false,
      true,
    ]);
    expect(spoerOmHvorfor(1, 3)).toBe(false);
  });
});

describe("slett alle data", () => {
  it("krever bekreftelse og går tilbake til velkomstskjermen", async () => {
    mock.lagringStatus.mockResolvedValue("klar");
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: tekster.nav.slettAlleData }));
    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveTextContent(tekster.slett.tekst);
    expect(mock.slettAlleData).not.toHaveBeenCalled();

    await userEvent.click(within(dialog).getByRole("button", { name: tekster.slett.bekreft }));
    expect(mock.slettAlleData).toHaveBeenCalled();
    expect(
      await screen.findByRole("heading", { name: tekster.velkommen.tittel }),
    ).toBeInTheDocument();
  });
});

describe("standardAar", () => {
  const naa = new Date("2026-10-04");
  it("velger forrige kalenderår hvis det har bilder", () => {
    expect(
      standardAar(
        [
          { year: 2024, count: 900 },
          { year: 2025, count: 10 },
        ],
        naa,
      ),
    ).toBe(2025);
  });
  it("ellers året med flest bilder", () => {
    expect(
      standardAar(
        [
          { year: 2010, count: 50 },
          { year: 2011, count: 80 },
        ],
        naa,
      ),
    ).toBe(2011);
    expect(standardAar([], naa)).toBeNull();
  });
});

describe("tekster", () => {
  it("bøyer entall og flertall", () => {
    expect(tekster.bilder.antall(1)).toBe("1 bilde");
    expect(tekster.merknader.usikreDatoer(1)).toMatch(/^1 bilde mangler/);
    expect(tekster.merknader.dubletter(1)).toMatch(/^1 dublett fra/);
    expect(tekster.innlesing.ferdig(1)).toBe("Fant 1 nytt bilde");
    expect(tekster.bilder.antall(8412)).toMatch(/^8\s412 bilder$/);
  });

  const alle = (o: unknown): string[] =>
    typeof o === "string"
      ? [o]
      : typeof o === "object" && o !== null
        ? Object.values(o as Record<string, unknown>).flatMap(alle)
        : [];

  it("sier aldri «last opp» om bildene (CLAUDE.md)", () => {
    for (const t of alle(tekster)) expect(t.toLowerCase()).not.toMatch(/last(e|er)? opp|opplast/);
  });

  it("bruker setningsstor bokstav", () => {
    // Merkenavn som skrives med liten forbokstav, er unntatt.
    const merkenavn = /^(iCloud|iPhone|iPad)\b/;
    for (const t of alle(tekster).filter((t) => !merkenavn.test(t))) {
      expect(t.charAt(0)).toBe(t.charAt(0).toUpperCase());
    }
  });
});
