import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Utkast, UtkastBilde, UtkastHendelse } from "./api";
import { DraftScreen } from "./components/DraftScreen";
import { tekster } from "./tekster";

const t = tekster.utkast;
const h = (c: string) => c.repeat(64);

const bilde = (id: string, event: number, included: boolean): UtkastBilde => ({
  id: h(id),
  takenAt: "2011-08-12T12:00:00",
  event,
  included,
  reason: { kode: "god_kvalitet" },
  related: null,
  blurry: false,
  hasThumbnail: true,
  width: 1200,
  height: 900,
  rotation: id === "b" ? 90 : 0,
});

const hendelse = (
  key: string,
  start: string,
  end: string,
  extra: Partial<UtkastHendelse>,
): UtkastHendelse => ({
  key: h(key),
  merged: false,
  pageTarget: null,
  tripAnswered: false,
  start,
  end,
  photos: 2,
  included: 2,
  pages: 1,
  everyday: false,
  looksLikeTrip: false,
  adultTrip: false,
  adultTripGuess: false,
  layout: [{ kind: "rutenett", kolonner: 2, photos: [h(key)] }],
  ...extra,
});

const utkast: Utkast = {
  year: 2011,
  mergeSuggestions: [[1, 2]],
  pages: 3,
  fullPages: 3,
  pageCap: null,
  cover: {
    front: h("a"),
    back: null,
    chosenFront: false,
    chosenBack: false,
    people: [h("a")],
    overview: [h("c")],
  },
  events: [
    hendelse("a", "2011-08-12T10:00:00", "2011-08-14T18:00:00", { looksLikeTrip: true }),
    hendelse("c", "2011-04-22T10:00:00", "2011-04-22T12:00:00", {}),
    hendelse("e", "2011-04-23T10:00:00", "2011-04-23T12:00:00", {}),
  ],
  photos: [bilde("a", 0, true), bilde("b", 0, true), bilde("c", 1, true), bilde("e", 2, true)],
  learned: { lessons: [], choices: 0, answers: 0 },
};

function vis(u: Utkast = utkast) {
  const onChange = vi.fn();
  render(
    <DraftScreen
      year={2011}
      draft={u}
      phase={null}
      onMake={() => {}}
      onPrint={() => {}}
      onChange={onChange}
      onSize={() => {}}
      onChoose={() => Promise.resolve(null)}
      onAnswer={() => {}}
      thumbUrl={(id) => `miniatyr://${id}`}
    />,
  );
  return onChange;
}

describe("redigering i utkastet", () => {
  it("merker en side som fornøyd", async () => {
    const onChange = vis();
    await userEvent.click(screen.getAllByRole("button", { name: /Fornøyd/ })[0]!);
    expect(onChange).toHaveBeenCalledWith(
      { type: "fornoyd", page: { kind: "rutenett", kolonner: 2, photos: [h("a")] }, on: true },
      t.fornoydMelding,
    );
  });

  it("spør om turen var uten barn", async () => {
    const onChange = vis();
    await userEvent.click(screen.getByRole("button", { name: t.turJa }));
    expect(onChange).toHaveBeenCalledWith({ type: "tur_uten_barn", photo: h("a"), on: true });
  });

  it("presenterer en historie på flere sider", async () => {
    const onChange = vis();
    await userEvent.click(screen.getAllByRole("button", { name: t.flereSider })[0]!);
    await userEvent.click(screen.getByRole("button", { name: t.lagForslag }));
    expect(onChange).toHaveBeenCalledWith({ type: "sider", photo: h("a"), pages: 2 });
  });

  it("foreslår å slå sammen korte dager", async () => {
    const onChange = vis();
    expect(screen.getByRole("note")).toHaveTextContent("22.–23. april");
    await userEvent.click(screen.getByRole("button", { name: t.slaaSammen }));
    expect(onChange).toHaveBeenCalledWith({ type: "slaa_sammen", photos: [h("c"), h("e")] });
  });

  it("velger forside og bakside blant forslagene", async () => {
    const onChange = vis();
    const kandidat = screen.getAllByRole("button", { name: t.bakside })[0]!;
    await userEvent.click(kandidat);
    expect(onChange).toHaveBeenCalledWith({ type: "bakside", photo: h("a") });
  });

  it("roterer og bruker et bilde som forside fra bildemenyen", async () => {
    const onChange = vis();
    const ev = screen.getAllByRole("region")[0] ?? document.body;
    await userEvent.click(
      within(ev as HTMLElement).getAllByRole("button", { name: /Bilde fra/ })[0]!,
    );
    await userEvent.click(screen.getByRole("button", { name: `↻ ${t.roter}` }));
    expect(onChange).toHaveBeenCalledWith({ type: "roter", photo: h("a") });
    await userEvent.click(screen.getByRole("button", { name: t.brukForside }));
    expect(onChange).toHaveBeenCalledWith({ type: "forside", photo: h("a") });
  });
});
