//! Hendelsesstyrt utkast: hele albumet foreslås før brukeren har valgt noe (PRODUCT.md, 3b).
//!
//! Hvert bilde fra året får en plass i forslaget, med eller ikke med, og en kort begrunnelse
//! (SCORING.md §9), så brukeren kan vurdere et komplett forslag og bytte der hun er uenig.
//!
//! 1. Hendelser: nytt treffpunkt når det går mer enn 3 t mellom bildene (`events`). Små
//!    hendelser (færre enn `event_min_photos` bilder) samles i «hverdager» per måned.
//! 2. Sider per hendelse ≈ 0,6 · √(antall bilder), store historier får stor plass (SCORING.md
//!    §4.2, §7). Ingen tak i utgangspunktet; velger brukeren et sidetak etter å ha sett
//!    forslaget, krymper alle historiene likt. Turer uten barn får 2–4 sider (myk maks).
//! 3. Serier (innen 20 s) og nesten like bilder (pHash) gir bare ett bilde hver.
//! 4. Skarphet måles mot resten av året (rangering), så «uskarpt» betyr uskarpt for denne
//!    familiens kamera og lys.
//! 5. Personer først: bilder med personer (foreløpig gjettet fra hudtoner) får et tillegg.
//!    Ting uten personer er med bare når de er et flott stemningsbilde, og da høyst ett per
//!    hendelse; på en tur der nesten ingen bilder har personer, er naturen selve historien.
//! 6. Innen hendelsen: grådig valg som veier kvalitet mot å vise flere deler av hendelsen.
//!    Vektene kommer fra det appen har lært (`learn::Preferences`).
//! 7. Brukerens egne valg (`Decision`) går alltid foran, også «fremhev» og «demp», sider hun
//!    er fornøyd med (beholdes som de er) og «presenter denne hendelsen på x sider»
//!    (`DraftHints`).
//!
//! Helhetsvurdering (SCORING.md §3.6): et ubrukelig bilde holdes ute, men er det det eneste
//! fra en hendelse, kommer det med og sier fra.

use std::collections::{HashMap, HashSet};

use crate::events::EVENT_GAP_SECONDS;
use crate::layout::{self, Page, PageKind, Single};
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
    /// Dager slås sammen til én tur når det går høyst så lenge mellom dem …
    pub trip_gap_seconds: i64,
    /// … og stedene er nærmere enn dette (km), og langt hjemmefra.
    pub trip_km: f64,
    /// Tur uten barn: myk maks for sider, som økes med ett per `adult_trip_extra_per`
    /// bilder over `adult_trip_extra_from`.
    pub adult_trip_pages: (usize, usize),
    pub adult_trip_extra_from: usize,
    pub adult_trip_extra_per: usize,
    /// Tur uten barn: så mange nærbilder av personer på portrettsiden.
    pub portrait_page_max: usize,
    pub unusable_sharp: f32,
    pub unusable_exposure: f32,
    /// Uskarpt: blant de svakeste `blurry_rank` av årets bilder og under `blurry_sharp`.
    pub blurry_rank: f32,
    pub blurry_sharp: f32,
    /// Andel hudtoner som regnes som «personer i bildet» (til M4).
    pub people_skin: f32,
    /// Tillegg i poeng for bilder med personer.
    pub people_bonus: f32,
    /// Hendelser der færre enn denne andelen har personer, er turer i naturen.
    pub nature_people_share: f32,
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
            trip_gap_seconds: 36 * 3600,
            trip_km: 30.0,
            adult_trip_pages: (2, 4),
            adult_trip_extra_from: 200,
            adult_trip_extra_per: 100,
            portrait_page_max: 9,
            unusable_sharp: 0.15,
            unusable_exposure: 0.1,
            blurry_rank: 0.2,
            blurry_sharp: 0.55,
            people_skin: 0.02,
            people_bonus: 0.3,
            nature_people_share: 0.25,
        }
    }
}

/// Brukerens eget valg for et bilde. Går foran algoritmen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Med,
    IkkeMed,
    /// Med, og på egen side.
    Fremhev,
    /// Med, men aldri alene på en side og lavere prioritet.
    Demp,
}

impl Decision {
    pub fn includes(self) -> bool {
        self != Decision::IkkeMed
    }
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
    /// Ingen personer, men et flott stemningsbilde.
    Stemningsbilde,
    /// På en side brukeren er fornøyd med.
    Fornoyd,
    // Ikke med
    ValgtBortAvDeg,
    SammeSerie {
        antall: usize,
    },
    NestenLikt,
    Uskarpt,
    MorktEllerUtbrent,
    Skjermbilde,
    /// Ingen personer, og ikke et spesielt flott bilde.
    Gjenstand,
    /// Et stemningsbilde herfra er allerede med.
    EnStemningHolder,
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
    /// Uskarpt sammenlignet med resten av året. Vises som et symbol.
    pub blurry: bool,
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
    /// Brukeren har sagt at dette er en tur uten barn.
    pub adult_trip: bool,
    /// Ser ut som en tur over flere dager: appen spør om den var uten barn.
    pub looks_like_trip: bool,
    /// Indekser (i `layout`) til sider brukeren er fornøyd med.
    pub locked_pages: Vec<usize>,
}

/// En side brukeren er fornøyd med. Beholdes som den er i neste revisjon.
#[derive(Debug, Clone, PartialEq)]
pub struct LockedPage {
    pub kind: PageKind,
    pub photos: Vec<ContentHash>,
}

/// Det brukeren har bestemt utover enkeltbilder. Hendelser er nøklet med et bilde i
/// hendelsen, så valgene overlever at utkastet lages på nytt.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DraftHints {
    /// Antall sider brukeren ønsker etter å ha sett forslaget, ned eller opp. Appen
    /// velger den skalaen for alle historiene som gir nærmest dette sidetallet.
    pub album_pages: Option<usize>,
    /// Hendelser som er turer uten barn (gutte- og jenteturer, jobbturer).
    pub adult_trips: HashSet<ContentHash>,
    /// «Presenter denne hendelsen på x sider.»
    pub page_targets: HashMap<ContentHash, usize>,
    /// Sider brukeren er fornøyd med.
    pub locked_pages: Vec<LockedPage>,
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
    /// `None` når bildet ikke er målt ennå.
    people: Option<bool>,
    mood: bool,
    /// Blant årets aller flotteste bilder (estetikk).
    exceptional: bool,
    blurry: bool,
    aesthetic: f32,
}

/// Lager utkastet. Med sidetak (`hints.page_cap`) krymper alle historiene likt, men hver
/// hendelse beholder minst én side. Deterministisk: samme bilder, valg og preferanser gir
/// samme utkast.
pub fn make_draft(
    photos: &[PhotoMeta],
    decisions: &HashMap<ContentHash, Decision>,
    prefs: &Preferences,
    cfg: &DraftConfig,
    hints: &DraftHints,
    progress: &mut dyn FnMut(Phase),
) -> Draft {
    progress(Phase::Hendelser);
    let dated: Vec<&PhotoMeta> = photos.iter().filter(|p| p.taken_at.is_some()).collect();
    let mut sharp_sorted: Vec<f32> = dated
        .iter()
        .filter_map(|p| p.quality.map(|q| q.sharp))
        .collect();
    sharp_sorted.sort_by(f32::total_cmp);
    let rank = |v: f32| {
        let below = sharp_sorted.partition_point(|&x| x < v);
        if sharp_sorted.len() > 1 {
            below as f32 / (sharp_sorted.len() - 1) as f32
        } else {
            0.5
        }
    };
    let mut items: Vec<Item> = dated
        .into_iter()
        .map(|p| {
            let raw = p.quality;
            let decision = decisions.get(&p.hash).copied();
            let rel = raw.map(|q| BasicQuality {
                sharp: rank(q.sharp),
                ..q
            });
            let people = raw.map(|q| q.skin >= cfg.people_skin);
            let mut q = rel.map_or(0.5, |q| prefs.quality(&q));
            if people == Some(true) {
                q += cfg.people_bonus;
            }
            if decision == Some(Decision::Demp) {
                q -= 0.15;
            }
            Item {
                p,
                t: p.taken_at.expect("har dato").local_seconds(),
                q,
                raw,
                burst: 0,
                noise: noise(p, cfg),
                decision,
                people,
                mood: false,
                exceptional: false,
                blurry: raw
                    .is_some_and(|r| rank(r.sharp) < cfg.blurry_rank && r.sharp < cfg.blurry_sharp),
                aesthetic: rel.map_or(0.0, |q| 0.4 * q.color + 0.3 * q.exposure + 0.3 * q.sharp),
            }
        })
        .collect();
    items.sort_by(|a, b| a.t.cmp(&b.t).then(a.p.hash.cmp(&b.p.hash)));
    if items.is_empty() {
        return Draft::default();
    }
    let mut groups = group_events(&items, cfg);
    let nature = mark_things(&mut items, &groups, prefs, cfg);
    let index: HashMap<ContentHash, usize> = items
        .iter()
        .enumerate()
        .map(|(i, it)| (it.p.hash, i))
        .collect();
    for g in &mut groups {
        g.adult_trip = g
            .members
            .iter()
            .any(|&i| hints.adult_trips.contains(&items[i].p.hash));
        g.target = g
            .members
            .iter()
            .find_map(|&i| hints.page_targets.get(&items[i].p.hash).copied());
    }
    // Sider brukeren er fornøyd med, som indekser, i den hendelsen de hører til.
    let mut locked_in: Vec<Vec<(PageKind, Vec<usize>)>> = vec![Vec::new(); groups.len()];
    let mut fixed = vec![false; items.len()];
    for lp in &hints.locked_pages {
        let idx: Vec<usize> = lp
            .photos
            .iter()
            .filter_map(|h| index.get(h).copied())
            .collect();
        let Some(&first) = idx.first() else { continue };
        if let Some(g) = groups.iter().position(|g| g.members.contains(&first)) {
            let idx: Vec<usize> = idx
                .into_iter()
                .filter(|i| groups[g].members.contains(i))
                .collect();
            idx.iter().for_each(|&i| fixed[i] = true);
            locked_in[g].push((lp.kind, idx));
        }
    }

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
    let scale = page_scale(&groups, cfg, hints.album_pages);
    let targets: Vec<usize> = groups.iter().map(|g| target_pages(g, cfg, scale)).collect();
    let mut left_pages = vec![0; groups.len()];
    for (g, group) in groups.iter().enumerate() {
        for (_, idx) in &locked_in[g] {
            idx.iter().for_each(|&i| included[i] = true);
        }
        let locked = locked_in[g].len();
        left_pages[g] = targets[g]
            .saturating_sub(locked)
            .max(usize::from(locked == 0));
        if left_pages[g] == 0 {
            continue;
        }
        let pool: Vec<usize> = group
            .members
            .iter()
            .copied()
            .filter(|&i| !fixed[i])
            .collect();
        if pool.is_empty() {
            continue;
        }
        let quota = quota(pool.len(), left_pages[g], scale, prefs, cfg);
        // Personer først: stemningsbilder er drypp. Høyst ett per hendelse (tre på turer uten
        // barn), og i naturen høyst fire, så ingen side blir fylt av bare ting.
        let mood_max = if nature[g] {
            4
        } else if group.adult_trip {
            3
        } else {
            1
        };
        let (picked, first, jumped) = pick(&pool, &items, quota, mood_max, prefs, cfg);
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
                .filter(|&i| included[i] && !fixed[i])
                .collect(),
        );
    }
    let events: Vec<DraftEvent> = groups
        .iter()
        .enumerate()
        .map(|(g, group)| {
            let chosen = &chosen_in[g];
            let mut pages: Vec<(bool, Page)> = locked_in[g]
                .iter()
                .map(|(kind, idx)| {
                    (
                        true,
                        Page {
                            kind: *kind,
                            photos: idx.clone(),
                        },
                    )
                })
                .collect();
            pages.extend(
                story_pages(&items, chosen, left_pages[g], group.adult_trip, cfg)
                    .into_iter()
                    .map(|p| (false, p)),
            );
            pages.sort_by_key(|(_, p)| p.photos.iter().min().copied().unwrap_or(usize::MAX));
            let locked_pages = pages
                .iter()
                .enumerate()
                .filter(|(_, (l, _))| *l)
                .map(|(k, _)| k)
                .collect();
            let layout: Vec<Page> = pages.into_iter().map(|(_, p)| p).collect();
            let days = (items[*group.members.last().expect("ikke tom")].t
                - items[group.members[0]].t)
                / 86_400
                + 1;
            DraftEvent {
                start: items[group.members[0]].p.taken_at.expect("har dato"),
                end: items[*group.members.last().expect("ikke tom")]
                    .p
                    .taken_at
                    .expect("har dato"),
                photos: group.members.len(),
                included: group.members.iter().filter(|&&i| included[i]).count(),
                pages: layout.len(),
                layout,
                everyday: group.everyday,
                adult_trip: group.adult_trip,
                looks_like_trip: !group.everyday && !nature[g] && days >= 2,
                locked_pages,
            }
        })
        .collect();

    let photos = items
        .iter()
        .enumerate()
        .map(|(i, it)| {
            let g = event_of[i];
            let chosen = &chosen_in[g];
            let (reason, related) = if fixed[i] {
                (Reason::Fornoyd, None)
            } else if included[i] {
                let first = first_pick[g] == Some(i);
                let r = reason_in(it, chosen.len(), &burst_size, first, out_of_order[i]);
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
                blurry: it.blurry,
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

/// Ting uten personer er med bare som flotte stemningsbilder. På turer der nesten ingen
/// bilder har personer, er naturen historien, og da holder det å være over middels.
fn mark_things(
    items: &mut [Item],
    groups: &[Group],
    prefs: &Preferences,
    cfg: &DraftConfig,
) -> Vec<bool> {
    let mut aes: Vec<f32> = items
        .iter()
        .filter(|it| it.raw.is_some())
        .map(|it| it.aesthetic)
        .collect();
    if aes.is_empty() {
        return vec![false; groups.len()];
    }
    aes.sort_by(f32::total_cmp);
    let at = |share: f32| aes[((aes.len() - 1) as f32 * share).round() as usize];
    let (mood_cut, median, top) = (at(prefs.mood_percentile), at(0.5), at(0.97));
    let mut natures = Vec::with_capacity(groups.len());
    for g in groups {
        let measured = g.members.iter().filter(|&&i| items[i].people.is_some());
        let with_people = measured.clone().filter(|&&i| items[i].people == Some(true));
        let nature =
            (with_people.count() as f32) < measured.count() as f32 * cfg.nature_people_share;
        natures.push(nature);
        for &i in &g.members {
            let it = &mut items[i];
            if it.people != Some(false) || it.noise.is_some() {
                continue;
            }
            if it.aesthetic >= if nature { median } else { mood_cut } {
                it.mood = true;
                it.exceptional = it.aesthetic >= top;
            } else {
                it.noise = Some(Reason::Gjenstand);
            }
        }
    }
    natures
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

/// Avstand i km mellom to punkter (breddegrad, lengdegrad).
fn km(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (la1, lo1, la2, lo2) = (
        a.0.to_radians(),
        a.1.to_radians(),
        b.0.to_radians(),
        b.1.to_radians(),
    );
    let h = ((la2 - la1) / 2.0).sin().powi(2)
        + la1.cos() * la2.cos() * ((lo2 - lo1) / 2.0).sin().powi(2);
    12_742.0 * h.sqrt().asin()
}

/// Midtpunktet (median) for bildene med GPS i en hendelse.
fn place(items: &[Item], a: usize, b: usize) -> Option<(f64, f64)> {
    let mut la: Vec<f64> = items[a..b]
        .iter()
        .filter_map(|it| it.p.gps.map(|g| g.0))
        .collect();
    let mut lo: Vec<f64> = items[a..b]
        .iter()
        .filter_map(|it| it.p.gps.map(|g| g.1))
        .collect();
    if la.is_empty() {
        return None;
    }
    la.sort_by(f64::total_cmp);
    lo.sort_by(f64::total_cmp);
    Some((la[la.len() / 2], lo[lo.len() / 2]))
}

/// Påfølgende dager på samme sted, langt hjemmefra, er én tur (SCORING.md §4.2). «Hjemme» er
/// stedet med flest bilder (ruter på ca. 10 km). Uten GPS slås ingenting sammen.
fn merge_trips(items: &[Item], raw: Vec<(usize, usize)>, cfg: &DraftConfig) -> Vec<(usize, usize)> {
    let mut cells: HashMap<(i32, i32), usize> = HashMap::new();
    for it in items {
        if let Some((la, lo)) = it.p.gps {
            *cells
                .entry(((la * 10.0).floor() as i32, (lo * 10.0).floor() as i32))
                .or_default() += 1;
        }
    }
    let Some((&(cla, clo), _)) = cells
        .iter()
        .max_by_key(|(c, n)| (**n, std::cmp::Reverse(**c)))
    else {
        return raw;
    };
    let home = ((cla as f64 + 0.5) / 10.0, (clo as f64 + 0.5) / 10.0);
    let mut out: Vec<(usize, usize)> = Vec::with_capacity(raw.len());
    for (a, b) in raw {
        if let Some(last) = out.last_mut() {
            let gap = items[a].t - items[last.1 - 1].t;
            if let (Some(p), Some(q)) = (place(items, last.0, last.1), place(items, a, b)) {
                if gap <= cfg.trip_gap_seconds
                    && km(p, q) <= cfg.trip_km
                    && km(p, home) > cfg.trip_km
                    && km(q, home) > cfg.trip_km
                {
                    last.1 = b;
                    continue;
                }
            }
        }
        out.push((a, b));
    }
    out
}

struct Group {
    /// Indekser i `items`, i tidsrekkefølge.
    members: Vec<usize>,
    everyday: bool,
    adult_trip: bool,
    /// Brukerens «presenter på x sider».
    target: Option<usize>,
}

/// Hendelser med minst `event_min_photos` bilder blir egne grupper; de små samles per
/// måned («hverdager»). Gruppene sorteres etter første bilde.
fn group_events(items: &[Item], cfg: &DraftConfig) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    let mut everyday: HashMap<(i32, u8), usize> = HashMap::new();
    for (a, b) in merge_trips(items, raw_events(items), cfg) {
        if b - a >= cfg.event_min_photos {
            groups.push(Group {
                members: (a..b).collect(),
                everyday: false,
                adult_trip: false,
                target: None,
            });
        } else {
            let t = items[a].p.taken_at.expect("har dato");
            let g = *everyday.entry((t.year, t.month)).or_insert_with(|| {
                groups.push(Group {
                    members: Vec::new(),
                    everyday: true,
                    adult_trip: false,
                    target: None,
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
    if let Some(t) = group.target {
        return t.max(1) as f32;
    }
    let n = group.members.len() as f32;
    if group.everyday {
        return (cfg.everyday_factor * n.sqrt() / cfg.photos_per_page).ceil();
    }
    let free = (cfg.pages_per_sqrt_photo * n.sqrt())
        .round()
        .clamp(1.0, cfg.event_max_pages as f32);
    if group.adult_trip {
        // Myk maks: brytes bare når det er svært mange bilder å ta av.
        let (lo, hi) = cfg.adult_trip_pages;
        let extra = group
            .members
            .len()
            .saturating_sub(cfg.adult_trip_extra_from)
            / cfg.adult_trip_extra_per;
        return free.clamp(lo as f32, (hi + extra) as f32);
    }
    free
}

/// Sidene for de valgte bildene. Stemningsbilder, ting og dempede bilder står aldri alene: de
/// setter omgivelsene mellom personbildene. Tur uten barn: én side med nærbilder av personene,
/// og resten med stemning rundt.
fn story_pages(
    items: &[Item],
    chosen: &[usize],
    pages: usize,
    adult_trip: bool,
    cfg: &DraftConfig,
) -> Vec<Page> {
    if chosen.is_empty() || pages == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut rest: Vec<usize> = chosen.to_vec();
    let mut pages = pages;
    if adult_trip && pages >= 2 {
        let mut faces: Vec<usize> = chosen
            .iter()
            .copied()
            .filter(|&i| items[i].people == Some(true))
            .collect();
        faces.sort_by(|&a, &b| {
            let skin = |i: usize| items[i].raw.map_or(0.0, |q| q.skin);
            skin(b).total_cmp(&skin(a)).then(a.cmp(&b))
        });
        faces.truncate(cfg.portrait_page_max);
        faces.sort();
        if faces.len() >= 3 {
            rest.retain(|i| !faces.contains(i));
            out.push(Page {
                kind: PageKind::Rutenett { kolonner: 3 },
                photos: faces,
            });
            pages -= 1;
        }
    }
    let quality: Vec<f32> = rest.iter().map(|&i| items[i].q).collect();
    let single: Vec<Single> = rest
        .iter()
        .map(|&i| match items[i].decision {
            Some(Decision::Fremhev) => Single::Must,
            Some(Decision::Demp) => Single::Never,
            // Et virkelig flott stemningsbilde (natur, oversikt) kan få en hel side innimellom.
            _ if items[i].mood && items[i].exceptional => Single::Free,
            // Ellers er ting og stemningsbilder fyll mellom personbildene.
            _ if items[i].people == Some(false) => Single::Never,
            _ => Single::Free,
        })
        .collect();
    out.extend(
        layout::story_with(&quality, &single, pages)
            .into_iter()
            .map(|p| Page {
                kind: p.kind,
                photos: p.photos.into_iter().map(|k| rest[k]).collect(),
            }),
    );
    out
}

fn target_pages(group: &Group, cfg: &DraftConfig, scale: f32) -> usize {
    let n = natural_pages(group, cfg) * scale;
    (if scale > 1.0 { n.round() } else { n.floor() } as usize).max(1)
}

/// Skalaen (0,2–3,0 i steg på 5 %) som gir nærmest ønsket antall sider, ned eller opp.
fn page_scale(groups: &[Group], cfg: &DraftConfig, wanted: Option<usize>) -> f32 {
    let Some(wanted) = wanted else { return 1.0 };
    (4..=60)
        .map(|k| k as f32 * 0.05)
        .min_by_key(|&s| {
            let pages: usize = groups.iter().map(|g| target_pages(g, cfg, s)).sum();
            pages.abs_diff(wanted)
        })
        .unwrap_or(1.0)
}

/// Hvor mange bilder en gruppe med `n` mulige bilder skal ha.
/// Flere sider enn forslaget (`scale` > 1) gir plass til en større andel av bildene.
fn quota(n: usize, pages: usize, scale: f32, prefs: &Preferences, cfg: &DraftConfig) -> usize {
    let want = pages as f32 * cfg.photos_per_page * prefs.event_density;
    let share = (cfg.max_share * scale.max(1.0)).min(0.9);
    let most = (n as f32 * share).ceil().max(1.0);
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
    mood_max: usize,
    prefs: &Preferences,
    cfg: &DraftConfig,
) -> (Vec<usize>, Option<usize>, Vec<usize>) {
    let mut chosen: Vec<usize> = members
        .iter()
        .copied()
        .filter(|&i| items[i].decision.is_some_and(Decision::includes))
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
    // Høyst ett stemningsbilde per hendelse (tre på turer uten barn), unntatt i naturen.
    let mut moods = chosen.iter().filter(|&&i| items[i].mood).count();

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
            .filter(|&i| !items[i].mood || moods < mood_max)
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
        if items[best].mood {
            moods += 1;
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
) -> Reason {
    if it.decision.is_some_and(Decision::includes) {
        return Reason::ValgtAvDeg;
    }
    let series = burst_size.get(&it.burst).copied().unwrap_or(1);
    let weak = it.noise.is_some() || it.blurry;
    if in_group == 1 {
        return if weak {
            Reason::SvaktMenEneste
        } else {
            Reason::EnesteFraHendelsen
        };
    }
    if it.mood && it.people == Some(false) {
        return Reason::Stemningsbilde;
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
    if it.blurry {
        return (Reason::Uskarpt, nearest);
    }
    if it.mood && chosen.iter().any(|&j| items[j].mood) {
        return (Reason::EnStemningHolder, nearest);
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
            skin: 0.1,
        });
        p
    }

    fn draft(photos: &[PhotoMeta], decisions: &HashMap<ContentHash, Decision>) -> Draft {
        make_draft(
            photos,
            decisions,
            &Preferences::default(),
            &DraftConfig::default(),
            &DraftHints::default(),
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
            &DraftHints::default(),
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
            &DraftHints {
                album_pages: Some(cap),
                ..DraftHints::default()
            },
            &mut |_| {},
        );
        assert!(small.pages() <= cap, "{} > {cap}", small.pages());
        assert_eq!(small.events.len(), full.events.len());
        assert!(small.events.iter().all(|e| e.pages >= 1));
    }

    fn thing(id: u16, h: u8, m: u8, color: f32) -> PhotoMeta {
        let mut p = photo(id, 4, 4, h, m, 0, 0.7);
        p.quality = Some(BasicQuality {
            sharp: 0.7,
            exposure: 0.8,
            color,
            skin: 0.0,
        });
        p
    }

    #[test]
    fn things_without_people_only_when_beautiful_and_one_per_event() {
        // En bursdag: 20 bilder med personer og 10 bilder av ting.
        let mut photos: Vec<PhotoMeta> = (0..20u16)
            .map(|k| photo(k, 4, 4, 12, k as u8 * 2, 0, 0.7))
            .collect();
        photos.extend((0..10u16).map(|k| thing(100 + k, 13, k as u8 * 3, 0.1 + k as f32 * 0.01)));
        // To svært fargerike stemningsbilder.
        photos.push(thing(200, 14, 0, 1.0));
        photos.push(thing(201, 14, 30, 1.0));
        let d = draft(&photos, &HashMap::new());
        let things: Vec<&DraftPhoto> = d
            .photos
            .iter()
            .filter(|p| {
                p.hash.0[0] == 0
                    && (100..202).contains(&u16::from_be_bytes([p.hash.0[0], p.hash.0[1]]))
            })
            .collect();
        assert!(
            things
                .iter()
                .filter(|p| p.reason == Reason::Gjenstand)
                .count()
                >= 9
        );
        let moods = d
            .included()
            .filter(|p| p.reason == Reason::Stemningsbilde)
            .count();
        assert_eq!(moods, 1, "høyst ett stemningsbilde per hendelse");
        assert!(d
            .photos
            .iter()
            .any(|p| p.reason == Reason::EnStemningHolder));
    }

    #[test]
    fn a_nature_trip_keeps_its_landscapes() {
        let photos: Vec<PhotoMeta> = (0..30u16)
            .map(|k| {
                thing(
                    k,
                    10 + (k / 12) as u8,
                    (k % 12) as u8 * 5,
                    0.3 + (k % 7) as f32 * 0.1,
                )
            })
            .collect();
        let d = draft(&photos, &HashMap::new());
        assert!(
            d.included().count() >= 4,
            "turen er historien: {}",
            d.included().count()
        );
    }

    #[test]
    fn no_cap_by_default_even_for_a_big_year() {
        // 40 store dager à 300 bilder: ingen tak på bilder eller sider i første utkast.
        let mut photos = Vec::new();
        for day in 0..40u16 {
            for k in 0..300u16 {
                let mut p = photo(
                    day * 300 + k,
                    1 + (day / 28) as u8,
                    1 + (day % 28) as u8,
                    8 + (k / 60) as u8,
                    (k % 60) as u8,
                    0,
                    0.5 + (k % 5) as f32 / 10.0,
                );
                p.hash.0[2] = day as u8;
                photos.push(p);
            }
        }
        let d = draft(&photos, &HashMap::new());
        assert!(d.included().count() > 500, "{}", d.included().count());
        assert_eq!(d.events.len(), 40);
    }

    fn hints(
        adult: &[ContentHash],
        target: Option<(ContentHash, usize)>,
        locked: Vec<LockedPage>,
    ) -> DraftHints {
        DraftHints {
            adult_trips: adult.iter().copied().collect(),
            page_targets: target.into_iter().collect(),
            locked_pages: locked,
            ..DraftHints::default()
        }
    }

    fn with(photos: &[PhotoMeta], h: &DraftHints) -> Draft {
        make_draft(
            photos,
            &HashMap::new(),
            &Preferences::default(),
            &DraftConfig::default(),
            h,
            &mut |_| {},
        )
    }

    /// En tur over tre dager med 300 bilder.
    fn trip() -> Vec<PhotoMeta> {
        let home = (1000..1400u16).map(|k| {
            let mut p = photo(
                k,
                3,
                1 + (k % 20) as u8,
                9 + (k % 9) as u8,
                (k % 60) as u8,
                0,
                0.6,
            );
            p.gps = Some((59.91, 10.75));
            p
        });
        (0..300u16)
            .map(|k| {
                let mut p = photo(
                    k,
                    6,
                    10 + (k / 100) as u8,
                    9 + ((k % 100) / 10) as u8,
                    (k % 10) as u8 * 5,
                    0,
                    0.5 + (k % 5) as f32 / 10.0,
                );
                if let Some(q) = p.quality.as_mut() {
                    q.skin = if k % 3 == 0 {
                        0.0
                    } else {
                        0.05 + (k % 7) as f32 / 50.0
                    };
                    q.color = if k % 3 == 0 { 0.9 } else { 0.5 };
                }
                p.gps = Some((61.1 + (k % 5) as f64 * 0.01, 8.5));
                p
            })
            .chain(home)
            .collect()
    }

    fn trip_event(d: &Draft) -> &DraftEvent {
        d.events.iter().find(|e| e.start.month == 6).expect("turen")
    }

    #[test]
    fn days_away_from_home_become_one_trip() {
        let d = draft(&trip(), &HashMap::new());
        let e = trip_event(&d);
        assert_eq!(e.photos, 300, "tre dager på samme sted er én tur");
        assert!(e.looks_like_trip);
    }

    #[test]
    fn adult_trip_gets_few_pages_and_a_portrait_page() {
        let photos = trip();
        let free = trip_event(&draft(&photos, &HashMap::new())).pages;
        let d = with(&photos, &hints(&[photos[0].hash], None, vec![]));
        let e = trip_event(&d);
        assert!(e.adult_trip);
        // 300 bilder: myk maks 4, pluss 1 fordi det er mange bilder å ta av.
        assert!(
            (2..=5).contains(&e.pages),
            "tur uten barn: {} sider (ellers {free})",
            e.pages
        );
        let first = &e.layout[0];
        assert!(first.photos.len() >= 3);
        assert!(first
            .photos
            .iter()
            .all(|&i| d.photos[i].reason != Reason::Stemningsbilde));
    }

    #[test]
    fn ordinary_mood_photos_never_stand_alone() {
        let d = draft(&trip(), &HashMap::new());
        for e in &d.events {
            for p in e.layout.iter().filter(|p| p.photos.len() == 1) {
                assert!(d.photos[p.photos[0]].reason != Reason::Stemningsbilde);
            }
        }
    }

    #[test]
    fn page_target_overrides_and_locked_pages_are_kept() {
        let photos = day(0, 9, 9, 120);
        let auto = draft(&photos, &HashMap::new());
        let d = with(&photos, &hints(&[], Some((photos[5].hash, 2)), vec![]));
        assert_eq!(d.pages(), 2, "auto hadde {}", auto.pages());

        let keep = &auto.events[0].layout[1];
        let locked = LockedPage {
            kind: keep.kind,
            photos: keep.photos.iter().map(|&i| auto.photos[i].hash).collect(),
        };
        let d = with(
            &photos,
            &hints(&[], Some((photos[5].hash, 3)), vec![locked.clone()]),
        );
        let e = &d.events[0];
        assert_eq!(e.pages, 3);
        assert_eq!(e.locked_pages.len(), 1);
        let page = &e.layout[e.locked_pages[0]];
        assert_eq!(
            page.photos
                .iter()
                .map(|&i| d.photos[i].hash)
                .collect::<Vec<_>>(),
            locked.photos
        );
        assert!(page
            .photos
            .iter()
            .all(|&i| d.photos[i].reason == Reason::Fornoyd));
    }

    #[test]
    fn blurry_is_measured_against_the_rest_of_the_year() {
        let mut photos = day(0, 5, 5, 30);
        let mut soft = photo(99, 5, 5, 12, 31, 0, 0.2);
        soft.quality = Some(BasicQuality {
            sharp: 0.2,
            exposure: 0.8,
            color: 0.5,
            skin: 0.1,
        });
        photos.push(soft.clone());
        let d = draft(&photos, &HashMap::new());
        assert!(get(&d, &soft).blurry);
        assert!(!get(&d, &photos[4]).blurry);
    }

    #[test]
    fn emphasized_photo_gets_own_page_dampened_never() {
        let photos = day(0, 8, 8, 60);
        let auto = draft(&photos, &HashMap::new());
        let weak = auto
            .photos
            .iter()
            .filter(|p| !p.included)
            .min_by(|a, b| a.quality.total_cmp(&b.quality))
            .unwrap()
            .hash;
        let best = auto
            .included()
            .max_by(|a, b| a.quality.total_cmp(&b.quality))
            .unwrap()
            .hash;
        let decisions = HashMap::from([(weak, Decision::Fremhev), (best, Decision::Demp)]);
        let d = draft(&photos, &decisions);
        let idx = |h: ContentHash| d.photos.iter().position(|p| p.hash == h).unwrap();
        let alone = |i: usize| {
            d.events
                .iter()
                .flat_map(|e| &e.layout)
                .any(|p| p.photos == vec![i])
        };
        assert!(get(&d, &photos[idx(weak)]).included);
        assert!(alone(idx(weak)));
        assert!(get(&d, &photos[idx(best)]).included);
        assert!(!alone(idx(best)));
    }

    #[test]
    fn more_pages_than_suggested_gives_more_photos() {
        let photos = day(0, 6, 6, 200);
        let auto = draft(&photos, &HashMap::new());
        let bigger = with(
            &photos,
            &DraftHints {
                album_pages: Some(auto.pages() * 2),
                ..DraftHints::default()
            },
        );
        assert!(
            bigger.pages() > auto.pages(),
            "{} > {}",
            bigger.pages(),
            auto.pages()
        );
        assert!(bigger.included().count() > auto.included().count());
    }
}
