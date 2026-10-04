//! Finner hvilket bilde i biblioteket hvert albumbilde er laget av.
//!
//! Albumprogrammer beskjærer bildene til rammen (f.eks. et liggende bilde i en kvadratisk
//! ramme), skalerer og komprimerer på nytt. Derfor sammenlignes albumbildet med utsnitt av
//! hvert bibliotekbilde med samme form som albumbildet (midt, venstre/topp, høyre/bunn),
//! med perseptuell hash. Treff over en terskel regnes som usikre og vises for kontroll.

use std::collections::BTreeMap;

use image::DynamicImage;
use p2a_core::ContentHash;
use p2a_ingest::phash;
use rayon::prelude::*;

use crate::album_pdf::AlbumImage;

/// Hamming-avstand for sikker match.
pub const SURE: u32 = 8;
/// Opp til denne avstanden er det en mulig match som bør kontrolleres.
pub const MAYBE: u32 = 14;

/// Formen avrundes, så albumbilder med nesten samme form deler utsnitt.
fn aspect_key(w: u32, h: u32) -> u32 {
    ((w as f64 / h.max(1) as f64) * 20.0).round() as u32
}

/// Utsnitt av `img` med forholdet `aspect` (= key/20): midt, start og slutt.
fn crops(img: &DynamicImage, key: u32) -> Vec<DynamicImage> {
    let a = key as f64 / 20.0;
    let (w, h) = (img.width(), img.height());
    let b = w as f64 / h as f64;
    if (a - b).abs() < 0.03 {
        return vec![img.clone()];
    }
    if a < b {
        let cw = ((h as f64 * a).round() as u32).clamp(1, w);
        [0, (w - cw) / 2, w - cw]
            .iter()
            .map(|&x| img.crop_imm(x, 0, cw, h))
            .collect()
    } else {
        let ch = ((w as f64 / a).round() as u32).clamp(1, h);
        [0, (h - ch) / 2, h - ch]
            .iter()
            .map(|&y| img.crop_imm(0, y, w, ch))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Match {
    pub page: u32,
    pub index: u32,
    /// Beste kandidat, hvis noen er innenfor [`MAYBE`].
    pub best: Option<(ContentHash, u32)>,
    /// Avstand til nest beste (annen) kandidat; stor margin = tydelig treff.
    pub runner_up: Option<u32>,
}

impl Match {
    pub fn is_sure(&self) -> bool {
        matches!(self.best, Some((_, d)) if d <= SURE)
    }
}

/// Matcher albumbilder mot biblioteket (miniatyrer holder, de er 400 px).
pub fn match_album(album: &[AlbumImage], library: &[(ContentHash, DynamicImage)]) -> Vec<Match> {
    let album_hashes: Vec<(u32, u64)> = album
        .par_iter()
        .map(|a| {
            (
                aspect_key(a.image.width(), a.image.height()),
                phash::phash(&a.image),
            )
        })
        .collect();

    // Bibliotekets hasher per form albumet bruker.
    let keys: Vec<u32> = {
        let mut k: Vec<u32> = album_hashes.iter().map(|(k, _)| *k).collect();
        k.sort();
        k.dedup();
        k
    };
    let lib_hashes: BTreeMap<u32, Vec<(usize, u64)>> = keys
        .iter()
        .map(|&key| {
            let hashes: Vec<(usize, u64)> = library
                .par_iter()
                .enumerate()
                .flat_map_iter(|(i, (_, img))| {
                    crops(img, key)
                        .into_iter()
                        .map(move |c| (i, phash::phash(&c)))
                })
                .collect();
            (key, hashes)
        })
        .collect();

    album
        .par_iter()
        .zip(album_hashes.par_iter())
        .map(|(a, (key, h))| {
            let mut best_per_photo: BTreeMap<usize, u32> = BTreeMap::new();
            for &(i, lh) in &lib_hashes[key] {
                let d = phash::distance(*h, lh);
                let e = best_per_photo.entry(i).or_insert(u32::MAX);
                *e = (*e).min(d);
            }
            let mut ranked: Vec<(u32, usize)> =
                best_per_photo.into_iter().map(|(i, d)| (d, i)).collect();
            ranked.sort();
            Match {
                page: a.page,
                index: a.index,
                best: ranked
                    .first()
                    .filter(|(d, _)| *d <= MAYBE)
                    .map(|&(d, i)| (library[i].0, d)),
                runner_up: ranked.get(1).map(|(d, _)| *d),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::album_pdf::AlbumImage;
    use p2a_ingest::synth;

    fn lib(n: u32) -> Vec<(ContentHash, DynamicImage)> {
        (0..n)
            .map(|i| {
                let mut h = [0u8; 32];
                h[0] = i as u8;
                (
                    ContentHash(h),
                    DynamicImage::ImageRgb8(image::imageops::thumbnail(
                        &synth::pattern(1600, 1200, i),
                        400,
                        300,
                    )),
                )
            })
            .collect()
    }

    #[test]
    fn finds_cropped_and_recompressed_copies() {
        let library = lib(30);
        // Albumet: bilde 7 beskåret kvadratisk (midt), bilde 12 helt, bilde 20 beskåret til venstre.
        let src = |i: u32| DynamicImage::ImageRgb8(synth::pattern(1600, 1200, i));
        let recompress = |img: DynamicImage| {
            image::load_from_memory(&synth::encode_jpeg(&img.to_rgb8(), 75)).unwrap()
        };
        let album = vec![
            AlbumImage {
                page: 1,
                index: 0,
                image: recompress(src(7).crop_imm(200, 0, 1200, 1200)),
            },
            AlbumImage {
                page: 1,
                index: 1,
                image: recompress(src(12).resize(900, 675, image::imageops::FilterType::Triangle)),
            },
            AlbumImage {
                page: 2,
                index: 0,
                image: recompress(src(20).crop_imm(0, 0, 1200, 1200)),
            },
            // Et bilde som ikke finnes i biblioteket.
            AlbumImage {
                page: 3,
                index: 0,
                image: src(99),
            },
        ];
        let m = match_album(&album, &library);
        assert_eq!(m[0].best.map(|b| b.0), Some(library[7].0), "{:?}", m[0]);
        assert_eq!(m[1].best.map(|b| b.0), Some(library[12].0), "{:?}", m[1]);
        assert_eq!(m[2].best.map(|b| b.0), Some(library[20].0), "{:?}", m[2]);
        assert!(m[..3].iter().all(Match::is_sure), "{m:?}");
        assert!(!m[3].is_sure(), "{:?}", m[3]);
    }
}
