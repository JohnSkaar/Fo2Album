import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Ramme, Side, Utkast, UtkastBilde, UtkastHendelse } from "./api";
import { oppslag } from "./components/Bla";
import { DraftScreen, nyStorrelse } from "./components/DraftScreen";
import { bildeStil, utsnittFoerRotering } from "./components/Side";
import { nyttUtsnitt } from "./components/StorVisning";
import { tekster } from "./tekster";

const r = tekster.redigering;
const h = (c: string) => c.repeat(64);

const bilde = (id: string, extra: Partial<UtkastBilde> = {}): UtkastBilde => ({
  id: h(id),
  takenAt: "2011-08-12T12:00:00",
  event: 0,
  included: true,
  reason: { kode: "god_kvalitet" },
  related: null,
  blurry: false,
  hasThumbnail: true,
  width: 1200,
  height: 800,
  rotation: 0,
  size: null,
  focus: [0.5, 0.5],
  whole: false,
  ...extra,
});

const rutenett: Side = {
  kind: "rutenett",
  kolonner: 2,
  photos: [h("a"), h("b")],
  frames: [
    { x: 10, y: 10, w: 80, h: 30, fill: false },
    { x: 10, y: 50, w: 80, h: 30, fill: false },
  ],
};
const helside: Side = {
  kind: "helside",
  photos: [h("c")],
  frames: [{ x: -1, y: -1, w: 102, h: 102, fill: true }],
};

const hendelse: UtkastHendelse = {
  key: h("a"),
  merged: false,
  pageTarget: null,
  tripAnswered: false,
  start: "2011-08-12T10:00:00",
  end: "2011-08-12T18:00:00",
  photos: 4,
  included: 3,
  pages: 2,
  everyday: false,
  looksLikeTrip: false,
  adultTrip: false,
  adultTripGuess: false,
  ownStory: true,
  layout: [rutenett, helside],
};

const utkast: Utkast = {
  year: 2011,
  mergeSuggestions: [],
  pages: 2,
  fullPages: 2,
  pageCap: null,
  cover: {
    front: h("a"),
    back: h("c"),
    chosenFront: false,
    chosenBack: false,
    people: [],
    overview: [],
  },
  events: [hendelse],
  photos: [bilde("a"), bilde("b"), bilde("c"), bilde("d", { included: false })],
  learned: { lessons: [], choices: 0, answers: 0 },
};

const rammer: Ramme[] = [
  { id: "2-over", photos: 2, frames: [] },
  { id: "1-full", photos: 1, frames: [] },
];

function vis(onChoose = vi.fn(() => Promise.resolve(null))) {
  const onChange = vi.fn();
  render(
    <DraftScreen
      year={2011}
      draft={utkast}
      phase={null}
      rammer={rammer}
      onMake={() => {}}
      onPrint={() => {}}
      onChange={onChange}
      onSize={() => {}}
      onChoose={onChoose}
      onAnswer={() => {}}
      thumbUrl={(id) => `miniatyr://${id}`}
    />,
  );
  return { onChange, onChoose };
}

describe("størrelse", () => {
  it("går trinnvis til egen side og hele siden, og tilbake", () => {
    expect(nyStorrelse(null, rutenett, 1)).toBe(1);
    expect(nyStorrelse(2, rutenett, 1)).toBe(3);
    expect(nyStorrelse(3, helside, 1)).toBe(4);
    expect(nyStorrelse(4, helside, -1)).toBe(3);
    expect(nyStorrelse(3, { ...helside, kind: "luft" }, -1)).toBe(0);
    expect(nyStorrelse(0, rutenett, -1)).toBe(-1);
    expect(nyStorrelse(-1, rutenett, -1)).toBe(-1);
  });

  it("dokken gjør bildet større og sier hvor stort det er", async () => {
    const { onChange } = vis();
    const side = screen.getByRole("group", { name: /Side 1/ });
    await userEvent.click(within(side).getAllByRole("button")[0]!);
    expect(screen.getByText(r.storrelse(null, false, false))).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: r.storre }));
    expect(onChange).toHaveBeenCalledWith(
      { type: "storrelse", photo: h("a"), size: 1 },
      r.storrelse(1, false, false),
    );
  });

  it("et bilde som tar hele siden kan ikke bli større", async () => {
    vis();
    const side = screen.getByRole("group", { name: /Side 2/ });
    await userEvent.click(within(side).getByRole("button"));
    expect(screen.getByText(r.storrelse(null, true, true))).toBeInTheDocument();
    expect(screen.getByRole("button", { name: r.storre })).toBeDisabled();
    expect(screen.getByRole("button", { name: r.utsnitt })).toBeInTheDocument();
  });
});

describe("rammer for siden", () => {
  it("viser rammer med like mange felt og lagrer valget som fornøyd side", async () => {
    const { onChange } = vis();
    await userEvent.click(screen.getByRole("button", { name: `Side 1: ${tekster.sider.rammer}` }));
    const valg = screen.getByRole("group", { name: tekster.sider.rammerTittel });
    expect(within(valg).queryByText(tekster.sider.mal["1-full"]!)).toBeNull();
    await userEvent.click(within(valg).getByRole("button", { name: /To over hverandre/ }));
    expect(onChange).toHaveBeenCalledWith(
      {
        type: "fornoyd",
        page: { kind: "rutenett", kolonner: 2, photos: [h("a"), h("b")], mal: "2-over" },
        on: true,
      },
      tekster.sider.malValgt,
    );
  });
});

describe("markering av flere bilder", () => {
  it("samler markerte bilder på én side eller i en egen historie", async () => {
    const { onChange } = vis();
    await userEvent.click(screen.getAllByRole("button", { name: /^Marker: / })[0]!);
    await userEvent.click(screen.getAllByRole("button", { name: /^Marker: / })[1]!);
    expect(screen.getByText(r.markert(2))).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: r.samleSide }));
    expect(onChange).toHaveBeenCalledWith(
      { type: "samle_side", photos: [h("a"), h("b")] },
      r.samlet(2),
    );
    expect(screen.queryByText(r.markert(2))).toBeNull();

    const user = userEvent.setup();
    await user.keyboard("{Control>}");
    await user.click(screen.getAllByRole("button", { name: /^Bilde fra/ })[2]!);
    await user.keyboard("{/Control}");
    await userEvent.click(screen.getByRole("button", { name: r.egenHistorie }));
    expect(onChange).toHaveBeenCalledWith(
      { type: "egen_historie", photos: [h("c")] },
      r.historieLaget(1),
    );
  });

  it("tar med og roterer mange på en gang", async () => {
    const { onChange, onChoose } = vis();
    for (const b of screen.getAllByRole("button", { name: /^Marker: / })) await userEvent.click(b);
    await userEvent.click(screen.getByRole("button", { name: `↻ ${tekster.utkast.roter}` }));
    expect(onChange).toHaveBeenCalledWith(
      ["a", "b", "c", "d"].map((x) => ({ type: "roter", photo: h(x) })),
      r.rotert(4),
    );
    for (const b of screen.getAllByRole("button", { name: /^Marker: / })) await userEvent.click(b);
    await userEvent.click(screen.getByRole("button", { name: tekster.utkast.taMed }));
    expect(onChoose).toHaveBeenCalledTimes(1);
    expect(onChoose).toHaveBeenCalledWith("ta_med", h("d"));
  });

  it("en egen historie kan legges tilbake", async () => {
    const { onChange } = vis();
    expect(screen.getByText(r.egenHistorieMerke)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: r.leggTilbake }));
    expect(onChange).toHaveBeenCalledWith({ type: "legg_tilbake", photo: h("a") }, r.lagtTilbake);
  });
});

describe("utsnitt og visning", () => {
  it("klikk i rammen flytter punktet mot midten", () => {
    // Bredt bilde i en kvadratisk ramme: halve bredden synes.
    expect(nyttUtsnitt(2, 1, [0.5, 0.5], [0.5, 0.5])).toEqual([0.5, 0.5]);
    expect(nyttUtsnitt(2, 1, [0.5, 0.5], [1, 0.2])).toEqual([1, 0.5]);
    expect(nyttUtsnitt(2, 1, [0.5, 0.5], [0.25, 0.5])[0]).toBeCloseTo(0.25);
    // Høyt bilde: bare høyden beskjæres.
    expect(nyttUtsnitt(0.5, 1, [0.5, 0.5], [0, 0])).toEqual([0.5, 0]);
  });

  it("utsnittet regnes om for roterte bilder", () => {
    expect(utsnittFoerRotering(0, [0.2, 0.7])).toEqual([0.2, 0.7]);
    expect(utsnittFoerRotering(90, [0.2, 0.7])).toEqual([0.7, 0.8]);
    expect(utsnittFoerRotering(270, [0.2, 0.7])[0]).toBeCloseTo(0.3);
    const stil = bildeStil({ rotation: 90, focus: [0.5, 0.5], whole: false }, true, 2);
    expect(stil.width).toBe("50.000%");
    expect(stil.height).toBe("200.000%");
  });

  it("oppslagene har forsiden og baksiden alene", () => {
    const o = oppslag(utkast);
    expect(o).toHaveLength(4);
    expect(o[0]![1]).toMatchObject({ type: "forside", id: h("a") });
    expect(o[1]![1]).toMatchObject({ type: "intro" });
    expect(o[2]!.map((b) => b?.type)).toEqual(["side", "side"]);
    expect(o[3]![0]).toMatchObject({ type: "bakside", id: h("c") });
  });

  it("blar i albumet med knappene", async () => {
    vis();
    await userEvent.click(screen.getByRole("button", { name: r.bla }));
    expect(screen.getByText(r.oppslag(1, 4))).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: `${r.neste} →` }));
    expect(screen.getByText(r.oppslag(2, 4))).toBeInTheDocument();
  });
});
