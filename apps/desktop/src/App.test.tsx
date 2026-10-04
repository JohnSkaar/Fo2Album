import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Aar, Bilde, Sammendrag } from "./api";
import { api } from "./api";
import { App, standardAar } from "./App";
import { tekster } from "./tekster";

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

    expect(
      await screen.findByRole("heading", { level: 1, name: "Familiealbum 2011" }),
    ).toBeInTheDocument();
    expect(mock.aapneLagring).toHaveBeenCalled();
    await waitFor(() => expect(mock.startInnlesing).toHaveBeenCalled());

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
