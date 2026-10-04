/**
 * Pris på albumet, regnet ut automatisk. Foreløpig: 4 kr per side, i trinn på 50 sider, så
 * brukeren ser hva et mindre album koster. Endelige priser kommer fra trykkeriet (M7).
 */
export const KR_PER_SIDE = 4;
export const SIDETRINN = 50;
export const MIN_SIDER = 50;

/** Sidetallet prisen regnes fra: neste trinn opp. 312 sider → 350. */
export function prisTrinn(sider: number): number {
  return Math.max(MIN_SIDER, Math.ceil(sider / SIDETRINN) * SIDETRINN);
}

export function pris(sider: number): number {
  return prisTrinn(sider) * KR_PER_SIDE;
}

export interface Prisvalg {
  /** Sidetaket som gir dette valget; `null` = hele historien. */
  sidetak: number | null;
  sider: number;
  pris: number;
}

/** Hele historien, og de tre trinnene under den. */
export function prisvalg(heleSider: number): Prisvalg[] {
  const valg: Prisvalg[] = [{ sidetak: null, sider: heleSider, pris: pris(heleSider) }];
  for (
    let t = prisTrinn(heleSider) - SIDETRINN;
    t >= MIN_SIDER && valg.length < 4;
    t -= SIDETRINN
  ) {
    valg.push({ sidetak: t, sider: t, pris: pris(t) });
  }
  return valg;
}
