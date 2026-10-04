//! Sideoppsett for én historie (SCORING.md §7), første versjon.
//!
//! En historie (hendelse) fordeles på et gitt antall sider. De beste bildene får egen side,
//! og resten samles i rutenett. Enkeltbildene veksler mellom å fylle hele siden («helside»)
//! og å stå med luft rundt («luft»), så albumet puster. Store historier åpner med et
//! enkeltbilde, slik familiens egne album gjør (dåpen: kirken over to helsider, så rutenett).
//!
//! Kommer senere (M4–M6): portrettgallerier av gjestene til slutt i historien, oppslag-par,
//! beskjæring og oppløsningskrav per felt.

/// Hvordan en side er bygd opp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageKind {
    /// Ett bilde som fyller hele siden, uten marg.
    Helside,
    /// Ett bilde med luft rundt.
    Luft,
    /// Flere bilder i rutenett med så mange kolonner.
    Rutenett { kolonner: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub kind: PageKind,
    /// Indekser i bildene som ble gitt inn, i tidsrekkefølge.
    pub photos: Vec<usize>,
}

/// Andel av sidene som får ett bilde, når historien har flere sider.
const SINGLE_SHARE: f32 = 0.4;
/// Under denne kvaliteten (0–1) får bildet aldri helside; uskarphet synes mindre med luft.
const FULL_BLEED_MIN_QUALITY: f32 = 0.45;

/// Kolonner i et rutenett med `n` bilder.
fn columns(n: usize) -> u8 {
    match n {
        0..=2 => 2,
        3 => 3,
        4 => 2,
        5..=9 => 3,
        _ => 4,
    }
}

/// Om et bilde kan stå alene på en side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Single {
    Free,
    /// Brukeren har fremhevet bildet: egen side.
    Must,
    /// Dempet, eller en ting uten personer: aldri alene på en side.
    Never,
}

/// Fordeler bildene (i tidsrekkefølge, med kvalitet 0–1) på `pages` sider.
pub fn story(quality: &[f32], pages: usize) -> Vec<Page> {
    story_with(quality, &vec![Single::Free; quality.len()], pages)
}

/// Som `story`, men med brukerens og algoritmens krav om hvilke bilder som kan stå alene.
pub fn story_with(quality: &[f32], single: &[Single], pages: usize) -> Vec<Page> {
    let n = quality.len();
    if n == 0 {
        return Vec::new();
    }
    let must = single.iter().filter(|&&s| s == Single::Must).count();
    let pages = pages.max(must + usize::from(n > must)).clamp(1, n);
    if pages == 1 {
        let kind = if n == 1 {
            PageKind::Luft
        } else {
            PageKind::Rutenett {
                kolonner: columns(n),
            }
        };
        return vec![Page {
            kind,
            photos: (0..n).collect(),
        }];
    }

    // Antall enkeltbilder: rundt 40 %, men nok til at hvert rutenett får minst to bilder.
    let wanted = if pages == n {
        n
    } else {
        ((pages as f32 * SINGLE_SHARE).round() as usize)
            .max((2 * pages).saturating_sub(n))
            .clamp(1, pages - 1)
    }
    .max(must);
    let mut order: Vec<usize> = (0..n).filter(|&i| single[i] != Single::Never).collect();
    order.sort_by(|&a, &b| {
        (single[b] == Single::Must)
            .cmp(&(single[a] == Single::Must))
            .then(quality[b].total_cmp(&quality[a]))
            .then(a.cmp(&b))
    });
    let mut is_single = vec![false; n];
    for &i in order.iter().take(wanted) {
        is_single[i] = true;
    }
    let singles = is_single.iter().filter(|&&s| s).count();

    // Resten deles i like store rutenett, i tidsrekkefølge.
    let rest: Vec<usize> = (0..n).filter(|&i| !is_single[i]).collect();
    let grids = if rest.is_empty() {
        0
    } else {
        pages.saturating_sub(singles).max(1)
    };
    let mut out: Vec<Page> = (0..n)
        .filter(|&i| is_single[i])
        .map(|i| Page {
            kind: PageKind::Luft,
            photos: vec![i],
        })
        .collect();
    for g in 0..grids {
        let chunk: Vec<usize> = rest[g * rest.len() / grids..(g + 1) * rest.len() / grids].to_vec();
        if !chunk.is_empty() {
            out.push(Page {
                kind: PageKind::Rutenett {
                    kolonner: columns(chunk.len()),
                },
                photos: chunk,
            });
        }
    }
    out.sort_by_key(|p| p.photos[0]);

    // Store historier åpner med et enkeltbilde.
    if pages >= 3 {
        if let Some(first) = out.iter().position(|p| p.photos.len() == 1) {
            let page = out.remove(first);
            out.insert(0, page);
        }
    }

    // Enkeltbildene veksler mellom helside og luft, og åpningen fyller hele siden.
    let mut full = true;
    for p in out.iter_mut().filter(|p| p.photos.len() == 1) {
        let ok = quality[p.photos[0]] >= FULL_BLEED_MIN_QUALITY;
        p.kind = if full && ok {
            PageKind::Helside
        } else {
            PageKind::Luft
        };
        if ok {
            full = !full;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_photos(pages: &[Page]) -> Vec<usize> {
        let mut v: Vec<usize> = pages.iter().flat_map(|p| p.photos.clone()).collect();
        v.sort();
        v
    }

    #[test]
    fn uses_every_photo_once_on_the_given_pages() {
        let q: Vec<f32> = (0..46).map(|i| (i * 7 % 10) as f32 / 10.0).collect();
        let pages = story(&q, 10);
        assert_eq!(pages.len(), 10);
        assert_eq!(all_photos(&pages), (0..46).collect::<Vec<_>>());
    }

    #[test]
    fn big_story_opens_with_full_bleed_and_alternates() {
        let q: Vec<f32> = (0..40).map(|i| 0.5 + (i % 5) as f32 / 10.0).collect();
        let pages = story(&q, 10);
        assert_eq!(pages[0].kind, PageKind::Helside);
        let singles: Vec<PageKind> = pages
            .iter()
            .filter(|p| p.photos.len() == 1)
            .map(|p| p.kind)
            .collect();
        assert!(
            singles.contains(&PageKind::Luft),
            "noen enkeltbilder står med luft"
        );
        assert!(pages
            .iter()
            .any(|p| matches!(p.kind, PageKind::Rutenett { .. })));
    }

    #[test]
    fn blurry_photos_never_fill_the_page() {
        let q = vec![0.3, 0.2, 0.25];
        let pages = story(&q, 3);
        assert!(pages.iter().all(|p| p.kind == PageKind::Luft));
    }

    #[test]
    fn small_story_is_one_page() {
        assert_eq!(story(&[0.8], 1)[0].kind, PageKind::Luft);
        assert_eq!(
            story(&[0.8, 0.7, 0.6, 0.5], 1)[0].kind,
            PageKind::Rutenett { kolonner: 2 }
        );
        assert!(story(&[], 3).is_empty());
    }

    #[test]
    fn is_deterministic() {
        let q: Vec<f32> = (0..30).map(|i| ((i * 13) % 17) as f32 / 17.0).collect();
        assert_eq!(story(&q, 7), story(&q, 7));
    }

    #[test]
    fn emphasized_photos_get_their_own_page_and_things_never_do() {
        let q = vec![0.9, 0.8, 0.3, 0.7, 0.6, 0.5, 0.4, 0.95];
        let mut single = vec![Single::Free; 8];
        single[2] = Single::Must;
        single[7] = Single::Never;
        let pages = story_with(&q, &single, 4);
        let alone = |i: usize| pages.iter().any(|p| p.photos == vec![i]);
        assert!(alone(2), "fremhevet bilde står alene: {pages:?}");
        assert!(!alone(7), "en ting står aldri alene: {pages:?}");
        assert_eq!(all_photos(&pages), (0..8).collect::<Vec<_>>());
    }
}
