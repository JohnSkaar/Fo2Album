//! Grupperer ansikter i personer. Deterministisk: samme ansikter i samme rekkefølge gir alltid
//! samme personer.
//!
//! Metode: ansiktene gås gjennom med de beste først (størst og skarpest). Hvert ansikt legges til
//! personen med likest gjennomsnitt hvis likheten er over terskelen, ellers blir det en ny
//! person. Til slutt slås personer sammen hvis gjennomsnittene deres er like nok. Ansikter
//! brukeren har plassert selv (navngitt, flyttet), sendes inn som `fixed` og flyttes aldri.

use crate::Embedding;

/// Ett ansikt som skal grupperes.
#[derive(Clone, Debug)]
pub struct Item<'a> {
    pub embedding: &'a Embedding,
    /// Hvor godt ansiktet er (størrelse × skarphet); de beste brukes først.
    pub quality: f32,
    /// Personen brukeren har lagt ansiktet til, hvis noen.
    pub fixed: Option<u32>,
}

/// Resultatet: en person (gruppe) per ansikt, i samme rekkefølge som inn. Grupper som kommer
/// fra `fixed`, beholder nummeret sitt; nye grupper får nummer fra `first_new`.
pub fn cluster(items: &[Item<'_>], threshold: f32, first_new: u32) -> Vec<u32> {
    struct Group {
        id: u32,
        sum: Vec<f32>,
        fixed: bool,
    }
    impl Group {
        fn sim(&self, e: &Embedding) -> f32 {
            let n = self.sum.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
            self.sum.iter().zip(&e.0).map(|(a, b)| a * b).sum::<f32>() / n
        }
        fn add(&mut self, e: &Embedding) {
            self.sum.iter_mut().zip(&e.0).for_each(|(a, b)| *a += b);
        }
    }
    let mut groups: Vec<Group> = Vec::new();
    let mut label = vec![u32::MAX; items.len()];
    // Først ansiktene brukeren har plassert.
    for (i, it) in items.iter().enumerate() {
        if let Some(id) = it.fixed {
            match groups.iter_mut().find(|g| g.id == id) {
                Some(g) => g.add(it.embedding),
                None => groups.push(Group {
                    id,
                    sum: it.embedding.0.clone(),
                    fixed: true,
                }),
            }
            label[i] = id;
        }
    }
    let mut order: Vec<usize> = (0..items.len())
        .filter(|&i| items[i].fixed.is_none())
        .collect();
    order.sort_by(|&a, &b| {
        items[b]
            .quality
            .total_cmp(&items[a].quality)
            .then(a.cmp(&b))
    });
    let mut next = first_new.max(groups.iter().map(|g| g.id + 1).max().unwrap_or(0));
    for i in order {
        let e = items[i].embedding;
        let best = groups
            .iter()
            .enumerate()
            .map(|(k, g)| (k, g.sim(e)))
            .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)));
        match best {
            Some((k, s)) if s >= threshold => {
                groups[k].add(e);
                label[i] = groups[k].id;
            }
            _ => {
                groups.push(Group {
                    id: next,
                    sum: e.0.clone(),
                    fixed: false,
                });
                label[i] = next;
                next += 1;
            }
        }
    }
    // Slå sammen grupper som ligner. En gruppe brukeren har plassert, beholder nummeret; to
    // slike slås aldri sammen.
    loop {
        let mut best: Option<(usize, usize, f32)> = None;
        for a in 0..groups.len() {
            for b in a + 1..groups.len() {
                if groups[a].fixed && groups[b].fixed {
                    continue;
                }
                let n = groups[b]
                    .sum
                    .iter()
                    .map(|x| x * x)
                    .sum::<f32>()
                    .sqrt()
                    .max(1e-6);
                let s = groups[a].sim(&Embedding(groups[b].sum.iter().map(|x| x / n).collect()));
                if s >= threshold && best.is_none_or(|(_, _, bs)| s > bs) {
                    best = Some((a, b, s));
                }
            }
        }
        let Some((a, b, _)) = best else { break };
        let (keep, gone) = if groups[b].fixed { (b, a) } else { (a, b) };
        let (kid, gid) = (groups[keep].id, groups[gone].id);
        let sum = groups[gone].sum.clone();
        groups[keep]
            .sum
            .iter_mut()
            .zip(&sum)
            .for_each(|(x, y)| *x += y);
        label
            .iter_mut()
            .filter(|l| **l == gid)
            .for_each(|l| *l = kid);
        groups.remove(gone);
    }
    label
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Et kjennetegn nær akse `k`, med litt støy i akse `k + 1`.
    fn e(k: usize, noise: f32) -> Embedding {
        let mut v = vec![0.0; 8];
        v[k] = 1.0;
        v[(k + 1) % 8] = noise;
        Embedding::normalized(v)
    }

    fn items(es: &[Embedding]) -> Vec<Item<'_>> {
        es.iter()
            .map(|e| Item {
                embedding: e,
                quality: 1.0,
                fixed: None,
            })
            .collect()
    }

    #[test]
    fn to_personer() {
        let es = [e(0, 0.1), e(3, 0.0), e(0, 0.3), e(3, 0.2), e(0, 0.0)];
        let l = cluster(&items(&es), 0.6, 0);
        assert_eq!(l[0], l[2]);
        assert_eq!(l[0], l[4]);
        assert_eq!(l[1], l[3]);
        assert_ne!(l[0], l[1]);
    }

    #[test]
    fn deterministisk() {
        let es = [e(1, 0.2), e(2, 0.1), e(1, 0.0), e(5, 0.4), e(2, 0.3)];
        assert_eq!(cluster(&items(&es), 0.6, 10), cluster(&items(&es), 0.6, 10));
    }

    #[test]
    fn brukerens_plassering_flyttes_ikke() {
        let es = [e(0, 0.0), e(0, 0.1), e(4, 0.0)];
        let mut it = items(&es);
        it[2].fixed = Some(7);
        // Ansikt 1 er likt ansikt 0, ikke brukerens person 7.
        let l = cluster(&it, 0.6, 100);
        assert_eq!(l[2], 7);
        assert_eq!(l[0], l[1]);
        assert!(l[0] >= 100);
    }

    #[test]
    fn ukjent_ansikt_finner_brukerens_person() {
        let es = [e(0, 0.0), e(0, 0.2)];
        let mut it = items(&es);
        it[0].fixed = Some(3);
        assert_eq!(cluster(&it, 0.6, 100), vec![3, 3]);
    }
}
