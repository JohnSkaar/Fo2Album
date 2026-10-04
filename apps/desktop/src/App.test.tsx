import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { tekster } from "./tekster";

describe("App", () => {
  it("viser startskjermen på norsk", () => {
    render(<App />);
    expect(
      screen.getByRole("heading", { level: 1, name: tekster.start.tittel }),
    ).toBeInTheDocument();
  });

  it("viser låsmerknaden der bilder velges (DESIGN.md)", () => {
    render(<App />);
    expect(screen.getByRole("note")).toHaveTextContent(tekster.lokalt.tittel);
  });

  it("viser alle fire bildekilder", () => {
    render(<App />);
    const liste = screen.getByRole("list", { name: tekster.start.kilderEtikett });
    const knapper = within(liste).getAllByRole("button");
    expect(knapper.map((k) => k.textContent)).toEqual([
      expect.stringContaining("Mappe på maskinen"),
      expect.stringContaining("Dropbox"),
      expect.stringContaining("iCloud Bilder"),
      expect.stringContaining("Google Disk"),
    ]);
  });

  it("gir beskjed når en kilde velges", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: /Dropbox/ }));
    expect(screen.getByRole("status")).toHaveTextContent(tekster.melding.kommerSnart);
  });
});

describe("tekster", () => {
  const alle = (o: unknown): string[] =>
    typeof o === "string" ? [o] : Object.values(o as Record<string, unknown>).flatMap(alle);

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
