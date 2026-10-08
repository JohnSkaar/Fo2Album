//! Hvor bildene står på en trykt side, i millimeter fra øverste venstre hjørne av den
//! ferdige (beskårne) siden. Samme regler som prototypen: automatisk viser hele motivet
//! (ingen beskjæring), unntatt en helside der bildets form ligger nær sidens. Brukeren kan
//! gjøre et bilde større eller mindre enn de andre, og velge faste rammer for siden; da
//! fyller bildene rammene med et utsnitt brukeren kan flytte.
//!
//! Appen viser sidene med de samme feltene (`page_frames`), så det brukeren ser, er det som
//! trykkes.

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

/// Hvordan brukeren vil ha et bilde på siden.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// -1 mindre enn de andre, 0 like stort, 1–2 større (3 og 4 gir egen side i utkastet).
    pub size: i8,
    /// Punktet (andel av bredde og høyde) som skal være i midten når bildet fyller en ramme.
    pub focus: (f32, f32),
    /// Vis hele bildet i rammen, uten beskjæring.
    pub whole: bool,
}

impl Default for Look {
    fn default() -> Self {
        Look {
            size: 0,
            focus: (0.5, 0.5),
            whole: false,
        }
    }
}

/// Faste rammer brukeren kan velge for en side, med antall bilder. Samme utvalg som
/// prototypen; navnene står i grensesnittet (tekster.ts).
pub const TEMPLATES: &[(&str, usize)] = &[
    ("1-full", 1),
    ("1-kvadrat", 1),
    ("1-landskap", 1),
    ("1-portrett", 1),
    ("2-over", 2),
    ("2-side", 2),
    ("3-topp", 3),
    ("3-venstre", 3),
    ("3-rader", 3),
    ("4-kvadrat", 4),
    ("4-landskap", 4),
    ("6", 6),
    ("9", 9),
    ("12", 12),
];

fn inner() -> Box_ {
    Box_ {
        x: MARGIN,
        y: MARGIN,
        w: TRIM_W - 2.0 * MARGIN,
        h: TRIM_H - 2.0 * MARGIN,
    }
}

/// `cols × rows` like felt, med forholdet `ca` (bredde/høyde) om det er gitt, midtstilt.
fn grid(cols: usize, rows: usize, ca: Option<f32>) -> Vec<Box_> {
    let b = inner();
    let mut cw = (b.w - GAP * (cols - 1) as f32) / cols as f32;
    let mut ch = (b.h - GAP * (rows - 1) as f32) / rows as f32;
    if let Some(ca) = ca {
        if cw / ch > ca {
            cw = ch * ca;
        } else {
            ch = cw / ca;
        }
    }
    let tw = cols as f32 * cw + GAP * (cols - 1) as f32;
    let th = rows as f32 * ch + GAP * (rows - 1) as f32;
    let (x0, y0) = ((TRIM_W - tw) / 2.0, (TRIM_H - th) / 2.0);
    (0..cols * rows)
        .map(|i| Box_ {
            x: x0 + (i % cols) as f32 * (cw + GAP),
            y: y0 + (i / cols) as f32 * (ch + GAP),
            w: cw,
            h: ch,
        })
        .collect()
}

/// Feltene i en fast ramme.
fn template(id: &str) -> Option<Vec<Box_>> {
    let b = inner();
    Some(match id {
        "1-full" => vec![Box_ {
            x: -BLEED,
            y: -BLEED,
            w: TRIM_W + 2.0 * BLEED,
            h: TRIM_H + 2.0 * BLEED,
        }],
        "1-kvadrat" => grid(1, 1, Some(1.0)),
        "1-landskap" => grid(1, 1, Some(4.0 / 3.0)),
        "1-portrett" => grid(1, 1, Some(3.0 / 4.0)),
        "2-over" => grid(1, 2, Some(4.0 / 3.0)),
        "2-side" => grid(2, 1, Some(2.0 / 3.0)),
        "3-topp" => {
            let top = b.h * 0.56;
            let w2 = (b.w - GAP) / 2.0;
            let (y2, h2) = (b.y + top + GAP, b.h - top - GAP);
            vec![
                Box_ { h: top, ..b },
                Box_ {
                    x: b.x,
                    y: y2,
                    w: w2,
                    h: h2,
                },
                Box_ {
                    x: b.x + w2 + GAP,
                    y: y2,
                    w: w2,
                    h: h2,
                },
            ]
        }
        "3-venstre" => {
            let w1 = b.w * 0.55;
            let w2 = b.w - w1 - GAP;
            let h2 = (b.h - GAP) / 2.0;
            vec![
                Box_ { w: w1, ..b },
                Box_ {
                    x: b.x + w1 + GAP,
                    y: b.y,
                    w: w2,
                    h: h2,
                },
                Box_ {
                    x: b.x + w1 + GAP,
                    y: b.y + h2 + GAP,
                    w: w2,
                    h: h2,
                },
            ]
        }
        "3-rader" => grid(1, 3, Some(4.0 / 3.0)),
        "4-kvadrat" => grid(2, 2, Some(1.0)),
        "4-landskap" => grid(2, 2, Some(4.0 / 3.0)),
        "6" => grid(2, 3, Some(1.0)),
        "9" => grid(3, 3, Some(1.0)),
        "12" => grid(3, 4, Some(1.0)),
        _ => return None,
    })
}

/// Feltene i en fast ramme, til forhåndsvisning av rammevalget (`None` for ukjent navn).
pub fn template_frames(id: &str) -> Option<Vec<Frame>> {
    Some(
        template(id)?
            .into_iter()
            .map(|b| Frame {
                x: b.x,
                y: b.y,
                w: b.w,
                h: b.h,
                fill: true,
            })
            .collect(),
    )
}

/// Ett bilde gjort større enn de andre: det får rundt 35 % (trinn 1) eller 55 % (trinn 2) av
/// flaten innenfor margen, til venstre om det er stående og øverst ellers. De andre deler
/// resten.
fn hero(aspects: &[f32], sizes: &[i8], b: Box_) -> Vec<Frame> {
    let top = sizes.iter().copied().max().unwrap_or(0);
    let h = sizes.iter().position(|&s| s == top).unwrap_or(0);
    let a = aspects[h];
    let target = b.w * b.h * if top >= 2 { 0.55 } else { 0.35 };
    let mut hw = (target * a).sqrt();
    let mut hh = hw / a;
    if hw > b.w {
        hw = b.w;
        hh = hw / a;
    }
    if hh > b.h * 0.75 {
        hh = b.h * 0.75;
        hw = hh * a;
    }
    let others: Vec<f32> = (0..aspects.len())
        .filter(|&i| i != h)
        .map(|i| aspects[i])
        .collect();
    let (big, rest) = if a < 1.0 {
        let w = hw;
        (
            Box_ { w, ..b },
            Box_ {
                x: b.x + w + GAP,
                w: b.w - w - GAP,
                ..b
            },
        )
    } else {
        (
            Box_ { h: hh, ..b },
            Box_ {
                y: b.y + hh + GAP,
                h: b.h - hh - GAP,
                ..b
            },
        )
    };
    let mut rr = justify(&others, rest).into_iter();
    (0..aspects.len())
        .map(|i| {
            if i == h {
                fit_in(big, a)
            } else {
                rr.next().expect("ett felt per bilde")
            }
        })
        .collect()
}

/// Feltene på en side, med brukerens størrelser, utsnitt og rammer. `photos` er formen
/// (bredde/høyde, etter rotering) og ønsket for hvert bilde, i sidens rekkefølge.
pub fn page_frames(kind: PageKind, mal: Option<&str>, photos: &[(f32, Look)]) -> Vec<Frame> {
    let aspects: Vec<f32> = photos.iter().map(|(a, _)| *a).collect();
    if let Some(rects) = mal.and_then(template) {
        if rects.len() == photos.len() {
            return rects
                .into_iter()
                .zip(photos)
                .map(|(r, (a, look))| {
                    if look.whole {
                        fit_in(r, *a)
                    } else {
                        Frame {
                            x: r.x,
                            y: r.y,
                            w: r.w,
                            h: r.h,
                            fill: true,
                        }
                    }
                })
                .collect();
        }
    }
    let sizes: Vec<i8> = photos.iter().map(|(_, l)| l.size.clamp(-1, 2)).collect();
    if photos.len() < 2 || sizes.iter().all(|&s| s == 0) {
        return frames(kind, &aspects);
    }
    let top = sizes.iter().copied().max().unwrap_or(0);
    let out = if top > 0 {
        hero(&aspects, &sizes, inner())
    } else {
        justify(&aspects, inner())
    };
    // Mindre enn de andre: 72 % av feltet, midtstilt.
    out.into_iter()
        .zip(&sizes)
        .map(|(f, &s)| {
            if s < 0 && s < top {
                let k = 0.72;
                Frame {
                    x: f.x + f.w * (1.0 - k) / 2.0,
                    y: f.y + f.h * (1.0 - k) / 2.0,
                    w: f.w * k,
                    h: f.h * k,
                    fill: false,
                }
            } else {
                f
            }
        })
        .collect()
}

/// Feltene på en side med disse bildeformene (bredde/høyde, etter rotering).
pub fn frames(kind: PageKind, aspects: &[f32]) -> Vec<Frame> {
    let inner = inner();
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

    fn look(size: i8) -> (f32, Look) {
        (
            1.5,
            Look {
                size,
                ..Look::default()
            },
        )
    }

    #[test]
    fn storre_bilde_faar_mer_plass_og_mindre_faar_mindre() {
        let kind = PageKind::Rutenett { kolonner: 2 };
        let area = |f: &Frame| f.w * f.h;
        let even = page_frames(kind, None, &[look(0), look(0), look(0), look(0)]);
        let big = page_frames(kind, None, &[look(0), look(1), look(0), look(0)]);
        let bigger = page_frames(kind, None, &[look(0), look(2), look(0), look(0)]);
        let small = page_frames(kind, None, &[look(-1), look(0), look(0), look(0)]);
        assert!(area(&big[1]) > area(&even[1]) * 1.3);
        assert!(area(&bigger[1]) > area(&big[1]));
        assert!(area(&big[1]) > area(&big[0]) * 2.0);
        assert!(area(&small[0]) < area(&small[1]) * 0.6);
        for f in big.iter().chain(&bigger).chain(&small) {
            assert!(inside(f), "{f:?}");
        }
    }

    #[test]
    fn faste_rammer_fyller_og_hele_bildet_beskjaeres_ikke() {
        for (id, n) in TEMPLATES {
            let fs = template_frames(id).unwrap();
            assert_eq!(fs.len(), *n, "{id}");
        }
        let mut photos = vec![look(0), look(0), look(0)];
        photos[2].1.whole = true;
        let fs = page_frames(PageKind::Luft, Some("3-topp"), &photos);
        assert!(fs[0].fill && fs[1].fill);
        assert!(!fs[2].fill && (fs[2].w / fs[2].h - 1.5).abs() < 0.01);
        // Feil antall bilder for rammen: automatisk.
        let fs = page_frames(
            PageKind::Rutenett { kolonner: 2 },
            Some("3-topp"),
            &photos[..2],
        );
        assert!(fs.iter().all(|f| !f.fill));
        assert_eq!(
            page_frames(PageKind::Helside, Some("1-full"), &photos[..1])[0].x,
            -BLEED
        );
    }

    #[test]
    fn luft_er_sentrert() {
        let f = frames(PageKind::Luft, &[1.5])[0];
        assert!((f.x + f.w / 2.0 - TRIM_W / 2.0).abs() < 0.01);
        assert!((f.y + f.h / 2.0 - TRIM_H / 2.0).abs() < 0.01);
    }
}
