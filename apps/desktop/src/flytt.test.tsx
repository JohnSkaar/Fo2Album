import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Albumtekst, Side, Utkast, UtkastBilde } from "./api";
import { Bla } from "./components/Bla";
import { DraftScreen, flytt } from "./components/DraftScreen";
import { tekster } from "./tekster";

const r = tekster.redigering;
const h = (c: string) => c.repeat(64);

const bilde = (id: string, included = true): UtkastBilde => ({
  id: h(id),
  takenAt: "2011-08-12T12:00:00",
  event: 0,
  included,
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
});

const felt = (n: number) =>
  Array.from({ length: n }, (_, i) => ({ x: 10, y: 10 + i * 30, w: 80, h: 25, fill: false }));
const s1: Side = { kind: "rutenett", kolonner: 2, photos: [h("a"), h("b")], frames: felt(2) };
const s2: Side = { kind: "luft", photos: [h("c")], frames: felt(1), mal: "1-kvadrat" };

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
  events: [
    {
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
      ownStory: false,
      layout: [s1, s2],
    },
  ],
  photos: [bilde("a"), bilde("b"), bilde("c"), bilde("d", false)],
  learned: { lessons: [], choices: 0, answers: 0 },
  checks: [],
};

function vis(extra: Partial<Parameters<typeof DraftScreen>[0]> = {}) {
  const onChange = vi.fn();
  render(
    <DraftScreen
      year={2011}
      draft={utkast}
      phase={null}
      onMake={() => {}}
      onPrint={() => {}}
      onChange={onChange}
      onSize={() => {}}
      onChoose={() => Promise.resolve(null)}
      onAnswer={() => {}}
      thumbUrl={(id) => `miniatyr://${id}`}
      {...extra}
    />,
  );
  return onChange;
}

describe("flytte bilder mellom sider", () => {
  it("flytter til en annen side og merker begge sidene fornøyd", () => {
    expect(flytt(s1, s2, h("b"), null)).toEqual([
      {
        type: "fornoyd",
        page: { kind: "rutenett", kolonner: 2, photos: [h("c"), h("b")], mal: null },
        on: true,
      },
      {
        type: "fornoyd",
        page: { kind: "luft", kolonner: null, photos: [h("a")], mal: null },
        on: true,
      },
    ]);
  });

  it("bytter rekkefølge på samme side og beholder rammene", () => {
    const [c] = flytt(s1, s1, h("b"), h("a"));
    expect(c).toMatchObject({ page: { photos: [h("b"), h("a")] } });
    expect(flytt(s1, s1, h("a"), h("a"))).toEqual([]);
    expect(flytt(s1, s1, h("a"), h("b"))).toEqual([]);
  });

  it("et bilde som ikke er med, kan dras inn på en side", () => {
    expect(flytt(undefined, s2, h("d"), h("c"))).toEqual([
      {
        type: "fornoyd",
        page: { kind: "rutenett", kolonner: 2, photos: [h("d"), h("c")], mal: null },
        on: true,
      },
    ]);
  });

  it("dra og slipp i utkastet", () => {
    const onChange = vis();
    const fra = within(screen.getByRole("group", { name: /Side 1/ })).getAllByRole("button")[1]!;
    fireEvent.dragStart(fra);
    fireEvent.drop(screen.getByRole("group", { name: /Side 2/ }));
    expect(onChange).toHaveBeenCalledWith(flytt(s1, s2, h("b"), null), r.flyttet);
  });

  it("neste side fra bildemenyen", async () => {
    const onChange = vis();
    await userEvent.click(
      within(screen.getByRole("group", { name: /Side 1/ })).getAllByRole("button")[0]!,
    );
    expect(screen.queryByRole("button", { name: r.forrigeSide })).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: r.nesteSide }));
    expect(onChange).toHaveBeenCalledWith(flytt(s1, s2, h("a"), h("c")), r.flyttet);
  });
});

describe("angre", () => {
  it("knappen og Ctrl+Z angrer", async () => {
    const onUndo = vi.fn();
    vis({ canUndo: true, onUndo });
    await userEvent.click(screen.getByRole("button", { name: `↶ ${r.angre}` }));
    await userEvent.keyboard("{Control>}z{/Control}");
    expect(onUndo).toHaveBeenCalledTimes(2);
  });

  it("knappen er av når det ikke er noe å angre", () => {
    vis({ canUndo: false, onUndo: () => {} });
    expect(screen.getByRole("button", { name: `↶ ${r.angre}` })).toBeDisabled();
  });

  it("markerte bilder tas med samlet", async () => {
    const onChooseMany = vi.fn();
    vis({ onChooseMany });
    for (const b of screen.getAllByRole("button", { name: /^Marker: / })) await userEvent.click(b);
    await userEvent.click(screen.getByRole("button", { name: tekster.utkast.taBort }));
    expect(onChooseMany).toHaveBeenCalledWith("ta_bort", [h("a"), h("b"), h("c")]);
  });
});

describe("tekst rett i albumet", () => {
  const tekst: Albumtekst = {
    title: "Øyeblikk fra 2011",
    subtitle: "Kari og Ola",
    intro: "",
    back: "",
  };

  it("skriver teksten om året på første side", async () => {
    const onSaveText = vi.fn();
    render(
      <Bla
        draft={utkast}
        tekst={tekst}
        thumbUrl={(x) => x}
        onClose={() => {}}
        onSaveText={onSaveText}
      />,
    );
    expect(screen.getByText("Kari og Ola")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: `${r.neste} →` }));
    await userEvent.click(screen.getByRole("button", { name: `✎ ${r.endreTekst}` }));
    await userEvent.type(screen.getByLabelText(tekster.trykk.intro), "Emma begynte på skolen.");
    await userEvent.click(screen.getByRole("button", { name: r.lagreTekst }));
    expect(onSaveText).toHaveBeenCalledWith({ ...tekst, intro: "Emma begynte på skolen." });
  });
});
