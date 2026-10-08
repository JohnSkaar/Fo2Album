import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Albumtekst } from "./api";
import { PrintDialog } from "./components/PrintDialog";
import { tekster } from "./tekster";

const t = tekster.trykk;
const tekst: Albumtekst = {
  title: "Øyeblikk fra 2011",
  subtitle: "Kari og Ola",
  intro: "",
  back: "",
};

describe("trykkfil", () => {
  it("viser forslaget, og sender tekstene brukeren skriver", async () => {
    const onMake = vi.fn();
    render(
      <PrintDialog
        open
        year={2011}
        text={tekst}
        status={{ type: "skriver" }}
        onMake={onMake}
        onClose={() => {}}
      />,
    );
    expect(screen.getByLabelText(t.tittelFelt)).toHaveValue("Øyeblikk fra 2011");
    expect(screen.getByText(t.trygt)).toBeInTheDocument();
    await userEvent.type(screen.getByLabelText(t.intro), "Emma begynte på skolen.");
    await userEvent.type(screen.getByLabelText(t.bakside), "Takk for et fint år!");
    await userEvent.click(screen.getByRole("button", { name: t.lag }));
    expect(onMake).toHaveBeenCalledWith({
      ...tekst,
      intro: "Emma begynte på skolen.",
      back: "Takk for et fint år!",
    });
  });

  it("viser fremdrift og stenger knappene mens filen lages", () => {
    render(
      <PrintDialog
        open
        year={2011}
        text={tekst}
        status={{ type: "lager", p: { done: 16, total: 49 } }}
        onMake={() => {}}
        onClose={() => {}}
      />,
    );
    expect(screen.getByRole("status")).toHaveTextContent(t.lager(16, 49));
    expect(screen.getByRole("button", { name: t.lag })).toBeDisabled();
  });

  it("sier fra om bilder som mangler eller har lav oppløsning", () => {
    render(
      <PrintDialog
        open
        year={2011}
        text={tekst}
        status={{
          type: "ferdig",
          fil: { pages: 24, photos: 49, missing: 2, lowResolution: 3, megabytes: 14.6 },
        }}
        onMake={() => {}}
        onClose={() => {}}
      />,
    );
    expect(screen.getByText(t.ferdig(24, 49, 14.6))).toBeInTheDocument();
    expect(screen.getByText(t.mangler(2))).toBeInTheDocument();
    expect(screen.getByText(t.lavOppl(3))).toBeInTheDocument();
  });
});
