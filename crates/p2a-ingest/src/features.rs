//! Prototypens enkle kvalitetsmål (prototype/pho2album-prototype.html, `analyze`), oversatt
//! til Rust så M3 har et ærlig utgangspunkt å slå på gullsettet. Erstattes av Q_tech
//! (SCORING.md §3.1) i M3.

use image::imageops::FilterType;
use image::DynamicImage;
use p2a_core::BasicQuality;

/// Bildet skaleres til denne bredden før målingen, som i prototypen.
const WIDTH: u32 = 160;

pub fn basic_quality(image: &DynamicImage) -> BasicQuality {
    let h = ((image.height() as u64 * WIDTH as u64) / image.width().max(1) as u64).max(1) as u32;
    let small = image.resize_exact(WIDTH, h, FilterType::Triangle).to_rgb8();
    let (w, h) = (small.width() as usize, small.height() as usize);
    let n = (w * h) as f64;

    let mut gray = vec![0.0f64; w * h];
    let (mut sum, mut clip) = (0.0, 0.0);
    let (mut rg_s, mut yb_s, mut rg_s2, mut yb_s2) = (0.0, 0.0, 0.0, 0.0);
    for (i, p) in small.pixels().enumerate() {
        let (r, g, b) = (p[0] as f64, p[1] as f64, p[2] as f64);
        let y = 0.299 * r + 0.587 * g + 0.114 * b;
        gray[i] = y;
        sum += y;
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

    // Varians av Laplace-filteret.
    let (mut lap, mut lap_s, mut m) = (0.0, 0.0, 0.0);
    for y in 1..h.saturating_sub(1) {
        for x in 1..w - 1 {
            let k = y * w + x;
            let v = 4.0 * gray[k] - gray[k - 1] - gray[k + 1] - gray[k - w] - gray[k + w];
            lap += v;
            lap_s += v * v;
            m += 1.0;
        }
    }
    let lap_var = if m > 0.0 {
        lap_s / m - (lap / m).powi(2)
    } else {
        0.0
    };

    let mean = sum / n;
    let (rg_m, yb_m) = (rg_s / n, yb_s / n);
    let colorful = ((rg_s2 / n - rg_m * rg_m) + (yb_s2 / n - yb_m * yb_m))
        .max(0.0)
        .sqrt()
        + 0.3 * (rg_m * rg_m + yb_m * yb_m).sqrt();

    BasicQuality {
        sharp: (lap_var.max(0.0).ln_1p() / 900f64.ln_1p()).min(1.0) as f32,
        exposure: ((1.0 - (mean - 118.0).abs() / 118.0).max(0.0)
            * (1.0 - (clip / n * 3.0).min(1.0))) as f32,
        color: (colorful / 90.0).min(1.0) as f32,
    }
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
        // Prototypens log-skala går nesten i metning (1,0 mot 0,96 her): uskarphet straffes
        // svakt. Det er en kjent svakhet som Q_tech i M3 skal rette; her sjekkes bare retningen.
        assert!(a.sharp > b.sharp, "skarp {a:?}, uskarp {b:?}");
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
        for v in [good.sharp, good.exposure, good.color] {
            assert!((0.0..=1.0).contains(&v));
        }
    }
}
