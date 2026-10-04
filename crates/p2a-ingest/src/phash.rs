//! Perseptuell hash (pHash, 64 bit) for å finne samme bilde i ulik oppløsning eller
//! format (SCORING.md §2, nivå 2).
//!
//! Bildet skaleres til 32 × 32 gråtoner, transformeres med DCT, og de 8 × 8 laveste
//! frekvensene sammenlignes med medianen. Like bilder får hasher med liten
//! Hamming-avstand, selv etter nedskalering og ny JPEG-komprimering.

use std::f64::consts::PI;
use std::sync::LazyLock;

use image::imageops::FilterType;
use image::DynamicImage;

const N: usize = 32;
const K: usize = 8;

/// Cosinus-tabell for DCT-II: COS[u][x] = cos((2x + 1) u π / 2N).
static COS: LazyLock<[[f64; N]; N]> = LazyLock::new(|| {
    let mut t = [[0.0; N]; N];
    for (u, row) in t.iter_mut().enumerate() {
        for (x, v) in row.iter_mut().enumerate() {
            *v = ((2 * x + 1) as f64 * u as f64 * PI / (2 * N) as f64).cos();
        }
    }
    t
});

pub fn phash(image: &DynamicImage) -> u64 {
    let small = image
        .resize_exact(N as u32, N as u32, FilterType::Triangle)
        .to_luma8();
    let px = |x: usize, y: usize| small.get_pixel(x as u32, y as u32)[0] as f64;

    // Separabel 2D-DCT, bare de K × K laveste frekvensene.
    let mut rows = [[0.0; K]; N]; // DCT langs x for hver rad
    for (y, row) in rows.iter_mut().enumerate() {
        for (u, out) in row.iter_mut().enumerate() {
            *out = (0..N).map(|x| px(x, y) * COS[u][x]).sum();
        }
    }
    let mut coeffs = [0.0; K * K];
    for v in 0..K {
        for u in 0..K {
            coeffs[v * K + u] = (0..N).map(|y| rows[y][u] * COS[v][y]).sum();
        }
    }

    // Median uten DC-leddet (gjennomsnittlig lysstyrke).
    let mut sorted: Vec<f64> = coeffs[1..].to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let median = sorted[sorted.len() / 2];
    coeffs
        .iter()
        .enumerate()
        .fold(0u64, |h, (i, &c)| if c > median { h | (1 << i) } else { h })
}

pub fn distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth;

    fn hash_of(img: &image::RgbImage) -> u64 {
        phash(&DynamicImage::ImageRgb8(img.clone()))
    }

    #[test]
    fn same_image_in_other_size_and_quality_is_close() {
        let original = synth::pattern(1600, 1200, 7);
        let h = hash_of(&original);
        let small = image::imageops::resize(&original, 400, 300, FilterType::Triangle);
        let recompressed = image::load_from_memory(&synth::encode_jpeg(&small, 60))
            .unwrap()
            .to_rgb8();
        assert!(
            distance(h, hash_of(&recompressed)) <= 6,
            "avstand {}",
            distance(h, hash_of(&recompressed))
        );
    }

    #[test]
    fn different_images_are_far_apart() {
        let mut min = u32::MAX;
        for seed in 1..12 {
            let a = hash_of(&synth::pattern(320, 240, seed));
            let b = hash_of(&synth::pattern(320, 240, seed + 100));
            min = min.min(distance(a, b));
        }
        assert!(min > 10, "minste avstand mellom ulike bilder var {min}");
    }

    #[test]
    fn is_deterministic() {
        let img = synth::pattern(200, 150, 3);
        assert_eq!(hash_of(&img), hash_of(&img));
        assert_eq!(distance(0b1011, 0b0001), 2);
    }
}
