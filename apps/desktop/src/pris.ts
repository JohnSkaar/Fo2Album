/**
 * Pris på albumet, regnet ut automatisk. Foreløpig modell (plassholdere til prisene er avklart
 * med trykkeriet, M7): grunnpris for permen pluss en pris per side, regnet i trinn på 50
 * sider. Endres bare her.
 */
export const GRUNNPRIS = 300;
export const KR_PER_SIDE = 4;
export const SIDETRINN = 50;

/** Sidetallet prisen regnes fra: neste trinn opp. 312 sider → 350. */
export function prisTrinn(sider: number): number {
  return Math.max(SIDETRINN, Math.ceil(sider / SIDETRINN) * SIDETRINN);
}

export function pris(sider: number): number {
  return GRUNNPRIS + prisTrinn(sider) * KR_PER_SIDE;
}

export interface Prisvalg {
  sider: number;
  pris: number;
}

/**
 * Valgene for å justere albumet etter gjennomkjøringen, både ned og opp: de to trinnene under
 * og de to over dagens sidetall (grensene der prisen endres).
 */
export function prisvalg(sider: number): { ned: Prisvalg[]; opp: Prisvalg[] } {
  const under = Math.ceil(sider / SIDETRINN) * SIDETRINN - SIDETRINN;
  const over = Math.floor(sider / SIDETRINN) * SIDETRINN + SIDETRINN;
  const v = (s: number) => ({ sider: s, pris: pris(s) });
  return {
    ned: [under - SIDETRINN, under].filter((s) => s >= SIDETRINN && s !== sider).map(v),
    opp: [over, over + SIDETRINN].map(v),
  };
}
