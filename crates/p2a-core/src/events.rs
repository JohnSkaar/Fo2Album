//! Enkel hendelsesinndeling på tid (SCORING.md §4.2, første trinn). Brukes av evalueringen
//! nå; M5 legger til sted, sammenslåing av ferier og viktighet.

use crate::PhotoMeta;

/// Ny hendelse når det går mer enn dette mellom to bilder.
pub const EVENT_GAP_SECONDS: i64 = 3 * 3600;

/// Hendelsesnummer per bilde (samme rekkefølge som `photos`). Bilder uten dato får `None`.
pub fn assign_events(photos: &[PhotoMeta]) -> Vec<Option<usize>> {
    let mut order: Vec<(i64, usize)> = photos
        .iter()
        .enumerate()
        .filter_map(|(i, p)| Some((p.taken_at?.local_seconds(), i)))
        .collect();
    order.sort();
    let mut out = vec![None; photos.len()];
    let mut event = 0;
    for (n, &(t, i)) in order.iter().enumerate() {
        if n > 0 && t - order[n - 1].0 > EVENT_GAP_SECONDS {
            event += 1;
        }
        out[i] = Some(event);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContentHash, TakenAt};

    #[test]
    fn splits_on_long_gaps() {
        let p = |h: u8, d: u8| {
            let mut m = PhotoMeta::new(ContentHash([h; 32]));
            m.taken_at = TakenAt::new(2010, 5, d, h, 0, 0);
            m
        };
        let photos = vec![
            p(10, 17),
            p(12, 17),
            p(20, 17),
            p(9, 18),
            PhotoMeta::new(ContentHash([0; 32])),
        ];
        assert_eq!(
            assign_events(&photos),
            vec![Some(0), Some(0), Some(1), Some(2), None]
        );
    }
}
