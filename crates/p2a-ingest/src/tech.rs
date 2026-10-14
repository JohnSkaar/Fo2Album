//! Tekniske kvalitetsmål utover skarphet og lys (SCORING.md §3.1): bevegelsesuskarphet,
//! støy og fargestikk. Alle 0–1, der 0 er bra. Deterministiske og uten modeller.
//!
//! - **Bevegelse:** kamerarystelse demper kantene i én retning. Kantenergien måles i fire
//!   retninger i 4 × 4 ruter; i ruter med tekstur sammenlignes vinkelrette retninger, og
//!   medianen over rutene avgjør. Jevn uskarphet (fokus) demper alle retninger
//!   likt og gir lav verdi her; den fanges av skarpheten.
//! - **Støy:** Immerkærs raske estimat (Laplace-lignende maske) i de flateste rutene, så
//!   kanter og tekstur ikke regnes som støy.
//! - **Fargestikk:** de minst mettede pikslene bør være grå; har de alle samme fargetone,
//!   er hvitbalansen feil.

use image::imageops::FilterType;
use image::{DynamicImage, GrayImage};
use p2a_core::TechQuality;

/// Bredden bevegelse og fargestikk måles på.
const WIDTH: u32 = 400;
const TILES: usize = 4;

pub fn tech_quality(image: &DynamicImage) -> TechQuality {
    let h = ((image.height() as u64 * WIDTH as u64) / image.width().max(1) as u64).max(8) as u32;
    let small = image.resize_exact(WIDTH, h, FilterType::Triangle);
    TechQuality {
        motion: motion(&small.to_luma8()),
        noise: noise(&image.to_luma8()),
        cast: cast(&small),
    }
}

/// Forholdet mellom svakeste og sterkeste kantretning (median over ruter med tekstur).
pub fn direction_ratio(g: &GrayImage) -> Option<f64> {
    let (w, h) = (g.width() as usize, g.height() as usize);
    let px = |x: usize, y: usize| g.as_raw()[y * w + x] as f64;
    let mut ratios = Vec::new();
    for ty in 0..TILES {
        for tx in 0..TILES {
            let (y0, y1) = ((ty * h / TILES).max(1), ((ty + 1) * h / TILES).min(h - 1));
            let (x0, x1) = ((tx * w / TILES).max(1), ((tx + 1) * w / TILES).min(w - 1));
            let mut e = [0.0f64; 4];
            let mut n = 0.0;
            for y in y0..y1 {
                for x in x0..x1 {
                    let c = px(x, y);
                    e[0] += (px(x + 1, y) - c).powi(2);
                    e[1] += (px(x, y + 1) - c).powi(2);
                    e[2] += (px(x + 1, y + 1) - c).powi(2);
                    e[3] += (px(x + 1, y - 1) - c).powi(2);
                    n += 1.0;
                }
            }
            if n == 0.0 {
                continue;
            }
            let e = e.map(|v| v / n);
            // Bare ruter med tydelig tekstur sier noe om retning.
            if e.iter().cloned().fold(0.0, f64::max) < TEXTURE_MIN {
                continue;
            }
            // Bevegelse demper én retning, mens retningen vinkelrett på den er urørt.
            // Sammenlign derfor vinkelrette par med samme steglengde (vannrett/loddrett,
            // og de to diagonalene); det svakeste paret avgjør.
            let pair = |a: f64, b: f64| a.min(b) / a.max(b).max(1e-9);
            ratios.push(pair(e[0], e[1]).min(pair(e[2], e[3])));
        }
    }
    if ratios.len() < 3 {
        return None;
    }
    ratios.sort_by(f64::total_cmp);
    Some(ratios[ratios.len() / 2])
}

/// Minste kantenergi (snitt av kvadrert differanse) for at en rute regnes som teksturert.
const TEXTURE_MIN: f64 = 12.0;
/// Retningsforhold under `MOTION_LO` regnes som tydelig bevegelse, over `MOTION_HI` som ingen.
const MOTION_LO: f64 = 0.20;
const MOTION_HI: f64 = 0.55;

fn motion(g: &GrayImage) -> f32 {
    match direction_ratio(g) {
        Some(r) => ((MOTION_HI - r) / (MOTION_HI - MOTION_LO)).clamp(0.0, 1.0) as f32,
        None => 0.0,
    }
}

/// Støyens standardavvik (0–255-skala) i de flateste rutene.
pub fn noise_sigma(g: &GrayImage) -> f64 {
    let (w, h) = (g.width() as usize, g.height() as usize);
    if w < 8 || h < 8 {
        return 0.0;
    }
    let px = |x: usize, y: usize| g.as_raw()[y * w + x] as f64;
    const T: usize = 8;
    let mut sigmas = Vec::with_capacity(T * T);
    for ty in 0..T {
        for tx in 0..T {
            let (y0, y1) = ((ty * h / T).max(1), ((ty + 1) * h / T).min(h - 1));
            let (x0, x1) = ((tx * w / T).max(1), ((tx + 1) * w / T).min(w - 1));
            let (mut s, mut n) = (0.0, 0.0);
            for y in y0..y1 {
                for x in x0..x1 {
                    let v = px(x - 1, y - 1) - 2.0 * px(x, y - 1) + px(x + 1, y - 1)
                        - 2.0 * px(x - 1, y)
                        + 4.0 * px(x, y)
                        - 2.0 * px(x + 1, y)
                        + px(x - 1, y + 1)
                        - 2.0 * px(x, y + 1)
                        + px(x + 1, y + 1);
                    s += v.abs();
                    n += 1.0;
                }
            }
            if n > 0.0 {
                sigmas.push((std::f64::consts::PI / 2.0).sqrt() * s / (6.0 * n));
            }
        }
    }
    sigmas.sort_by(f64::total_cmp);
    // Den flateste fjerdedelen: der er det støy, ikke motiv, som gir utslag.
    let k = (sigmas.len() / 4).max(1);
    sigmas[..k].iter().sum::<f64>() / k as f64
}

/// σ under `NOISE_LO` er rent, over `NOISE_HI` svært støyete.
const NOISE_LO: f64 = 2.0;
const NOISE_HI: f64 = 9.0;

fn noise(g: &GrayImage) -> f32 {
    ((noise_sigma(g) - NOISE_LO) / (NOISE_HI - NOISE_LO)).clamp(0.0, 1.0) as f32
}

/// Avstanden fra nøytral (i YCbCr) for snittet av de minst mettede pikslene. Med riktig
/// hvitbalanse er de grå; med fargestikk har de alle samme fargetone.
pub fn cast_distance(img: &DynamicImage) -> Option<f64> {
    let rgb = img.to_rgb8();
    let mut px: Vec<(u8, f64, f64)> = rgb
        .pixels()
        .filter_map(|p| {
            let (r, g, b) = (p[0] as f64, p[1] as f64, p[2] as f64);
            let y = 0.299 * r + 0.587 * g + 0.114 * b;
            // Middels lyse piksler; svart og utbrent sier ingenting om fargen.
            (40.0..225.0).contains(&y).then(|| {
                let spread = p.0.iter().max().unwrap() - p.0.iter().min().unwrap();
                (
                    spread,
                    -0.168736 * r - 0.331264 * g + 0.5 * b,
                    0.5 * r - 0.418688 * g - 0.081312 * b,
                )
            })
        })
        .collect();
    if px.len() < (rgb.width() * rgb.height()) as usize / 10 {
        return None;
    }
    px.sort_by_key(|p| p.0);
    let k = (px.len() * 3 / 10).max(1);
    let (cb, cr) = px[..k]
        .iter()
        .fold((0.0, 0.0), |(a, b), p| (a + p.1, b + p.2));
    Some(((cb / k as f64).powi(2) + (cr / k as f64).powi(2)).sqrt())
}

/// Avstand under `CAST_LO` er nøytral, over `CAST_HI` tydelig stikk.
const CAST_LO: f64 = 5.0;
const CAST_HI: f64 = 16.0;

fn cast(img: &DynamicImage) -> f32 {
    match cast_distance(img) {
        Some(d) => ((d - CAST_LO) / (CAST_HI - CAST_LO)).clamp(0.0, 1.0) as f32,
        None => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth;
    use image::RgbImage;

    fn t(img: RgbImage) -> TechQuality {
        tech_quality(&DynamicImage::ImageRgb8(img))
    }

    fn scene(seed: u32) -> RgbImage {
        synth::textured(1000, 750, seed)
    }

    /// Mest dempede farger, som vegger, himmel og klær i et vanlig bilde.
    fn muted(seed: u32) -> RgbImage {
        let s = scene(seed);
        RgbImage::from_fn(1000, 750, |x, y| {
            let p = s.get_pixel(x, y);
            let g = (p[0] as u32 + p[1] as u32 + p[2] as u32) / 3;
            image::Rgb(p.0.map(|c| ((c as u32 + 3 * g) / 4) as u8))
        })
    }

    #[test]
    fn sharp_images_have_no_technical_faults() {
        for seed in 1..4 {
            let q = t(scene(seed));
            assert_eq!((q.motion, q.noise), (0.0, 0.0), "frø {seed}: {q:?}");
            assert_eq!(t(muted(seed)).cast, 0.0);
        }
    }

    #[test]
    fn motion_blur_is_found_in_any_direction() {
        for seed in 1..4 {
            let s = scene(seed);
            let across = t(synth::motion_blur(&s, 25, 1, 0)).motion;
            let diagonal = t(synth::motion_blur(&s, 25, 1, 1)).motion;
            assert!(across > 0.5, "frø {seed}: vannrett {across}");
            assert!(diagonal > 0.25, "frø {seed}: skrå {diagonal}");
        }
    }

    #[test]
    fn focus_blur_and_slight_shake_are_not_motion() {
        for seed in 1..4 {
            let s = scene(seed);
            // Uskarphet i alle retninger fanges av skarpheten, ikke her.
            assert_eq!(t(image::imageops::blur(&s, 4.0)).motion, 0.0);
            assert_eq!(t(synth::motion_blur(&s, 7, 0, 1)).motion, 0.0);
        }
    }

    #[test]
    fn noise_follows_the_amount_of_noise() {
        let s = scene(1);
        let sigma = |img: &RgbImage| noise_sigma(&DynamicImage::ImageRgb8(img.clone()).to_luma8());
        // Estimatet treffer innlagt støy godt, og kanter regnes ikke som støy.
        assert!(sigma(&s) < 1.0);
        let six = sigma(&synth::add_noise(&s, 6.0, 9));
        assert!((5.0..7.5).contains(&six), "σ = {six}");
        let some = t(synth::add_noise(&s, 6.0, 9)).noise;
        let much = t(synth::add_noise(&s, 15.0, 9)).noise;
        assert!((0.3..0.8).contains(&some), "{some}");
        assert_eq!(much, 1.0);
    }

    #[test]
    fn colour_cast_is_noticed() {
        for seed in 1..3 {
            let m = muted(seed);
            let warm = RgbImage::from_fn(1000, 750, |x, y| {
                let p = m.get_pixel(x, y);
                image::Rgb([
                    p[0].saturating_add(25),
                    p[1].saturating_add(5),
                    p[2].saturating_sub(25),
                ])
            });
            assert!(t(warm).cast > 0.3, "frø {seed}");
        }
    }

    #[test]
    fn q_tech_follows_scoring() {
        let q = p2a_core::BasicQuality {
            sharp: 1.0,
            exposure: 1.0,
            color: 0.5,
            skin: 0.0,
        };
        let perfect = TechQuality::default();
        assert!((perfect.q_tech(&q) - 1.0).abs() < 1e-6);
        let shaky = TechQuality {
            motion: 1.0,
            ..perfect
        };
        assert!((shaky.q_tech(&q) - 0.85).abs() < 1e-6);
    }
}
