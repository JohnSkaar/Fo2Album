//! Prototypens enkle kvalitetsmål (prototype/fo2album-prototype.html, `analyze`), oversatt
//! til Rust så M3 har et ærlig utgangspunkt å slå på gullsettet. Erstattes av Q_tech
//! (SCORING.md §3.1) i M3.
//!
//! Skarphet måles på motivet: Laplace-varians i 4 × 4 ruter på en kopi som er 360 px bred,
//! og den nest skarpeste ruten teller. Uskarp bakgrunn trekker da ikke ned, men et bilde der
//! ingenting er skarpt, gjør det. Hudtoner (andel piksler) er en grov stedfortreder for
//! personer til ansiktsgjenkjenningen kommer (M4).

use image::imageops::FilterType;
use image::DynamicImage;
use p2a_core::BasicQuality;

/// Bildet skaleres til denne bredden før målingen av farger og lys, som i prototypen.
const WIDTH: u32 = 160;
/// Bredden skarpheten måles på.
const SHARP_WIDTH: u32 = 360;
const TILES: usize = 4;

pub fn basic_quality(image: &DynamicImage) -> BasicQuality {
    let h = ((image.height() as u64 * WIDTH as u64) / image.width().max(1) as u64).max(1) as u32;
    let small = image.resize_exact(WIDTH, h, FilterType::Triangle).to_rgb8();
    let (w, h) = (small.width() as usize, small.height() as usize);
    let n = (w * h) as f64;

    let (mut sum, mut clip, mut skin) = (0.0, 0.0, 0.0);
    let (mut rg_s, mut yb_s, mut rg_s2, mut yb_s2) = (0.0, 0.0, 0.0, 0.0);
    for p in small.pixels() {
        let (r, g, b) = (p[0] as f64, p[1] as f64, p[2] as f64);
        let y = 0.299 * r + 0.587 * g + 0.114 * b;
        sum += y;
        let cb = 128.0 - 0.168736 * r - 0.331264 * g + 0.5 * b;
        let cr = 128.0 + 0.5 * r - 0.418688 * g - 0.081312 * b;
        if (77.0..127.0).contains(&cb) && (133.0..173.0).contains(&cr) && y > 40.0 {
            skin += 1.0;
        }
        if !(12.0..=245.0).contains(&y) {
            clip += 1.0;
        }
        let rg = r - g;
        let yb = 0.5 * (r + g) - b;
        rg_s += rg;
        yb_s += yb;
        rg_s2 += rg * rg;
        yb_s2 += yb * yb;
    }

    let lap_var = subject_sharpness(image);

    let mean = sum / n;
    let (rg_m, yb_m) = (rg_s / n, yb_s / n);
    let colorful = ((rg_s2 / n - rg_m * rg_m) + (yb_s2 / n - yb_m * yb_m))
        .max(0.0)
        .sqrt()
        + 0.3 * (rg_m * rg_m + yb_m * yb_m).sqrt();

    BasicQuality {
        sharp: (lap_var.max(0.0).ln_1p() / 3000f64.ln_1p()).min(1.0) as f32,
        exposure: ((1.0 - (mean - 118.0).abs() / 118.0).max(0.0)
            * (1.0 - (clip / n * 3.0).min(1.0))) as f32,
        color: (colorful / 90.0).min(1.0) as f32,
        skin: (skin / n) as f32,
    }
}

/// Laplace-varians i den nest skarpeste av 4 × 4 ruter.
fn subject_sharpness(image: &DynamicImage) -> f64 {
    let sh =
        ((image.height() as u64 * SHARP_WIDTH as u64) / image.width().max(1) as u64).max(3) as u32;
    let img = image
        .resize_exact(SHARP_WIDTH, sh, FilterType::Triangle)
        .to_luma8();
    let (w, h) = (img.width() as usize, img.height() as usize);
    let g: Vec<f64> = img.pixels().map(|p| p[0] as f64).collect();
    let mut tiles = Vec::with_capacity(TILES * TILES);
    for ty in 0..TILES {
        for tx in 0..TILES {
            let (y0, y1) = ((ty * h / TILES).max(1), ((ty + 1) * h / TILES).min(h - 1));
            let (x0, x1) = ((tx * w / TILES).max(1), ((tx + 1) * w / TILES).min(w - 1));
            let (mut s1, mut s2, mut m) = (0.0, 0.0, 0.0);
            for y in y0..y1 {
                for x in x0..x1 {
                    let k = y * w + x;
                    let v = 4.0 * g[k] - g[k - 1] - g[k + 1] - g[k - w] - g[k + w];
                    s1 += v;
                    s2 += v * v;
                    m += 1.0;
                }
            }
            tiles.push(if m > 0.0 {
                s2 / m - (s1 / m).powi(2)
            } else {
                0.0
            });
        }
    }
    tiles.sort_by(|a, b| b.total_cmp(a));
    tiles.get(1).copied().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth;

    fn q(img: image::RgbImage) -> BasicQuality {
        basic_quality(&DynamicImage::ImageRgb8(img))
    }

    #[test]
    fn blur_lowers_sharpness() {
        // Sjakkbrett med ruter på 20 px: tydelige kanter også etter nedskalering til 160 px.
        let sharp = image::RgbImage::from_fn(800, 600, |x, y| {
            if (x / 20 + y / 20) % 2 == 0 {
                image::Rgb([230, 200, 160])
            } else {
                image::Rgb([40, 60, 90])
            }
        });
        let blurred = image::imageops::blur(&sharp, 6.0);
        let (a, b) = (q(sharp), q(blurred));
        assert!(a.sharp > b.sharp + 0.2, "skarp {a:?}, uskarp {b:?}");
    }

    #[test]
    fn dark_and_flat_images_score_low() {
        let dark = image::RgbImage::from_pixel(400, 300, image::Rgb([4, 4, 4]));
        let gray = image::RgbImage::from_pixel(400, 300, image::Rgb([128, 128, 128]));
        let good = q(synth::pattern(800, 600, 5));
        let dark = q(dark);
        assert!(dark.exposure < 0.1);
        assert!(q(gray).color < 0.05);
        assert!(good.score() > dark.score());
        for v in [good.sharp, good.exposure, good.color, good.skin] {
            assert!((0.0..=1.0).contains(&v));
        }
    }

    #[test]
    fn sharp_subject_on_blurry_background_counts_as_sharp() {
        // Uskarp bakgrunn med et skarpt motiv i midten (som et portrett med bokeh).
        let sharp = image::RgbImage::from_fn(800, 600, |x, y| {
            if (x / 8 + y / 8) % 2 == 0 {
                image::Rgb([230, 200, 160])
            } else {
                image::Rgb([40, 60, 90])
            }
        });
        let mut portrait = image::imageops::blur(&sharp, 8.0);
        image::imageops::replace(
            &mut portrait,
            &image::imageops::crop_imm(&sharp, 300, 200, 240, 220).to_image(),
            300,
            200,
        );
        let all_blurry = image::imageops::blur(&sharp, 8.0);
        assert!(q(portrait).sharp > q(all_blurry).sharp + 0.2);
    }

    #[test]
    fn skin_tones_are_counted() {
        let skin = image::RgbImage::from_pixel(400, 300, image::Rgb([224, 172, 140]));
        let sky = image::RgbImage::from_pixel(400, 300, image::Rgb([90, 140, 220]));
        assert!(q(skin).skin > 0.9);
        assert!(q(sky).skin < 0.01);
    }
}
