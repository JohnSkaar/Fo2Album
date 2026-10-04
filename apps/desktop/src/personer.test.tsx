import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Personer } from "./api";
import { PeopleScreen } from "./components/PeopleScreen";
import { tekster } from "./tekster";

const t = tekster.personer;
const ansikt = (faceId: number) => ({
  faceId,
  id: "ab".repeat(32),
  x: 0.3,
  y: 0.2,
  w: 0.2,
  h: 0.25,
});

const personer: Personer = {
  pending: 12,
  persons: [{ id: 7, name: "Ella", role: "barn" }],
  groups: [
    {
      group: 1000000,
      personId: null,
      name: null,
      role: null,
      faces: 14,
      photos: 12,
      samples: [ansikt(1), ansikt(2)],
    },
    {
      group: 7,
      personId: 7,
      name: "Ella",
      role: "barn",
      faces: 40,
      photos: 33,
      samples: [ansikt(3), ansikt(4)],
    },
  ],
};

function vis() {
  const props = { onName: vi.fn(), onIgnore: vi.fn(), onMove: vi.fn() };
  render(<PeopleScreen people={personer} thumbUrl={(id) => `miniatyr://${id}`} {...props} />);
  return props;
}

describe("hvem er med?", () => {
  it("viser ukjente og navngitte personer", () => {
    vis();
    expect(screen.getByRole("heading", { name: t.tittel, level: 1 })).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent(t.venter(12));
    expect(screen.getByRole("heading", { name: t.ukjent })).toBeInTheDocument();
    expect(screen.getByText(t.antall(14, 12))).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /Ella/ })).toBeInTheDocument();
  });

  it("gir en gruppe navn og rolle", async () => {
    const { onName } = vis();
    const gruppe = screen.getByRole("heading", { name: t.ukjent }).closest("li")!;
    const g = within(gruppe as HTMLElement);
    await userEvent.type(g.getByLabelText(t.navn), "Jonas");
    await userEvent.selectOptions(g.getByLabelText(t.rolle), "barn");
    await userEvent.click(g.getByRole("button", { name: t.lagre }));
    expect(onName).toHaveBeenCalledWith(personer.groups[0], "Jonas", "barn", null);
  });

  it("kan si at gruppen ikke er viktig, eller er en person som finnes", async () => {
    const { onIgnore, onName } = vis();
    const g = within(screen.getByRole("heading", { name: t.ukjent }).closest("li") as HTMLElement);
    await userEvent.selectOptions(g.getByLabelText(t.finnes), "7");
    await userEvent.click(g.getByRole("button", { name: t.lagre }));
    expect(onName).toHaveBeenCalledWith(personer.groups[0], "", "kjernefamilie", 7);
    await userEvent.click(g.getByRole("button", { name: t.ikkeViktig }));
    expect(onIgnore).toHaveBeenCalledWith(personer.groups[0]);
  });

  it("tar ett ansikt ut av en navngitt person", async () => {
    const { onMove } = vis();
    await userEvent.click(screen.getAllByRole("button", { name: t.ikkeSamme("Ella") })[0]!);
    expect(onMove).toHaveBeenCalledWith(3);
  });
});
