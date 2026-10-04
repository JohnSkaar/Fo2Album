import { pris, prisTrinn, prisvalg } from "./pris";

describe("pris i trinn", () => {
  it("regner fra neste trinn på 50 sider", () => {
    expect(prisTrinn(350)).toBe(350);
    expect(prisTrinn(312)).toBe(350);
    expect(pris(350)).toBe(1400);
    expect(pris(300)).toBe(1200);
    expect(pris(10)).toBe(200);
  });

  it("viser hele historien og trinnene under", () => {
    expect(prisvalg(350)).toEqual([
      { sidetak: null, sider: 350, pris: 1400 },
      { sidetak: 300, sider: 300, pris: 1200 },
      { sidetak: 250, sider: 250, pris: 1000 },
      { sidetak: 200, sider: 200, pris: 800 },
    ]);
    expect(prisvalg(60).map((v) => v.sider)).toEqual([60, 50]);
  });
});
