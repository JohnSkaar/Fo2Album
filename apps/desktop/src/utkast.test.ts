import type { Side } from "./api";
import { datoSpenn, oppdaterSider } from "./utkast";

describe("oppdaterSider", () => {
  const sider: Side[] = [
    { kind: "helside", photos: ["a"] },
    { kind: "rutenett", kolonner: 2, photos: ["b", "c"] },
  ];

  it("lar et bilde som er byttet inn ta plassen til det som gikk ut", () => {
    expect(oppdaterSider(sider, ["x", "b", "c"])).toEqual({
      sider: [
        { kind: "helside", photos: ["x"] },
        { kind: "rutenett", kolonner: 2, photos: ["b", "c"] },
      ],
      endret: true,
    });
  });

  it("legger nye bilder på en side til slutt, og er uendret uten valg", () => {
    expect(oppdaterSider(sider, ["a", "b", "c"]).endret).toBe(false);
    const r = oppdaterSider(sider, ["a", "b", "c", "d"]);
    expect(r.sider.at(-1)).toEqual({ kind: "rutenett", kolonner: 2, photos: ["d"] });
  });
});

describe("datoSpenn", () => {
  it("skriver datoer slik nordmenn gjør", () => {
    expect(datoSpenn("2011-07-14T10:00:00", "2011-07-14T18:00:00")).toBe("14. juli");
    expect(datoSpenn("2011-07-14T10:00:00", "2011-07-16T18:00:00")).toBe("14.–16. juli");
    expect(datoSpenn("2011-06-30T10:00:00", "2011-07-02T18:00:00")).toBe("30. juni–2. juli");
  });
});
