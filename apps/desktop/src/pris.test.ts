import { pris, prisTrinn, prisvalg } from "./pris";

describe("pris i trinn", () => {
  it("er grunnpris pluss pris per side, regnet fra neste trinn", () => {
    expect(prisTrinn(312)).toBe(350);
    expect(pris(350)).toBe(300 + 1400);
    expect(pris(300)).toBe(300 + 1200);
  });

  it("gir valg både ned og opp rundt dagens sidetall", () => {
    expect(prisvalg(76)).toEqual({
      ned: [{ sider: 50, pris: 500 }],
      opp: [
        { sider: 100, pris: 700 },
        { sider: 150, pris: 900 },
      ],
    });
    expect(prisvalg(300).ned.map((v) => v.sider)).toEqual([200, 250]);
    expect(prisvalg(300).opp.map((v) => v.sider)).toEqual([350, 400]);
  });
});
