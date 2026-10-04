//! Grupperer ansikter i personer. Deterministisk: samme ansikter i samme rekkefølge gir alltid
//! samme personer.
//!
//! Metode: ansiktene gås gjennom med de beste først (størst og skarpest). Hvert ansikt legges til
//! personen med likest gjennomsnitt hvis likheten er over terskelen, ellers blir det en ny
//! person. Deretter slås personer sammen hvis gjennomsnittene deres er like nok, de likeste
//! først. Ansikter brukeren har plassert selv (navngitt, flyttet), sendes inn som `fixed` og
//! flyttes aldri. To ansikter i samme bilde er aldri samme person.

use std::collections::HashSet;

use crate::Embedding;

/// Ett ansikt som skal grupperes.
#[derive(Clone, Debug)]
pub struct Item<'a> {
    pub embedding: &'a Embedding,
    /// Hvor godt ansiktet er (størrelse × skarphet); de beste brukes først.
    pub quality: f32,
    /// Personen brukeren har lagt ansiktet til, hvis noen.
    pub fixed: Option<u32>,
    /// Bildet ansiktet er i.
    pub photo: u64,
}

struct Group {
    id: u32,
    sum: Vec<f32>,
    fixed: bool,
    photos: HashSet<u64>,
    alive: bool,
}

impl Group {
    fn new(id: u32, it: &Item<'_>, fixed: bool) -> Self {
        Group {
            id,
            sum: it.embedding.0.clone(),
            fixed,
            photos: [it.photo].into(),
            alive: true,
        }
    }
    fn add(&mut self, it: &Item<'_>) {
        self.sum
            .iter_mut()
            .zip(&it.embedding.0)
            .for_each(|(a, b)| *a += b);
        self.photos.insert(it.photo);
    }
    fn unit(&self) -> Vec<f32> {
        let n = self.sum.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
        self.sum.iter().map(|x| x / n).collect()
    }
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Resultatet: en person (gruppe) per ansikt, i samme rekkefølge som inn. Grupper som kommer
/// fra `fixed`, beholder nummeret sitt; nye grupper får nummer fra `first_new`.
pub fn cluster(items: &[Item<'_>], threshold: f32, first_new: u32) -> Vec<u32> {
    let mut groups: Vec<Group> = Vec::new();
    let mut label = vec![u32::MAX; items.len()];
    // Først ansiktene brukeren har plassert.
    for (i, it) in items.iter().enumerate() {
        if let Some(id) = it.fixed {
            match groups.iter_mut().find(|g| g.id == id) {
                Some(g) => g.add(it),
                None => groups.push(Group::new(id, it, true)),
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
    let mut units: Vec<Vec<f32>> = groups.iter().map(Group::unit).collect();
    for i in order {
        let it = &items[i];
        let best = units
            .iter()
            .enumerate()
            .filter(|(k, _)| !groups[*k].photos.contains(&it.photo))
            .map(|(k, u)| (k, dot(u, &it.embedding.0)))
            .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)));
        match best {
            Some((k, s)) if s >= threshold => {
                groups[k].add(it);
                units[k] = groups[k].unit();
                label[i] = groups[k].id;
            }
            _ => {
                groups.push(Group::new(next, it, false));
                units.push(groups.last().unwrap().unit());
                label[i] = next;
                next += 1;
            }
        }
    }
    // Slå sammen grupper som ligner, de likeste først. Etter en sammenslåing er gjennomsnittet
    // nytt, så det gjøres i runder til ingenting mer kan slås sammen.
    loop {
        let units: Vec<Vec<f32>> = groups.iter().map(Group::unit).collect();
        let mut pairs: Vec<(f32, usize, usize)> = Vec::new();
        for a in 0..groups.len() {
            for b in a + 1..groups.len() {
                if !groups[a].alive
                    || !groups[b].alive
                    || groups[a].fixed && groups[b].fixed
                    || !groups[a].photos.is_disjoint(&groups[b].photos)
                {
                    continue;
                }
                let s = dot(&units[a], &units[b]);
                if s >= threshold {
                    pairs.push((s, a, b));
                }
            }
        }
        if pairs.is_empty() {
            break;
        }
        pairs.sort_by(|x, y| y.0.total_cmp(&x.0).then(x.1.cmp(&y.1)).then(x.2.cmp(&y.2)));
        let mut touched = vec![false; groups.len()];
        for (_, a, b) in pairs {
            if touched[a] || touched[b] {
                continue;
            }
            touched[a] = true;
            touched[b] = true;
            let (keep, gone) = if groups[b].fixed { (b, a) } else { (a, b) };
            let (kid, gid) = (groups[keep].id, groups[gone].id);
            let sum = std::mem::take(&mut groups[gone].sum);
            let photos = std::mem::take(&mut groups[gone].photos);
            groups[gone].alive = false;
            groups[keep]
                .sum
                .iter_mut()
                .zip(&sum)
                .for_each(|(x, y)| *x += y);
            groups[keep].photos.extend(photos);
            label
                .iter_mut()
                .filter(|l| **l == gid)
                .for_each(|l| *l = kid);
        }
        groups.retain(|g| g.alive);
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
            .enumerate()
            .map(|(i, e)| Item {
                embedding: e,
                quality: 1.0,
                fixed: None,
                photo: i as u64,
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
    fn samme_bilde_er_aldri_samme_person() {
        let es = [e(0, 0.0), e(0, 0.1), e(0, 0.2)];
        let mut it = items(&es);
        it[1].photo = 0;
        let l = cluster(&it, 0.6, 0);
        assert_ne!(l[0], l[1]);
        assert!(l[2] == l[0] || l[2] == l[1]);
    }

    #[test]
    fn ukjent_ansikt_finner_brukerens_person() {
        let es = [e(0, 0.0), e(0, 0.2)];
        let mut it = items(&es);
        it[0].fixed = Some(3);
        assert_eq!(cluster(&it, 0.6, 100), vec![3, 3]);
    }
}
