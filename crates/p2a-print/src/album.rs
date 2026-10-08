//! Fra utkastet og katalogen til et album som kan skrives ut.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use p2a_core::learn::learn;
use p2a_core::select::cover;
use p2a_core::select::draft::{make_draft, Draft, DraftConfig, DraftHints};
use p2a_core::{ContentHash, PhotoMeta, Role};
use p2a_store::{Store, StoreError};

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

/// Utkastet for året, slik appen lager det: brukerens valg, det appen har lært og personene.
pub fn draft_for_year(
    store: &Store,
    year: i32,
    album_pages: Option<usize>,
) -> Result<(Draft, Vec<PhotoMeta>), StoreError> {
    let metas = store.photo_metas_in_year(year)?;
    let hints = DraftHints {
        album_pages,
        ..people_hints(store, year)?
    };
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
pub fn default_covers(metas: &[PhotoMeta]) -> (Option<ContentHash>, Option<ContentHash>) {
    let c = cover::candidates(metas, &HashSet::new(), 2, 4);
    let front = c.people.first().or(c.overview.first()).copied();
    let back = c
        .overview
        .iter()
        .chain(&c.people)
        .find(|h| Some(**h) != front)
        .copied();
    (front, back)
}

fn source(files: &HashMap<ContentHash, p2a_store::PhotoFile>, h: &ContentHash) -> PhotoSource {
    match files.get(h) {
        Some(f) => PhotoSource {
            path: f.path.clone(),
            format: f.format.clone(),
            orientation: f.orientation,
            width: f.width.unwrap_or(0),
            height: f.height.unwrap_or(0),
        },
        // Ingen lokal fil (bare i skyen): blir et grått felt og står i rapporten.
        None => PhotoSource {
            path: PathBuf::from(format!("mangler-{}", &h.to_hex()[..12])),
            format: None,
            orientation: None,
            width: 0,
            height: 0,
        },
    }
}

/// Albumet slik utkastet viser det: alle sidene i alle historiene, i rekkefølge.
pub fn from_draft(
    store: &Store,
    year: i32,
    draft: &Draft,
    text: AlbumText,
    front: Option<ContentHash>,
    back: Option<ContentHash>,
) -> Result<Album, StoreError> {
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
            photos: p
                .photos
                .iter()
                .map(|&i| source(&files, &draft.photos[i].hash))
                .collect(),
        })
        .collect();
    Ok(Album {
        year,
        text,
        cover: front.map(|h| source(&files, &h)),
        back: back.map(|h| source(&files, &h)),
        pages,
    })
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
