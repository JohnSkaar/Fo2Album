import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { meldingstekst, Tilbakemelding } from "./components/Tilbakemelding";
import { tekster } from "./tekster";

const t = tekster.tilbakemelding;
const info = {
  version: "0.3.0",
  coreVersion: "0.3.0",
  os: "windows x86_64",
  feedbackEmail: "plussdelta@fo2album.no",
};

describe("gi oss tilbakemelding", () => {
  it("teksten har bare det kunden skrev, og versjon om kunden vil", () => {
    const m = meldingstekst("feil", "  Appen stoppet ved innlesing ", info, true);
    expect(m.emne).toBe(t.emne(t.slag.feil));
    expect(m.tekst).toBe(
      [
        "Appen stoppet ved innlesing",
        "",
        "—",
        "Gjelder: Noe fungerer ikke",
        "Appversjon: 0.3.0",
        "System: windows x86_64",
      ].join("\n"),
    );
    expect(meldingstekst("ros", "Fint", info, false).tekst).not.toMatch(/Appversjon|System/);
  });

  it("viser hva som sendes og sender det kunden ser", async () => {
    const onSend = vi.fn();
    render(<Tilbakemelding info={info} onSend={onSend} onClose={() => {}} />);
    const send = screen.getByRole("button", { name: t.send });
    expect(send).toBeDisabled();
    await userEvent.click(screen.getByLabelText(t.slag.feil));
    await userEvent.type(screen.getByLabelText(t.tekstEtikett), "Knappen virker ikke");
    await userEvent.click(screen.getByLabelText(t.medInfo));
    expect(screen.getByText(/Knappen virker ikke/, { selector: "pre" })).not.toHaveTextContent(
      "Appversjon",
    );
    await userEvent.click(send);
    expect(onSend).toHaveBeenCalledWith(
      t.emne(t.slag.feil),
      ["Knappen virker ikke", "", "—", "Gjelder: Noe fungerer ikke"].join("\n"),
    );
  });
});
