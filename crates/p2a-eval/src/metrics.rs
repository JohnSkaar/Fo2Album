//! Målinger av et utvalg mot et gullsett (SCORING.md §10).
//!
//! «Nesten samme bilde» teller som treff: et bilde fra samme serie (≤ 20 s) eller en
//! nesten-dublett (pHash ≤ 10) av et bilde familien valgte.

use std::collections::{BTreeSet, HashMap, HashSet};

use p2a_core::events::assign_events;
use p2a_core::{ContentHash, PhotoMeta};
use serde::Serialize;

/// Bilder innen så mange sekunder regnes som samme øyeblikk.
const SAME_MOMENT_SECONDS: i64 = 20;
/// pHash-avstand for «nesten samme bilde».
const SAME_IMAGE_HAMMING: u32 = 10;
/// Hendelser med færre bilder enn dette regnes ikke med i dekningen (SCORING §11).
pub const EVENT_MIN_PHOTOS: usize = 3;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Metrics {
    pub bilder_i_biblioteket: usize,
    pub bilder_i_fasit: usize,
    pub bilder_valgt: usize,
    /// Andel av utvalget som familien også valgte (med «nesten samme» som treff).
    pub presisjon: f64,
    /// Andel av familiens valg som er med i utvalget.
    pub gjenfinning: f64,
    pub f1: f64,
    /// Hendelser (≥ EVENT_MIN_PHOTOS bilder) i biblioteket.
    pub hendelser: usize,
    /// Andel av hendelsene som utvalget har med.
    pub hendelsesdekning_utvalg: f64,
    /// Andel av hendelsene som familiens eget album har med (sjekker «nesten alle»).
    pub hendelsesdekning_fasit: f64,
    /// Andel av hendelsene i familiens album som utvalget også har med.
    pub hendelser_gjenfunnet: f64,
    /// Andel av månedene i familiens album som utvalget har med.
    pub maaneder_gjenfunnet: f64,
    /// Snitt av høyeste likhet (1 − pHash-avstand/64) mellom hvert valgt bilde og de andre.
    pub redundans: f64,
}

fn near(a: &PhotoMeta, b: &PhotoMeta) -> bool {
    if a.hash == b.hash {
        return true;
    }
    if let (Some(ta), Some(tb)) = (a.taken_at, b.taken_at) {
        if (ta.local_seconds() - tb.local_seconds()).abs() <= SAME_MOMENT_SECONDS {
            return true;
        }
    }
    matches!((a.phash, b.phash), (Some(x), Some(y)) if (x ^ y).count_ones() <= SAME_IMAGE_HAMMING)
}

/// Andel av `items` som har et «nesten samme» bilde i `others`.
fn covered(items: &[&PhotoMeta], others: &[&PhotoMeta]) -> f64 {
    if items.is_empty() {
        return 0.0;
    }
    items
        .iter()
        .filter(|a| others.iter().any(|b| near(a, b)))
        .count() as f64
        / items.len() as f64
}

fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 {
        0.0
    } else {
        a as f64 / b as f64
    }
}

pub fn evaluate(library: &[PhotoMeta], gold: &[ContentHash], selection: &[ContentHash]) -> Metrics {
    let by_hash: HashMap<ContentHash, &PhotoMeta> = library.iter().map(|p| (p.hash, p)).collect();
    let fasit: Vec<&PhotoMeta> = gold
        .iter()
        .filter_map(|h| by_hash.get(h).copied())
        .collect();
    let valgt: Vec<&PhotoMeta> = selection
        .iter()
        .filter_map(|h| by_hash.get(h).copied())
        .collect();

    let presisjon = covered(&valgt, &fasit);
    let gjenfinning = covered(&fasit, &valgt);
    let f1 = if presisjon + gjenfinning > 0.0 {
        2.0 * presisjon * gjenfinning / (presisjon + gjenfinning)
    } else {
        0.0
    };

    // Hendelser over hele biblioteket for året.
    let events = assign_events(library);
    let mut size: HashMap<usize, usize> = HashMap::new();
    for e in events.iter().flatten() {
        *size.entry(*e).or_default() += 1;
    }
    let event_of: HashMap<ContentHash, usize> = library
        .iter()
        .zip(&events)
        .filter_map(|(p, e)| Some((p.hash, (*e)?)))
        .collect();
    let big: HashSet<usize> = size
        .iter()
        .filter(|(_, &n)| n >= EVENT_MIN_PHOTOS)
        .map(|(&e, _)| e)
        .collect();
    let events_in = |set: &[&PhotoMeta]| -> HashSet<usize> {
        set.iter()
            .filter_map(|p| event_of.get(&p.hash).copied())
            .collect()
    };
    let (ev_sel, ev_gold) = (events_in(&valgt), events_in(&fasit));

    let month = |p: &&PhotoMeta| p.taken_at.map(|t| (t.year, t.month));
    let months_gold: BTreeSet<_> = fasit.iter().filter_map(month).collect();
    let months_sel: BTreeSet<_> = valgt.iter().filter_map(month).collect();

    let redundans = if valgt.len() < 2 {
        0.0
    } else {
        valgt
            .iter()
            .enumerate()
            .map(|(i, a)| {
                valgt
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .filter_map(|(_, b)| {
                        Some(1.0 - (a.phash? ^ b.phash?).count_ones() as f64 / 64.0)
                    })
                    .fold(0.0, f64::max)
            })
            .sum::<f64>()
            / valgt.len() as f64
    };

    Metrics {
        bilder_i_biblioteket: library.len(),
        bilder_i_fasit: fasit.len(),
        bilder_valgt: valgt.len(),
        presisjon,
        gjenfinning,
        f1,
        hendelser: big.len(),
        hendelsesdekning_utvalg: ratio(ev_sel.intersection(&big).count(), big.len()),
        hendelsesdekning_fasit: ratio(ev_gold.intersection(&big).count(), big.len()),
        hendelser_gjenfunnet: ratio(ev_sel.intersection(&ev_gold).count(), ev_gold.len()),
        maaneder_gjenfunnet: ratio(
            months_gold.intersection(&months_sel).count(),
            months_gold.len(),
        ),
        redundans,
    }
}

/// Én linje per gullsett i `eval/RESULTS.md`.
pub fn markdown_row(navn: &str, m: &Metrics) -> String {
    let p = |x: f64| format!("{:.0} %", x * 100.0);
    format!(
        "| {navn} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.2} |",
        m.bilder_i_biblioteket,
        m.bilder_i_fasit,
        p(m.presisjon),
        p(m.gjenfinning),
        p(m.f1),
        p(m.hendelsesdekning_utvalg),
        p(m.hendelsesdekning_fasit),
        p(m.hendelser_gjenfunnet),
        p(m.maaneder_gjenfunnet),
        m.redundans
    )
}

pub const MARKDOWN_HEADER: &str = "| Gullsett | Bilder i biblioteket | I familiens album | Presisjon | Gjenfinning | F1 | Hendelser med (utvalg) | Hendelser med (familien) | Familiens hendelser gjenfunnet | Måneder gjenfunnet | Redundans |\n|---|---|---|---|---|---|---|---|---|---|---|";

#[cfg(test)]
mod tests {
    use super::*;
    use p2a_core::{DateSource, TakenAt};

    fn photo(id: u8, day: u8, hour: u8, phash: u64) -> PhotoMeta {
        let mut p = PhotoMeta::new(ContentHash([id; 32]));
        p.taken_at = TakenAt::new(2010, 6, day, hour, 0, 0);
        p.date_source = Some(DateSource::Exif);
        p.phash = Some(phash);
        p
    }

    #[test]
    fn perfect_and_partial_selections() {
        // Tre hendelser à tre bilder (dag 1, 5, 9), med ulike pHash.
        let lib: Vec<PhotoMeta> = (0..9u8)
            .map(|i| {
                photo(
                    i,
                    1 + (i / 3) * 4,
                    10 + (i % 3) * 2,
                    (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
                )
            })
            .collect();
        let gold = vec![lib[0].hash, lib[3].hash, lib[6].hash];
        let m = evaluate(&lib, &gold, &gold);
        assert_eq!((m.presisjon, m.gjenfinning, m.f1), (1.0, 1.0, 1.0));
        assert_eq!(m.hendelser, 3);
        assert_eq!(
            (m.hendelsesdekning_utvalg, m.hendelsesdekning_fasit),
            (1.0, 1.0)
        );

        // Bare den første hendelsen (to andre bilder fra den), ingen treff på samme øyeblikk.
        let sel = vec![lib[1].hash, lib[2].hash];
        let m = evaluate(&lib, &gold, &sel);
        assert_eq!(m.presisjon, 0.0);
        assert!((m.hendelser_gjenfunnet - 1.0 / 3.0).abs() < 1e-9);
        assert!((m.hendelsesdekning_utvalg - 1.0 / 3.0).abs() < 1e-9);
        assert_eq!(m.maaneder_gjenfunnet, 1.0);
    }

    #[test]
    fn same_moment_counts_as_hit() {
        let mut a = photo(1, 3, 12, 0);
        let mut b = photo(2, 3, 12, u64::MAX);
        b.taken_at = TakenAt::new(2010, 6, 3, 12, 0, 10); // 10 s senere: samme serie
        a.phash = Some(0);
        let lib = vec![a.clone(), b.clone()];
        let m = evaluate(&lib, &[a.hash], &[b.hash]);
        assert_eq!(m.presisjon, 1.0);
    }
}
