//! Transkodede dubletter: samme bilde i ulik oppløsning eller format (SCORING.md §2, nivå 2).
//!
//! Ren funksjon over metadata, så den kan testes deterministisk og kjøres på nytt når
//! nye kilder legges til.

use crate::config::DedupConfig;
use crate::{ContentHash, DateSource, PhotoMeta};

fn distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Sekunder i UTC hvis tidssonen er kjent, ellers lokal tid.
fn seconds(p: &PhotoMeta) -> Option<i64> {
    let t = p.taken_at?;
    Some(t.local_seconds() - t.offset_minutes.unwrap_or(0) as i64 * 60)
}

fn exif_time(p: &PhotoMeta) -> Option<i64> {
    (p.date_source == Some(DateSource::Exif))
        .then(|| seconds(p))
        .flatten()
}

/// Er `a` og `b` samme bilde?
pub fn is_near_duplicate(a: &PhotoMeta, b: &PhotoMeta, cfg: &DedupConfig) -> bool {
    let (Some(ha), Some(hb)) = (a.phash, b.phash) else {
        return false;
    };
    let d = distance(ha, hb);
    match (exif_time(a), exif_time(b)) {
        (Some(ta), Some(tb)) => {
            d <= cfg.near_dup_hamming && (ta - tb).abs() <= cfg.near_dup_seconds
        }
        _ => d <= cfg.near_dup_hamming_without_time,
    }
}

/// Hvilken kopi som beholdes: høyest oppløsning, deretter best metadata.
fn rank(p: &PhotoMeta) -> (u64, bool, bool, bool, std::cmp::Reverse<ContentHash>) {
    (
        p.pixels(),
        p.date_source == Some(DateSource::Exif),
        p.camera_make.is_some() || p.camera_model.is_some(),
        p.gps.is_some(),
        // Siste utvei: stabil rekkefølge.
        std::cmp::Reverse(p.hash),
    )
}

/// Finner grupper av samme bilde og returnerer (kopi, beste) for hver kopi.
pub fn find_near_duplicates(
    photos: &[PhotoMeta],
    cfg: &DedupConfig,
) -> Vec<(ContentHash, ContentHash)> {
    let n = photos.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for i in 0..n {
        for j in i + 1..n {
            if is_near_duplicate(&photos[i], &photos[j], cfg) {
                let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                if ri != rj {
                    parent[ri] = rj;
                }
            }
        }
    }
    let mut groups: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
    for i in 0..n {
        let r = find(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }
    let mut out = Vec::new();
    for members in groups.values().filter(|m| m.len() > 1) {
        let best = *members.iter().max_by_key(|&&i| rank(&photos[i])).unwrap();
        for &i in members {
            if i != best {
                out.push((photos[i].hash, photos[best].hash));
            }
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TakenAt;

    fn photo(id: u8, phash: u64, w: u32, exif: Option<(u8, u8)>) -> PhotoMeta {
        let mut p = PhotoMeta::new(ContentHash([id; 32]));
        p.phash = Some(phash);
        p.width = Some(w);
        p.height = Some(w * 3 / 4);
        if let Some((min, sec)) = exif {
            p.taken_at = TakenAt::new(2011, 7, 14, 12, min, sec);
            p.date_source = Some(DateSource::Exif);
            p.camera_make = Some("Apple".into());
        }
        p
    }

    #[test]
    fn keeps_highest_resolution_with_exif() {
        let cfg = DedupConfig::default();
        let heic = photo(1, 0xFFFF_0000, 4032, Some((34, 56)));
        let jpeg_small = photo(2, 0xFFFF_0003, 1600, Some((34, 56))); // 2 bit unna
        let whatsapp = photo(3, 0xFFFF_0001, 1280, None); // 1 bit unna, uten EXIF
        let other = photo(4, 0x0000_FFFF, 4032, Some((34, 57)));
        let pairs = find_near_duplicates(
            &[heic.clone(), jpeg_small.clone(), whatsapp.clone(), other],
            &cfg,
        );
        assert_eq!(
            pairs,
            vec![(jpeg_small.hash, heic.hash), (whatsapp.hash, heic.hash)]
        );
    }

    #[test]
    fn burst_frames_with_different_times_are_not_duplicates() {
        let cfg = DedupConfig::default();
        let a = photo(1, 0xF0F0, 4032, Some((34, 50)));
        let b = photo(2, 0xF0F1, 4032, Some((34, 56))); // lik hash, men 6 s senere
        assert!(!is_near_duplicate(&a, &b, &cfg));
        assert!(find_near_duplicates(&[a, b], &cfg).is_empty());
    }

    #[test]
    fn stricter_threshold_without_time() {
        let cfg = DedupConfig::default();
        let a = photo(1, 0, 4032, Some((0, 0)));
        let five_bits = photo(2, 0b11111, 1280, None);
        let four_bits = photo(3, 0b1111, 1280, None);
        assert!(!is_near_duplicate(&a, &five_bits, &cfg));
        assert!(is_near_duplicate(&a, &four_bits, &cfg));
    }

    #[test]
    fn utc_offsets_are_respected() {
        let cfg = DedupConfig::default();
        let mut a = photo(1, 0, 4032, Some((34, 56)));
        let mut b = photo(2, 0, 1600, Some((34, 56)));
        a.taken_at.as_mut().unwrap().offset_minutes = Some(120);
        b.taken_at.as_mut().unwrap().offset_minutes = Some(60);
        // Samme lokale klokkeslett i ulike tidssoner er ikke samme øyeblikk.
        assert!(!is_near_duplicate(&a, &b, &cfg));
    }
}
