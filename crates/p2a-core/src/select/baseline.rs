//! Prototypens utvalg (prototype/fo2album-prototype.html, «Foreslå de beste bildene»),
//! oversatt til Rust og justert til å velge nøyaktig `k` bilder, så det kan sammenlignes
//! rettferdig med familiens eget album.
//!
//! 1. Poeng = skarphet 50 % + eksponering 30 % + farge 20 % (`BasicQuality::score`).
//! 2. Serier (bilder innen 20 s): bare det beste bildet er kandidat.
//! 3. Kvote per måned ∝ √(antall bilder i måneden), minst 3 når det finnes.
//! 4. Innen måneden: høyest poeng først, helst minst 30 min mellom valgte bilder.

use std::collections::BTreeMap;

use crate::{ContentHash, PhotoMeta};

const BURST_SECONDS: i64 = 20;
const SPACING_SECONDS: i64 = 30 * 60;
const MIN_PER_MONTH: usize = 3;

struct Cand<'a> {
    p: &'a PhotoMeta,
    t: i64,
    score: f32,
    burst_best: bool,
}

/// Velger `k` bilder. Bilder uten dato tas ikke med. Deterministisk.
pub fn select(photos: &[PhotoMeta], k: usize) -> Vec<ContentHash> {
    let mut cands: Vec<Cand> = photos
        .iter()
        .filter_map(|p| {
            Some(Cand {
                p,
                t: p.taken_at?.local_seconds(),
                score: p.quality.map_or(0.0, |q| q.score()),
                burst_best: true,
            })
        })
        .collect();
    cands.sort_by(|a, b| a.t.cmp(&b.t).then(a.p.hash.cmp(&b.p.hash)));
    mark_bursts(&mut cands);

    // Per måned (år*12 + måned), i tidsrekkefølge.
    let mut months: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
    for (i, c) in cands.iter().enumerate() {
        let t = c.p.taken_at.expect("filtrert");
        months
            .entry(t.year * 12 + t.month as i32)
            .or_default()
            .push(i);
    }
    let quotas = month_quotas(&months.values().map(Vec::len).collect::<Vec<_>>(), k);

    let mut chosen: Vec<usize> = Vec::new();
    for (idx, want) in months.values().zip(quotas) {
        let mut ranked: Vec<usize> = idx
            .iter()
            .copied()
            .filter(|&i| cands[i].burst_best)
            .collect();
        ranked.sort_by(|&a, &b| by_score(&cands, a, b));
        let mut picked: Vec<usize> = Vec::new();
        for &i in &ranked {
            if picked.len() >= want {
                break;
            }
            if picked
                .iter()
                .all(|&j| (cands[j].t - cands[i].t).abs() > SPACING_SECONDS)
            {
                picked.push(i);
            }
        }
        for &i in &ranked {
            if picked.len() >= want {
                break;
            }
            if !picked.contains(&i) {
                picked.push(i);
            }
        }
        chosen.extend(picked);
    }

    // Juster til nøyaktig k: fyll med de beste gjenværende, eller fjern de svakeste.
    if chosen.len() < k {
        let mut rest: Vec<usize> = (0..cands.len()).filter(|i| !chosen.contains(i)).collect();
        rest.sort_by(|&a, &b| {
            cands[b]
                .burst_best
                .cmp(&cands[a].burst_best)
                .then(by_score(&cands, a, b))
        });
        chosen.extend(rest.into_iter().take(k - chosen.len()));
    } else if chosen.len() > k {
        chosen.sort_by(|&a, &b| by_score(&cands, a, b));
        chosen.truncate(k);
    }
    chosen.sort_by(|&a, &b| {
        cands[a]
            .t
            .cmp(&cands[b].t)
            .then(cands[a].p.hash.cmp(&cands[b].p.hash))
    });
    chosen.into_iter().map(|i| cands[i].p.hash).collect()
}

fn by_score(c: &[Cand], a: usize, b: usize) -> std::cmp::Ordering {
    c[b].score
        .total_cmp(&c[a].score)
        .then(c[a].p.hash.cmp(&c[b].p.hash))
}

fn mark_bursts(cands: &mut [Cand]) {
    let mut start = 0;
    for i in 1..=cands.len() {
        let split = i == cands.len() || cands[i].t - cands[i - 1].t > BURST_SECONDS;
        if split {
            if i - start > 1 {
                let best = (start..i)
                    .max_by(|&a, &b| {
                        cands[a]
                            .score
                            .total_cmp(&cands[b].score)
                            .then(cands[b].p.hash.cmp(&cands[a].p.hash))
                    })
                    .expect("ikke tom");
                for (j, c) in cands.iter_mut().enumerate().take(i).skip(start) {
                    c.burst_best = j == best;
                }
            }
            start = i;
        }
    }
}

/// Kvote per måned ∝ √n, minst 3 (eller alle hvis færre), sum så nær `k` som mulig.
fn month_quotas(sizes: &[usize], k: usize) -> Vec<usize> {
    let roots: f64 = sizes.iter().map(|&n| (n as f64).sqrt()).sum();
    if roots == 0.0 {
        return vec![0; sizes.len()];
    }
    sizes
        .iter()
        .map(|&n| {
            let share = (k as f64 * (n as f64).sqrt() / roots).round() as usize;
            share.max(MIN_PER_MONTH.min(n)).min(n)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BasicQuality, DateSource, TakenAt};

    fn photo(id: u16, month: u8, day: u8, h: u8, m: u8, s: u8, sharp: f32) -> PhotoMeta {
        let mut hash = [0u8; 32];
        hash[..2].copy_from_slice(&id.to_be_bytes());
        let mut p = PhotoMeta::new(ContentHash(hash));
        p.taken_at = TakenAt::new(2010, month, day, h, m, s);
        p.date_source = Some(DateSource::Exif);
        p.quality = Some(BasicQuality {
            sharp,
            exposure: 0.8,
            color: 0.5,
            skin: 0.0,
        });
        p
    }

    #[test]
    fn selects_exactly_k_and_is_deterministic() {
        let photos: Vec<PhotoMeta> = (0..200u16)
            .map(|i| {
                photo(
                    i,
                    1 + (i % 12) as u8,
                    1 + (i % 28) as u8,
                    (i % 24) as u8,
                    0,
                    0,
                    (i % 10) as f32 / 10.0,
                )
            })
            .collect();
        for k in [10, 50, 150, 200, 250] {
            let a = select(&photos, k);
            assert_eq!(a.len(), k.min(200), "k = {k}");
            assert_eq!(a, select(&photos, k));
        }
    }

    #[test]
    fn keeps_only_best_of_a_burst_when_possible() {
        let mut photos = vec![
            photo(1, 7, 14, 12, 0, 0, 0.4),
            photo(2, 7, 14, 12, 0, 5, 0.9), // beste i serien
            photo(3, 7, 14, 12, 0, 10, 0.5),
        ];
        photos.extend((10..20).map(|i| photo(i, 7, 1 + i as u8, 9, 0, 0, 0.6)));
        let chosen = select(&photos, 5);
        assert!(chosen.contains(&photos[1].hash));
        assert!(!chosen.contains(&photos[0].hash) && !chosen.contains(&photos[2].hash));
    }

    #[test]
    fn every_month_gets_some_when_room() {
        let mut photos: Vec<PhotoMeta> = (0..100)
            .map(|i| photo(i, 7, 1 + (i % 28) as u8, (i % 24) as u8, 0, 0, 0.9))
            .collect();
        photos.extend((100..104).map(|i| photo(i, 2, 3, (i % 24) as u8, 0, 0, 0.1)));
        let chosen = select(&photos, 20);
        let feb = photos[100..]
            .iter()
            .filter(|p| chosen.contains(&p.hash))
            .count();
        assert_eq!(feb, 3, "februar får minst 3 selv med lave poeng");
    }

    #[test]
    fn quotas_follow_sqrt() {
        assert_eq!(month_quotas(&[100, 25], 15), vec![10, 5]);
        assert_eq!(month_quotas(&[2, 100], 10), vec![2, 9]);
    }
}
