//! Hvor bildene står på en trykt side, i millimeter fra øverste venstre hjørne av den
//! ferdige (beskårne) siden. Samme regler som utkastet og prototypen: hele motivet vises
//! (ingen beskjæring), unntatt en helside der bildets form ligger nær sidens.

use p2a_core::layout::PageKind;

/// Ferdig side: 21 × 28 cm (innbundet album).
pub const TRIM_W: f32 = 210.0;
pub const TRIM_H: f32 = 280.0;
/// Utfallende kant rundt hele siden (trykkeriet skjærer den bort).
pub const BLEED: f32 = 3.0;
/// Marg til tekst og bilder som ikke skal ut i kanten.
pub const MARGIN: f32 = 16.0;
/// Luft mellom bildene i et rutenett.
pub const GAP: f32 = 5.0;

/// Et felt for ett bilde. `fill` betyr at bildet fyller feltet og beskjæres litt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub fill: bool,
}

#[derive(Debug, Clone, Copy)]
struct Box_ {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

/// Størst mulig rektangel med forholdet `a` (bredde/høyde) inne i boksen, sentrert.
fn fit_in(b: Box_, a: f32) -> Frame {
    let (mut w, mut h) = (b.w, b.w / a);
    if h > b.h {
        h = b.h;
        w = h * a;
    }
    Frame {
        x: b.x + (b.w - w) / 2.0,
        y: b.y + (b.h - h) / 2.0,
        w,
        h,
        fill: false,
    }
}

/// Bildene i rader med lik høyde per rad, hele bildene synlige. Prøver 1–6 rader og velger
/// oppsettet som gir størst samlet bildeflate.
/// Et kandidatoppsett: samlet flate, radene (bildeindekser), radhøyder og total høyde.
type Rows = (f32, Vec<Vec<usize>>, Vec<f32>, f32);

fn justify(aspects: &[f32], b: Box_) -> Vec<Frame> {
    let n = aspects.len();
    let total: f32 = aspects.iter().sum();
    let mut best: Option<Rows> = None;
    for r in 1..=n.min(6) {
        let mut rows: Vec<Vec<usize>> = Vec::new();
        let mut cur = Vec::new();
        let mut acc = 0.0;
        for (i, a) in aspects.iter().enumerate() {
            cur.push(i);
            acc += a;
            let left = n - i - 1;
            let rows_left = r - rows.len() - 1;
            if rows_left > 0
                && (acc >= total * (rows.len() + 1) as f32 / r as f32 || left == rows_left)
            {
                rows.push(std::mem::take(&mut cur));
            }
        }
        if !cur.is_empty() {
            rows.push(cur);
        }
        let hs: Vec<f32> = rows
            .iter()
            .map(|row| {
                (b.w - GAP * (row.len() - 1) as f32) / row.iter().map(|&i| aspects[i]).sum::<f32>()
            })
            .collect();
        let th = hs.iter().sum::<f32>() + GAP * (rows.len() - 1) as f32;
        let s = (b.h / th).min(1.0);
        let area: f32 = rows
            .iter()
            .zip(&hs)
            .map(|(row, h)| {
                row.iter()
                    .map(|&i| aspects[i] * (h * s).powi(2))
                    .sum::<f32>()
            })
            .sum();
        if best.as_ref().is_none_or(|(a, ..)| area > *a) {
            best = Some((area, rows, hs, th * s));
        }
    }
    let Some((_, rows, hs, th)) = best else {
        return Vec::new();
    };
    let s = th / (hs.iter().sum::<f32>() + GAP * (rows.len() - 1) as f32);
    let mut out = vec![
        Frame {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            fill: false
        };
        n
    ];
    let mut y = b.y + (b.h - th) / 2.0;
    for (row, h) in rows.iter().zip(&hs) {
        let h = h * s;
        let rw =
            row.iter().map(|&i| aspects[i] * h).sum::<f32>() + GAP * s * (row.len() - 1) as f32;
        let mut x = b.x + (b.w - rw) / 2.0;
        for &i in row {
            out[i] = Frame {
                x,
                y,
                w: aspects[i] * h,
                h,
                fill: false,
            };
            x += aspects[i] * h + GAP * s;
        }
        y += h + GAP * s;
    }
    out
}

/// Feltene på en side med disse bildeformene (bredde/høyde, etter rotering).
pub fn frames(kind: PageKind, aspects: &[f32]) -> Vec<Frame> {
    let inner = Box_ {
        x: MARGIN,
        y: MARGIN,
        w: TRIM_W - 2.0 * MARGIN,
        h: TRIM_H - 2.0 * MARGIN,
    };
    match (kind, aspects) {
        (_, []) => Vec::new(),
        (PageKind::Helside, [a]) => {
            let page = TRIM_W / TRIM_H;
            if (a - page).abs() / page < 0.2 {
                // Nesten samme form som siden: fyll hele siden ut i kanten.
                vec![Frame {
                    x: -BLEED,
                    y: -BLEED,
                    w: TRIM_W + 2.0 * BLEED,
                    h: TRIM_H + 2.0 * BLEED,
                    fill: true,
                }]
            } else if *a > page {
                // Bredere enn siden: hele bredden ut i kantene, hele bildet synlig.
                let w = TRIM_W + 2.0 * BLEED;
                let h = w / a;
                vec![Frame {
                    x: -BLEED,
                    y: (TRIM_H - h) / 2.0,
                    w,
                    h,
                    fill: false,
                }]
            } else {
                // Smalere enn siden: hele høyden ut i kantene.
                let h = TRIM_H + 2.0 * BLEED;
                let w = h * a;
                vec![Frame {
                    x: (TRIM_W - w) / 2.0,
                    y: -BLEED,
                    w,
                    h,
                    fill: false,
                }]
            }
        }
        (PageKind::Luft, [a]) => vec![fit_in(
            Box_ {
                x: TRIM_W * 0.14,
                y: TRIM_H * 0.16,
                w: TRIM_W * 0.72,
                h: TRIM_H * 0.68,
            },
            *a,
        )],
        _ => justify(aspects, inner),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inside(f: &Frame) -> bool {
        f.x >= MARGIN - 0.01
            && f.y >= MARGIN - 0.01
            && f.x + f.w <= TRIM_W - MARGIN + 0.01
            && f.y + f.h <= TRIM_H - MARGIN + 0.01
    }

    #[test]
    fn rutenett_holder_seg_innenfor_margen_og_viser_hele_bildene() {
        let a = [1.5, 0.75, 1.33, 1.0, 1.5, 0.66];
        let fs = frames(PageKind::Rutenett { kolonner: 3 }, &a);
        assert_eq!(fs.len(), 6);
        for (f, a) in fs.iter().zip(a) {
            assert!(inside(f), "{f:?}");
            assert!(!f.fill);
            assert!((f.w / f.h - a).abs() < 0.01, "formen beholdes");
        }
        // Ingen overlapp.
        for i in 0..fs.len() {
            for j in i + 1..fs.len() {
                let (p, q) = (fs[i], fs[j]);
                let apart = p.x + p.w <= q.x + 0.01
                    || q.x + q.w <= p.x + 0.01
                    || p.y + p.h <= q.y + 0.01
                    || q.y + q.h <= p.y + 0.01;
                assert!(apart, "{p:?} {q:?}");
            }
        }
    }

    #[test]
    fn helside_fyller_ut_i_kanten() {
        let f = frames(PageKind::Helside, &[0.75])[0];
        assert!(f.fill && f.x < 0.0 && f.w > TRIM_W);
        let f = frames(PageKind::Helside, &[1.5])[0];
        assert!(!f.fill && f.x < 0.0 && (f.w / f.h - 1.5).abs() < 0.01);
    }

    #[test]
    fn luft_er_sentrert() {
        let f = frames(PageKind::Luft, &[1.5])[0];
        assert!((f.x + f.w / 2.0 - TRIM_W / 2.0).abs() < 0.01);
        assert!((f.y + f.h / 2.0 - TRIM_H / 2.0).abs() < 0.01);
    }
}
