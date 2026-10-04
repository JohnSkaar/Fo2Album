//! Hendelsesstyrt utkast: hele albumet foreslås før brukeren har valgt noe (PRODUCT.md, 3b).
//!
//! Hvert bilde fra året får en plass i forslaget, med eller ikke med, og en kort begrunnelse
//! (SCORING.md §9), så brukeren kan vurdere et komplett forslag og bytte der hun er uenig.
//!
//! 1. Hendelser: nytt treffpunkt når det går mer enn 3 t mellom bildene (`events`). Små
//!    hendelser (færre enn `event_min_photos` bilder) samles i «hverdager» per måned.
//! 2. Sider per hendelse: 1, 2 når det er tatt mange bilder, maks 4 (SCORING.md §4.2, §7).
//! 3. Serier (innen 20 s) og nesten like bilder (pHash) gir bare ett bilde hver.
//! 4. Innen hendelsen: grådig valg som veier kvalitet mot å vise flere deler av hendelsen.
//!    Vektene kommer fra det appen har lært (`learn::Preferences`).
//! 5. Brukerens egne valg (`Decision`) går alltid foran.
//!
//! Helhetsvurdering (SCORING.md §3.6): et ubrukelig bilde holdes ute, men er det det eneste
//! fra en hendelse, kommer det med og sier fra. Personer og estetikk kommer i M3–M5.

use std::collections::HashMap;

use crate::events::EVENT_GAP_SECONDS;
use crate::layout::{self, Page};
use crate::learn::Preferences;
use crate::{BasicQuality, ContentHash, PhotoMeta, TakenAt};

/// Parametre (SCORING.md §11). Startverdier, kalibreres mot evalueringssettet.
#[derive(Debug, Clone, PartialEq)]
pub struct DraftConfig {
    pub burst_seconds: i64,
    /// Nesten like bilder må være tatt innen så mange sekunder fra hverandre.
    pub similar_seconds: i64,
    pub event_min_photos: usize,
    /// Sider per hendelse ≈ faktor · √(antall bilder). Store historier får stor plass i
    /// første utkast (eierens føring); brukeren kan velge et mindre album (`page_cap`).
    pub pages_per_sqrt_photo: f32,
    pub event_max_pages: usize,
    pub photos_per_page: f32,
    /// Maks andel av en hendelses bilder som foreslås.
    pub max_share: f32,
    /// Hverdager: bilder ≈ faktor · √(antall bilder i måneden).
    pub everyday_factor: f32,
    pub unusable_sharp: f32,
    pub unusable_exposure: f32,
    pub blurry_sharp: f32,
}

impl Default for DraftConfig {
    fn default() -> Self {
        DraftConfig {
            burst_seconds: 20,
            similar_seconds: 10 * 60,
            event_min_photos: 3,
            pages_per_sqrt_photo: 0.6,
            event_max_pages: 16,
            photos_per_page: 4.0,
            max_share: 0.5,
            everyday_factor: 1.5,
            unusable_sharp: 0.15,
            unusable_exposure: 0.1,
            blurry_sharp: 0.35,
        }
    }
}

/// Brukerens eget valg for et bilde. Går foran algoritmen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Med,
    IkkeMed,
}

/// Hvorfor et bilde er med eller ikke. Grensesnittet gjør dem om til korte setninger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    // Med
    ValgtAvDeg,
    BesteFraHendelsen,
    EnesteFraHendelsen,
    /// Litt uskarpt eller mørkt, men det eneste fra hendelsen.
    SvaktMenEneste,
    BesteISerie {
        antall: usize,
    },
    AnnenDelAvHendelsen,
    GodKvalitet,
    // Ikke med
    ValgtBortAvDeg,
    SammeSerie {
        antall: usize,
    },
    NestenLikt,
    Uskarpt,
    MorktEllerUtbrent,
    Skjermbilde,
    /// Det er ikke plass til flere fra hendelsen; `med` bilder herfra er med.
    IkkePlass {
        med: usize,
    },
}

/// Hvor langt utkastet har kommet, til fremdriftsvisningen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Hendelser,
    Serier,
    Velger,
    Begrunnelser,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DraftPhoto {
    pub hash: ContentHash,
    pub taken_at: TakenAt,
    /// Indeks i `Draft::events`.
    pub event: usize,
    pub included: bool,
    pub reason: Reason,
    /// Kvalitet 0–1 med de lærte vektene.
    pub quality: f32,
    /// Bildet dette henger sammen med: det valgte i samme serie, det det ligner, eller
    /// nærmeste valgte bilde i tid. Grensesnittet foreslår det som bytte.
    pub related: Option<ContentHash>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DraftEvent {
    pub start: TakenAt,
    pub end: TakenAt,
    pub photos: usize,
    pub included: usize,
    pub pages: usize,
    /// Sidene historien får, med bildene i `Draft::photos` som indekser.
    pub layout: Vec<Page>,
    /// Små hendelser i en måned, samlet («hverdager»).
    pub everyday: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Draft {
    pub events: Vec<DraftEvent>,
    /// Alle bilder med dato, i tidsrekkefølge.
    pub photos: Vec<DraftPhoto>,
}

impl Draft {
    pub fn included(&self) -> impl Iterator<Item = &DraftPhoto> {
        self.photos.iter().filter(|p| p.included)
    }

    pub fn pages(&self) -> usize {
        self.events.iter().map(|e| e.pages).sum()
    }
}

/// Bilde under arbeid.
struct Item<'a> {
    p: &'a PhotoMeta,
    t: i64,
    q: f32,
    raw: Option<BasicQuality>,
    burst: usize,
    noise: Option<Reason>,
    decision: Option<Decision>,
}

/// Lager utkastet. `page_cap` er brukerens valg av et mindre album: da krymper alle
/// historiene likt, men hver hendelse beholder minst én side. Deterministisk: samme bilder,
/// valg og preferanser gir samme utkast.
pub fn make_draft(
    photos: &[PhotoMeta],
    decisions: &HashMap<ContentHash, Decision>,
    prefs: &Preferences,
    cfg: &DraftConfig,
    page_cap: Option<usize>,
    progress: &mut dyn FnMut(Phase),
) -> Draft {
    progress(Phase::Hendelser);
    let mut items: Vec<Item> = photos
        .iter()
        .filter_map(|p| {
            let t = p.taken_at?.local_seconds();
            let raw = p.quality;
            Some(Item {
                p,
                t,
                q: raw.map_or(0.5, |q| prefs.quality(&q)),
                raw,
                burst: 0,
                noise: noise(p, cfg),
                decision: decisions.get(&p.hash).copied(),
            })
        })
        .collect();
    items.sort_by(|a, b| a.t.cmp(&b.t).then(a.p.hash.cmp(&b.p.hash)));
    if items.is_empty() {
        return Draft::default();
    }
    let groups = group_events(&items, cfg);

    progress(Phase::Serier);
    let mut burst = 0;
    for i in 0..items.len() {
        if i > 0 && items[i].t - items[i - 1].t > cfg.burst_seconds {
            burst += 1;
        }
        items[i].burst = burst;
    }
    let mut burst_size: HashMap<usize, usize> = HashMap::new();
    for it in &items {
        *burst_size.entry(it.burst).or_default() += 1;
    }

    progress(Phase::Velger);
    let mut included = vec![false; items.len()];
    let mut first_pick: Vec<Option<usize>> = vec![None; groups.len()];
    let mut out_of_order = vec![false; items.len()];
    let scale = page_scale(&groups, cfg, page_cap);
    let targets: Vec<usize> = groups.iter().map(|g| target_pages(g, cfg, scale)).collect();
    for (g, group) in groups.iter().enumerate() {
        let quota = quota(group, targets[g], prefs, cfg);
        let (picked, first, jumped) = pick(&group.members, &items, quota, prefs, cfg);
        for &i in &picked {
            included[i] = true;
        }
        for i in jumped {
            out_of_order[i] = true;
        }
        first_pick[g] = first;
    }

    progress(Phase::Begrunnelser);
    let mut event_of = vec![0; items.len()];
    let mut chosen_in: Vec<Vec<usize>> = Vec::with_capacity(groups.len());
    for (g, group) in groups.iter().enumerate() {
        for &i in &group.members {
            event_of[i] = g;
        }
        chosen_in.push(
            group
                .members
                .iter()
                .copied()
                .filter(|&i| included[i])
                .collect(),
        );
    }
    let events: Vec<DraftEvent> = groups
        .iter()
        .zip(&chosen_in)
        .zip(&targets)
        .map(|((group, chosen), &target)| {
            let quality: Vec<f32> = chosen.iter().map(|&i| items[i].q).collect();
            let layout: Vec<Page> = layout::story(&quality, target)
                .into_iter()
                .map(|p| Page {
                    kind: p.kind,
                    photos: p.photos.into_iter().map(|k| chosen[k]).collect(),
                })
                .collect();
            DraftEvent {
                start: items[group.members[0]].p.taken_at.expect("har dato"),
                end: items[*group.members.last().expect("ikke tom")]
                    .p
                    .taken_at
                    .expect("har dato"),
                photos: group.members.len(),
                included: chosen.len(),
                pages: layout.len(),
                layout,
                everyday: group.everyday,
            }
        })
        .collect();

    let photos = items
        .iter()
        .enumerate()
        .map(|(i, it)| {
            let g = event_of[i];
            let chosen = &chosen_in[g];
            let (reason, related) = if included[i] {
                let first = first_pick[g] == Some(i);
                let r = reason_in(it, chosen.len(), &burst_size, first, out_of_order[i], cfg);
                (r, None)
            } else {
                reason_out(it, &items, chosen, &burst_size, prefs, cfg)
            };
            DraftPhoto {
                hash: it.p.hash,
                taken_at: it.p.taken_at.expect("har dato"),
                event: g,
                included: included[i],
                reason,
                quality: it.q,
                related,
            }
        })
        .collect();
    Draft { events, photos }
}

/// Skjermbilder og bilder som ikke kan brukes, holdes utenfor forslaget.
fn noise(p: &PhotoMeta, cfg: &DraftConfig) -> Option<Reason> {
    if p.format.as_deref() == Some("png") && p.camera_make.is_none() {
        return Some(Reason::Skjermbilde);
    }
    let q = p.quality?;
    if q.exposure < cfg.unusable_exposure {
        Some(Reason::MorktEllerUtbrent)
    } else if q.sharp < cfg.unusable_sharp {
        Some(Reason::Uskarpt)
    } else {
        None
    }
}

/// Rå hendelser (tidsgap) i tidsrekkefølge: (start, slutt) i `items`, slutt eksklusiv.
fn raw_events(items: &[Item]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0;
    for i in 1..=items.len() {
        if i == items.len() || items[i].t - items[i - 1].t > EVENT_GAP_SECONDS {
            out.push((start, i));
            start = i;
        }
    }
    out
}

struct Group {
    /// Indekser i `items`, i tidsrekkefølge.
    members: Vec<usize>,
    everyday: bool,
}

/// Hendelser med minst `event_min_photos` bilder blir egne grupper; de små samles per
/// måned («hverdager»). Gruppene sorteres etter første bilde.
fn group_events(items: &[Item], cfg: &DraftConfig) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    let mut everyday: HashMap<(i32, u8), usize> = HashMap::new();
    for (a, b) in raw_events(items) {
        if b - a >= cfg.event_min_photos {
            groups.push(Group {
                members: (a..b).collect(),
                everyday: false,
            });
        } else {
            let t = items[a].p.taken_at.expect("har dato");
            let g = *everyday.entry((t.year, t.month)).or_insert_with(|| {
                groups.push(Group {
                    members: Vec::new(),
                    everyday: true,
                });
                groups.len() - 1
            });
            groups[g].members.extend(a..b);
        }
    }
    groups.sort_by_key(|g| g.members[0]);
    groups
}

fn usable(it: &Item) -> bool {
    it.noise.is_none() && it.decision != Some(Decision::IkkeMed)
}

/// Sider hendelsen får før et eventuelt sidetak.
fn natural_pages(group: &Group, cfg: &DraftConfig) -> f32 {
    let n = group.members.len() as f32;
    if group.everyday {
        (cfg.everyday_factor * n.sqrt() / cfg.photos_per_page).ceil()
    } else {
        (cfg.pages_per_sqrt_photo * n.sqrt())
            .round()
            .clamp(1.0, cfg.event_max_pages as f32)
    }
}

fn target_pages(group: &Group, cfg: &DraftConfig, scale: f32) -> usize {
    ((natural_pages(group, cfg) * scale).floor() as usize).max(1)
}

/// Største skala (i steg på 5 %) som holder albumet innenfor sidetaket.
fn page_scale(groups: &[Group], cfg: &DraftConfig, cap: Option<usize>) -> f32 {
    let Some(cap) = cap else { return 1.0 };
    (0..=19)
        .map(|k| 1.0 - k as f32 * 0.05)
        .find(|&s| {
            groups
                .iter()
                .map(|g| target_pages(g, cfg, s))
                .sum::<usize>()
                <= cap
        })
        .unwrap_or(0.05)
}

/// Hvor mange bilder gruppen skal ha.
fn quota(group: &Group, pages: usize, prefs: &Preferences, cfg: &DraftConfig) -> usize {
    let n = group.members.len();
    let want = pages as f32 * cfg.photos_per_page * prefs.event_density;
    let most = (n as f32 * cfg.max_share).ceil().max(1.0);
    (want.min(most).round() as usize).max(1)
}

fn similar(a: &Item, b: &Item, prefs: &Preferences, cfg: &DraftConfig) -> bool {
    match (a.p.phash, b.p.phash) {
        (Some(x), Some(y)) => {
            (a.t - b.t).abs() <= cfg.similar_seconds
                && ((x ^ y).count_ones() as f32) <= prefs.similar_hamming
        }
        _ => false,
    }
}

/// Velger bilder i én gruppe. Returnerer valgte, det første automatiske valget, og valg der
/// et bedre bilde ble hoppet over for å vise en annen del av hendelsen.
fn pick(
    members: &[usize],
    items: &[Item],
    quota: usize,
    prefs: &Preferences,
    cfg: &DraftConfig,
) -> (Vec<usize>, Option<usize>, Vec<usize>) {
    let mut chosen: Vec<usize> = members
        .iter()
        .copied()
        .filter(|&i| items[i].decision == Some(Decision::Med))
        .collect();
    let mut cands: Vec<usize> = members
        .iter()
        .copied()
        .filter(|&i| usable(&items[i]) && items[i].decision.is_none())
        .collect();
    // Helhetsvurdering: finnes ingenting brukbart, er det beste av resten bedre enn ingenting.
    let mut quota = quota;
    if chosen.is_empty() && cands.is_empty() {
        quota = 1;
        cands = members
            .iter()
            .copied()
            .filter(|&i| items[i].decision.is_none() && items[i].noise != Some(Reason::Skjermbilde))
            .collect();
    }
    let span = (items[*members.last().expect("ikke tom")].t - items[members[0]].t).max(1);
    let step = (span as f32 / (quota as f32 + 1.0)).max(60.0);
    let imp = prefs.quality_importance;
    let mut first = None;
    let mut jumped = Vec::new();

    while chosen.len() < quota {
        let blocked = |i: usize, chosen: &[usize]| {
            chosen.iter().any(|&j| {
                items[j].burst == items[i].burst || similar(&items[i], &items[j], prefs, cfg)
            })
        };
        let open: Vec<usize> = cands
            .iter()
            .copied()
            .filter(|&i| !chosen.contains(&i) && !blocked(i, &chosen))
            .collect();
        let value = |i: usize| {
            let spread = chosen
                .iter()
                .map(|&j| (items[i].t - items[j].t).abs())
                .min()
                .map_or(1.0, |d| (d as f32 / step).min(1.0));
            imp * items[i].q + (1.0 - imp) * spread
        };
        let Some(best) = open.iter().copied().max_by(|&a, &b| {
            value(a)
                .total_cmp(&value(b))
                .then(items[b].t.cmp(&items[a].t))
                .then(items[b].p.hash.cmp(&items[a].p.hash))
        }) else {
            break;
        };
        let top_quality = open.iter().map(|&i| items[i].q).fold(f32::MIN, f32::max);
        if items[best].q + 1e-6 < top_quality {
            jumped.push(best);
        }
        if first.is_none() {
            first = Some(best);
        }
        chosen.push(best);
    }
    (chosen, first, jumped)
}

fn reason_in(
    it: &Item,
    in_group: usize,
    burst_size: &HashMap<usize, usize>,
    first: bool,
    jumped: bool,
    cfg: &DraftConfig,
) -> Reason {
    if it.decision == Some(Decision::Med) {
        return Reason::ValgtAvDeg;
    }
    let series = burst_size.get(&it.burst).copied().unwrap_or(1);
    let weak = it.noise.is_some() || it.raw.is_some_and(|q| q.sharp < cfg.blurry_sharp);
    if in_group == 1 {
        return if weak {
            Reason::SvaktMenEneste
        } else {
            Reason::EnesteFraHendelsen
        };
    }
    if series > 1 {
        return Reason::BesteISerie { antall: series };
    }
    if first {
        return Reason::BesteFraHendelsen;
    }
    if jumped {
        Reason::AnnenDelAvHendelsen
    } else {
        Reason::GodKvalitet
    }
}

fn reason_out(
    it: &Item,
    items: &[Item],
    chosen: &[usize],
    burst_size: &HashMap<usize, usize>,
    prefs: &Preferences,
    cfg: &DraftConfig,
) -> (Reason, Option<ContentHash>) {
    let nearest = chosen
        .iter()
        .copied()
        .min_by_key(|&j| ((items[j].t - it.t).abs(), j))
        .map(|j| items[j].p.hash);
    if it.decision == Some(Decision::IkkeMed) {
        return (Reason::ValgtBortAvDeg, nearest);
    }
    if let Some(r) = it.noise {
        return (r, nearest);
    }
    if let Some(&j) = chosen.iter().find(|&&j| items[j].burst == it.burst) {
        let antall = burst_size.get(&it.burst).copied().unwrap_or(1);
        return (Reason::SammeSerie { antall }, Some(items[j].p.hash));
    }
    if let Some(&j) = chosen.iter().find(|&&j| similar(it, &items[j], prefs, cfg)) {
        return (Reason::NestenLikt, Some(items[j].p.hash));
    }
    if it.raw.is_some_and(|q| q.sharp < cfg.blurry_sharp) {
        return (Reason::Uskarpt, nearest);
    }
    (Reason::IkkePlass { med: chosen.len() }, nearest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BasicQuality, DateSource};

    fn photo(id: u16, month: u8, day: u8, h: u8, m: u8, s: u8, sharp: f32) -> PhotoMeta {
        let mut hash = [0u8; 32];
        hash[..2].copy_from_slice(&id.to_be_bytes());
        let mut p = PhotoMeta::new(ContentHash(hash));
        p.taken_at = TakenAt::new(2010, month, day, h, m, s);
        p.date_source = Some(DateSource::Exif);
        p.format = Some("jpeg".into());
        p.camera_make = Some("Apple".into());
        // Ulike pHash som standard, så bildene ikke regnes som like.
        p.phash = Some(u64::from(id).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        p.quality = Some(BasicQuality {
            sharp,
            exposure: 0.8,
            color: 0.5,
        });
        p
    }

    fn draft(photos: &[PhotoMeta], decisions: &HashMap<ContentHash, Decision>) -> Draft {
        make_draft(
            photos,
            decisions,
            &Preferences::default(),
            &DraftConfig::default(),
            None,
            &mut |_| {},
        )
    }

    fn get<'a>(d: &'a Draft, p: &PhotoMeta) -> &'a DraftPhoto {
        d.photos.iter().find(|x| x.hash == p.hash).expect("finnes")
    }

    /// En dag med `n` bilder, ett hvert 5. minutt fra kl. 10.
    fn day(first_id: u16, month: u8, d: u8, n: u16) -> Vec<PhotoMeta> {
        (0..n)
            .map(|k| {
                let min = k as u32 * 5;
                photo(
                    first_id + k,
                    month,
                    d,
                    10 + (min / 60) as u8,
                    (min % 60) as u8,
                    0,
                    0.4 + (k % 5) as f32 / 10.0,
                )
            })
            .collect()
    }

    #[test]
    fn every_photo_gets_a_reason_and_every_event_is_included() {
        let mut photos = day(0, 3, 4, 30);
        photos.extend(day(100, 6, 20, 60));
        photos.extend(day(200, 12, 24, 5));
        let d = draft(&photos, &HashMap::new());
        assert_eq!(d.photos.len(), photos.len());
        assert_eq!(d.events.len(), 3);
        assert!(d.events.iter().all(|e| e.included >= 1 && e.pages >= 1));
        // 60 bilder gir to sider, 30 bilder én.
        assert!(d.events[1].included > d.events[0].included);
    }

    #[test]
    fn is_deterministic() {
        let mut photos = day(0, 3, 4, 40);
        photos.extend(day(100, 7, 1, 12));
        let a = draft(&photos, &HashMap::new());
        photos.reverse();
        assert_eq!(a, draft(&photos, &HashMap::new()));
    }

    #[test]
    fn keeps_only_best_of_a_series_and_says_so() {
        let mut photos = vec![
            photo(1, 7, 14, 12, 0, 0, 0.4),
            photo(2, 7, 14, 12, 0, 5, 0.9),
            photo(3, 7, 14, 12, 0, 10, 0.5),
        ];
        photos.extend(day(10, 7, 14, 6).into_iter().map(|mut p| {
            let t = p.taken_at.unwrap();
            p.taken_at = TakenAt::new(2010, 7, 14, t.hour + 3, t.minute, 0);
            p
        }));
        let d = draft(&photos, &HashMap::new());
        assert!(get(&d, &photos[1]).included);
        assert_eq!(
            get(&d, &photos[1]).reason,
            Reason::BesteISerie { antall: 3 }
        );
        let worse = get(&d, &photos[0]);
        assert!(!worse.included);
        assert_eq!(worse.reason, Reason::SammeSerie { antall: 3 });
        assert_eq!(worse.related, Some(photos[1].hash));
    }

    #[test]
    fn near_duplicates_are_excluded_with_reason() {
        let mut photos = day(0, 5, 17, 3);
        let mut twin = photo(50, 5, 17, 10, 1, 0, 0.5);
        twin.phash = photos[0].phash.map(|h| h ^ 0b11);
        photos.push(twin.clone());
        let d = draft(&photos, &HashMap::new());
        let t = get(&d, &twin);
        if !t.included {
            assert!(matches!(
                t.reason,
                Reason::NestenLikt | Reason::IkkePlass { .. }
            ));
        }
        assert!(!(get(&d, &photos[0]).included && t.included));
    }

    #[test]
    fn only_photo_of_an_event_is_kept_even_if_blurry() {
        let mut photos = day(0, 2, 1, 10);
        photos.extend([
            photo(90, 9, 9, 12, 0, 0, 0.1),
            photo(91, 9, 9, 12, 30, 0, 0.12),
            photo(92, 9, 9, 13, 0, 0, 0.11),
        ]);
        let d = draft(&photos, &HashMap::new());
        let sept: Vec<_> = d.photos.iter().filter(|p| p.taken_at.month == 9).collect();
        assert_eq!(sept.iter().filter(|p| p.included).count(), 1);
        let kept = sept.iter().find(|p| p.included).unwrap();
        assert_eq!(kept.reason, Reason::SvaktMenEneste);
    }

    #[test]
    fn small_events_become_everyday_per_month() {
        let photos = vec![
            photo(1, 4, 2, 9, 0, 0, 0.7),
            photo(2, 4, 9, 9, 0, 0, 0.7),
            photo(3, 4, 20, 9, 0, 0, 0.7),
            photo(4, 4, 28, 9, 0, 0, 0.7),
        ];
        let d = draft(&photos, &HashMap::new());
        assert_eq!(d.events.len(), 1);
        assert!(d.events[0].everyday);
        assert!(d.events[0].included >= 2);
    }

    #[test]
    fn user_decisions_win() {
        let photos = day(0, 8, 8, 30);
        let auto = draft(&photos, &HashMap::new());
        let out = auto.included().next().unwrap().hash;
        let inn = auto.photos.iter().find(|p| !p.included).unwrap().hash;
        let decisions = HashMap::from([(out, Decision::IkkeMed), (inn, Decision::Med)]);
        let d = draft(&photos, &decisions);
        let o = d.photos.iter().find(|p| p.hash == out).unwrap();
        let i = d.photos.iter().find(|p| p.hash == inn).unwrap();
        assert!(!o.included && o.reason == Reason::ValgtBortAvDeg);
        assert!(i.included && i.reason == Reason::ValgtAvDeg);
        assert_eq!(
            d.included().count(),
            auto.included().count(),
            "et bytte endrer ikke antallet"
        );
    }

    #[test]
    fn screenshots_are_left_out() {
        let mut photos = day(0, 1, 5, 5);
        let mut shot = photo(77, 1, 5, 10, 2, 30, 0.95);
        shot.format = Some("png".into());
        shot.camera_make = None;
        photos.push(shot.clone());
        let d = draft(&photos, &HashMap::new());
        assert_eq!(get(&d, &shot).reason, Reason::Skjermbilde);
    }

    #[test]
    fn learned_density_changes_amount() {
        let photos = day(0, 6, 6, 80);
        let base = draft(&photos, &HashMap::new()).included().count();
        let prefs = Preferences {
            event_density: 1.5,
            ..Preferences::default()
        };
        let more = make_draft(
            &photos,
            &HashMap::new(),
            &prefs,
            &DraftConfig::default(),
            None,
            &mut |_| {},
        )
        .included()
        .count();
        assert!(more > base);
    }

    #[test]
    fn big_story_gets_more_than_four_pages() {
        // En dåp med 300 bilder: historien får stor plass i første utkast.
        let photos: Vec<PhotoMeta> = (0..300u16)
            .map(|k| {
                photo(
                    k,
                    6,
                    5,
                    11 + (k / 60) as u8,
                    (k % 60) as u8,
                    0,
                    0.5 + (k % 5) as f32 / 10.0,
                )
            })
            .collect();
        let d = draft(&photos, &HashMap::new());
        assert_eq!(d.events.len(), 1);
        assert!(d.events[0].pages > 4, "sider: {}", d.events[0].pages);
        let in_layout: usize = d.events[0].layout.iter().map(|p| p.photos.len()).sum();
        assert_eq!(in_layout, d.events[0].included);
    }

    #[test]
    fn page_cap_shrinks_album_but_keeps_every_event() {
        let mut photos: Vec<PhotoMeta> = (0..300u16)
            .map(|k| photo(k, 6, 5, 11 + (k / 60) as u8, (k % 60) as u8, 0, 0.6))
            .collect();
        photos.extend(day(1000, 2, 2, 20));
        photos.extend(day(2000, 9, 9, 8));
        let full = draft(&photos, &HashMap::new());
        let cap = full.pages() - 4;
        let small = make_draft(
            &photos,
            &HashMap::new(),
            &Preferences::default(),
            &DraftConfig::default(),
            Some(cap),
            &mut |_| {},
        );
        assert!(small.pages() <= cap, "{} > {cap}", small.pages());
        assert_eq!(small.events.len(), full.events.len());
        assert!(small.events.iter().all(|e| e.pages >= 1));
    }
}
