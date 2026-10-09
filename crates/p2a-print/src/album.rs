//! Fra utkastet og katalogen til et album som kan skrives ut.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use p2a_core::learn::learn;
use p2a_core::select::cover;
use p2a_core::select::draft::{make_draft, Draft, DraftConfig, DraftHints};
use p2a_core::{ContentHash, PhotoMeta, Role};
use p2a_store::{Store, StoreError};

use crate::choices::{self, AlbumChoices};
use crate::{Album, AlbumPage, AlbumText, PhotoSource};

/// Forslag til tittel: «Øyeblikk fra 2011».
pub fn default_title(year: i32) -> String {
    format!("Øyeblikk fra {year}")
}

/// «Kari, Ola og Emma».
pub fn name_line(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} og {last}", rest.join(", ")),
    }
}

/// Forslag til tekst: tittel med året og navnene i familien (barna og foreldrene) som
/// undertittel, i rekkefølgen de ble navngitt.
pub fn default_text(store: &Store, year: i32) -> Result<AlbumText, StoreError> {
    let names: Vec<String> = store
        .persons()?
        .into_iter()
        .filter(|p| matches!(p.role, Role::Kjernefamilie | Role::Barn))
        .map(|p| p.name)
        .collect();
    Ok(AlbumText {
        title: default_title(year),
        subtitle: name_line(&names),
        ..AlbumText::default()
    })
}

fn text_key(year: i32) -> String {
    format!("album_tekst:{year}")
}

/// Tekstene for årets album: det brukeren har lagret, ellers forslaget.
pub fn load_text(store: &Store, year: i32) -> Result<AlbumText, StoreError> {
    match store.setting(&text_key(year))? {
        Some(json) => Ok(serde_json::from_str(&json).unwrap_or(default_text(store, year)?)),
        None => default_text(store, year),
    }
}

/// Lagrer tekstene (kryptert, i familieprofilen).
pub fn save_text(store: &Store, year: i32, text: &AlbumText) -> Result<(), StoreError> {
    let json = serde_json::to_string(text).expect("tekst kan alltid gjøres om til JSON");
    store.set_setting(&text_key(year), &json)
}

/// Personene i årets bilder (ansiktene) og hvem som er barna, til utkastet (M4).
pub fn people_hints(store: &Store, year: i32) -> Result<DraftHints, StoreError> {
    Ok(DraftHints {
        faces: store.faces_in_year(year)?,
        children: store
            .persons()?
            .into_iter()
            .filter(|p| p.role == Role::Barn)
            .map(|p| p.id)
            .collect(),
        ..DraftHints::default()
    })
}

/// Alt utkastet trenger utover bildene: personene og brukerens valg for albumet.
pub fn hints_for(
    store: &Store,
    year: i32,
    album_pages: Option<usize>,
) -> Result<DraftHints, StoreError> {
    let base = DraftHints {
        album_pages,
        ..people_hints(store, year)?
    };
    Ok(choices::load(store, year)?.hints(base))
}

/// Utkastet for året, slik appen lager det: brukerens valg, det appen har lært og personene.
pub fn draft_for_year(
    store: &Store,
    year: i32,
    album_pages: Option<usize>,
) -> Result<(Draft, Vec<PhotoMeta>), StoreError> {
    let metas = store.photo_metas_in_year(year)?;
    let hints = hints_for(store, year, album_pages)?;
    let draft = make_draft(
        &metas,
        &store.decisions(year)?,
        &learn(&store.feedback_log()?),
        &DraftConfig::default(),
        &hints,
        &mut |_| {},
    );
    Ok((draft, metas))
}

/// Forside og bakside: brukerens valg, ellers appens forslag (et bilde med personer på
/// forsiden, et oversiktsbilde på baksiden).
pub fn covers(
    metas: &[PhotoMeta],
    chosen: &AlbumChoices,
) -> (Option<ContentHash>, Option<ContentHash>) {
    let c = cover::candidates(metas, &HashSet::new(), 2, 4);
    let pick = |s: &Option<String>| s.as_deref().and_then(ContentHash::from_hex);
    let front = pick(&chosen.cover).or(c.people.first().or(c.overview.first()).copied());
    let back = pick(&chosen.back).or(c
        .overview
        .iter()
        .chain(&c.people)
        .find(|h| Some(**h) != front)
        .copied());
    (front, back)
}

fn source(
    files: &HashMap<ContentHash, p2a_store::PhotoFile>,
    chosen: &AlbumChoices,
    h: &ContentHash,
) -> PhotoSource {
    match files.get(h) {
        Some(f) => PhotoSource {
            path: f.path.clone(),
            format: f.format.clone(),
            orientation: f.orientation,
            width: f.width.unwrap_or(0),
            height: f.height.unwrap_or(0),
            rotation: chosen.rotation_of(h),
            look: chosen.look_of(h),
            hash: Some(*h),
        },
        // Ingen lokal fil (bare i skyen): blir et grått felt og står i rapporten.
        None => PhotoSource {
            path: PathBuf::from(format!("mangler-{}", &h.to_hex()[..12])),
            format: None,
            orientation: None,
            width: 0,
            height: 0,
            rotation: 0,
            look: chosen.look_of(h),
            hash: Some(*h),
        },
    }
}

/// Albumet slik utkastet viser det: alle sidene i alle historiene, i rekkefølge, med
/// forsiden, baksiden, rammene, roteringen, størrelsene og utsnittene brukeren har valgt.
pub fn from_draft(
    store: &Store,
    year: i32,
    draft: &Draft,
    metas: &[PhotoMeta],
    text: AlbumText,
) -> Result<Album, StoreError> {
    let chosen = choices::load(store, year)?;
    let (front, back) = covers(metas, &chosen);
    let mut hashes: Vec<ContentHash> = draft
        .events
        .iter()
        .flat_map(|e| e.layout.iter().flat_map(|p| p.photos.iter()))
        .map(|&i| draft.photos[i].hash)
        .collect();
    hashes.extend(front.iter().chain(back.iter()));
    let files = store.photo_files(&hashes)?;
    let pages = draft
        .events
        .iter()
        .flat_map(|e| e.layout.iter())
        .filter(|p| !p.photos.is_empty())
        .map(|p| AlbumPage {
            kind: p.kind,
            mal: chosen
                .template_of(
                    &p.photos
                        .iter()
                        .map(|&i| draft.photos[i].hash)
                        .collect::<Vec<_>>(),
                )
                .map(str::to_string),
            photos: p
                .photos
                .iter()
                .map(|&i| source(&files, &chosen, &draft.photos[i].hash))
                .collect(),
        })
        .collect();
    Ok(Album {
        year,
        text,
        cover: front.map(|h| source(&files, &chosen, &h)),
        back: back.map(|h| source(&files, &chosen, &h)),
        pages,
    })
}

/// Hvor i albumet et funn i kvalitetssjekken står.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Cover,
    Back,
    /// Side `page` i historie `event` (indekser i utkastet).
    Page {
        event: usize,
        page: usize,
    },
}

/// Hva kvalitetssjekken sier om et bilde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckKind {
    LowResolution {
        ppi: u32,
    },
    Missing,
    /// Uskarpt (sammenlignet med resten av året), og står stort: alene på en side eller på
    /// omslaget.
    BlurryBig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Check {
    pub photo: ContentHash,
    pub place: Place,
    pub kind: CheckKind,
}

/// Kvalitetssjekk før trykk for utkastet: lav oppløsning og manglende filer (som trykkfilen
/// ville fått), og uskarpe bilder som står stort. Leser ikke bildene, så den er rask nok til
/// å kjøres hver gang utkastet lages.
pub fn checks(
    store: &Store,
    year: i32,
    draft: &Draft,
    metas: &[PhotoMeta],
) -> Result<Vec<Check>, StoreError> {
    let album = from_draft(store, year, draft, metas, AlbumText::default())?;
    // Sidene i albumet er utkastets sider i rekkefølge (uten tomme).
    let places: Vec<Place> = draft
        .events
        .iter()
        .enumerate()
        .flat_map(|(event, e)| {
            e.layout
                .iter()
                .enumerate()
                .filter(|(_, p)| !p.photos.is_empty())
                .map(move |(page, _)| Place::Page { event, page })
        })
        .collect();
    let mut out: Vec<Check> = crate::check(&album)
        .into_iter()
        .filter_map(|i| {
            let place = match i.spot {
                crate::Spot::Cover => Place::Cover,
                crate::Spot::Back => Place::Back,
                crate::Spot::Page(k) => *places.get(k)?,
            };
            let kind = match i.kind {
                crate::IssueKind::LowResolution { ppi } => CheckKind::LowResolution { ppi },
                crate::IssueKind::Missing => CheckKind::Missing,
            };
            Some(Check {
                photo: i.photo.hash?,
                place,
                kind,
            })
        })
        .collect();
    let blurry: HashSet<ContentHash> = draft
        .photos
        .iter()
        .filter(|p| p.blurry)
        .map(|p| p.hash)
        .collect();
    let mut big: Vec<(ContentHash, Place)> = Vec::new();
    for (event, e) in draft.events.iter().enumerate() {
        for (page, p) in e.layout.iter().enumerate() {
            if let [i] = p.photos.as_slice() {
                big.push((draft.photos[*i].hash, Place::Page { event, page }));
            }
        }
    }
    big.extend(
        album
            .cover
            .iter()
            .filter_map(|c| Some((c.hash?, Place::Cover))),
    );
    big.extend(
        album
            .back
            .iter()
            .filter_map(|c| Some((c.hash?, Place::Back))),
    );
    for (photo, place) in big {
        let already = out.iter().any(|c| c.photo == photo && c.place == place);
        if blurry.contains(&photo) && !already {
            out.push(Check {
                photo,
                place,
                kind: CheckKind::BlurryBig,
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navn_med_og() {
        let n = |v: &[&str]| name_line(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(n(&[]), "");
        assert_eq!(n(&["Kari"]), "Kari");
        assert_eq!(n(&["Kari", "Ola"]), "Kari og Ola");
        assert_eq!(n(&["Kari", "Ola", "Emma"]), "Kari, Ola og Emma");
        assert_eq!(default_title(2011), "Øyeblikk fra 2011");
    }
}
