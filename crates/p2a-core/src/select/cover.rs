//! Forslag til forside og bakside (PRODUCT.md, «Forsidehjelp»; SCORING.md §8).
//!
//! Forsiden er viktig. Appen foreslår de beste bildene gjennom hele året i to grupper: minst
//! to med personer (familien eller barna) og minst fire oversiktsbilder (de beste fotoene,
//! gjerne natur eller steder). Bilder brukeren har merket som kandidater, kommer først.
//! Baksiden velges normalt fra de samme kandidatene. Høyst ett forslag per dag, så
//! forslagene spenner over året.

use std::collections::HashSet;

use crate::{ContentHash, PhotoMeta};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CoverCandidates {
    /// Bilder brukeren selv har merket som kandidater, i tidsrekkefølge.
    pub marked: Vec<ContentHash>,
    /// Med personer (familien eller barna), best først.
    pub people: Vec<ContentHash>,
    /// Oversiktsbilder, best først.
    pub overview: Vec<ContentHash>,
}

/// Andel hudtoner som regnes som personer (samme som utkastet, til M4).
const PEOPLE_SKIN: f32 = 0.02;

/// Forslag fra årets bilder. `people` og `overview` er minimum antall forslag.
pub fn candidates(
    photos: &[PhotoMeta],
    marked: &HashSet<ContentHash>,
    people: usize,
    overview: usize,
) -> CoverCandidates {
    let mut scored: Vec<(&PhotoMeta, f32, bool)> = photos
        .iter()
        .filter(|p| p.taken_at.is_some() && !marked.contains(&p.hash))
        .filter_map(|p| {
            let q = p.quality?;
            // Forsiden trykkes stort: oppløsning teller, og skarphet mer enn ellers.
            let big = p.pixels() >= 2_000_000;
            let score =
                0.45 * q.sharp + 0.3 * q.color + 0.25 * q.exposure + if big { 0.1 } else { 0.0 };
            Some((p, score, q.skin >= PEOPLE_SKIN))
        })
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.hash.cmp(&b.0.hash)));
    let pick = |with_people: bool, n: usize| {
        let mut days = HashSet::new();
        let mut out = Vec::new();
        for (p, _, has) in &scored {
            if out.len() >= n {
                break;
            }
            let t = p.taken_at.expect("har dato");
            if *has == with_people && days.insert((t.year, t.month, t.day)) {
                out.push(p.hash);
            }
        }
        out
    };
    let mut marked_list: Vec<&PhotoMeta> =
        photos.iter().filter(|p| marked.contains(&p.hash)).collect();
    marked_list.sort_by_key(|p| p.taken_at.map(|t| t.local_seconds()));
    CoverCandidates {
        marked: marked_list.into_iter().map(|p| p.hash).collect(),
        people: pick(true, people),
        overview: pick(false, overview),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BasicQuality, TakenAt};

    fn photo(id: u8, day: u8, sharp: f32, skin: f32) -> PhotoMeta {
        let mut p = PhotoMeta::new(ContentHash([id; 32]));
        p.taken_at = TakenAt::new(2011, 5, day, 12, 0, 0);
        p.width = Some(4000);
        p.height = Some(3000);
        p.quality = Some(BasicQuality {
            sharp,
            exposure: 0.8,
            color: 0.6,
            skin,
        });
        p
    }

    #[test]
    fn at_least_two_with_people_and_four_overviews_from_different_days() {
        let mut photos: Vec<PhotoMeta> = (1..=10)
            .map(|d| photo(d, d, 0.5 + d as f32 / 40.0, 0.1))
            .collect();
        photos.extend((11..=20).map(|d| photo(d, d, 0.4 + d as f32 / 50.0, 0.0)));
        photos.push(photo(30, 20, 0.99, 0.0)); // samme dag som et annet oversiktsbilde
        let marked = HashSet::from([ContentHash([1; 32])]);
        let c = candidates(&photos, &marked, 2, 4);
        assert_eq!(c.marked, vec![ContentHash([1; 32])]);
        assert_eq!(c.people.len(), 2);
        assert_eq!(c.overview.len(), 4);
        assert_eq!(c.overview[0], ContentHash([30; 32]));
        assert!(
            !c.overview.contains(&ContentHash([20; 32])),
            "høyst ett forslag per dag"
        );
        assert!(
            !c.people.contains(&ContentHash([1; 32])),
            "merket kommer ikke to ganger"
        );
    }
}
