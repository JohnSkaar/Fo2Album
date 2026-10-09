//! «Hvem er med?»: gruppene av ansikter appen har funnet, og brukerens navn og roller.
//!
//! Ansiktsutsnittene vises fra miniatyrene (kryptert lagring) med boksen som andeler, så
//! ingen ansiktsbilder skrives til disk.

use p2a_core::Role;
use p2a_ingest::pipeline::group_faces;
use p2a_store::{FaceGroup, Store};
use serde::Serialize;
use tauri::State;

use crate::commands::{AppState, CmdResult, CommandError};

/// Grupper med færre ansikter enn dette vises ikke (tilfeldige folk i bakgrunnen).
const MIN_FACES: usize = 2;
/// Antall ansikter som vises per gruppe.
const SAMPLES: usize = 6;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FaceDto {
    face_id: i64,
    /// Bildet (hash), for miniatyren.
    id: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupDto {
    group: i64,
    person_id: Option<i64>,
    name: Option<String>,
    role: Option<&'static str>,
    faces: usize,
    photos: usize,
    samples: Vec<FaceDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonDto {
    id: i64,
    name: String,
    role: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeopleDto {
    groups: Vec<GroupDto>,
    persons: Vec<PersonDto>,
    /// Bilder der ansiktene ikke er funnet ennå (innlesingen tar dem).
    pending: usize,
}

fn people(store: &Store) -> CmdResult<PeopleDto> {
    let persons = store.persons()?;
    let groups = store
        .face_groups(MIN_FACES, SAMPLES)?
        .into_iter()
        .map(|g: FaceGroup| {
            let p = g
                .person_id
                .and_then(|id| persons.iter().find(|p| p.id == id));
            GroupDto {
                group: g.group,
                person_id: g.person_id,
                name: p.map(|p| p.name.clone()),
                role: p.map(|p| p.role.as_str()),
                faces: g.faces,
                photos: g.photos,
                samples: g
                    .samples
                    .into_iter()
                    .map(|s| FaceDto {
                        face_id: s.face_id,
                        id: s.hash.to_hex(),
                        x: s.x,
                        y: s.y,
                        w: s.w,
                        h: s.h,
                    })
                    .collect(),
            }
        })
        .collect();
    Ok(PeopleDto {
        groups,
        persons: persons
            .into_iter()
            .map(|p| PersonDto {
                id: p.id,
                name: p.name,
                role: p.role.as_str(),
            })
            .collect(),
        pending: store
            .photos_missing_faces(p2a_ingest::pipeline::face_model())?
            .len(),
    })
}

#[tauri::command]
pub fn face_groups(state: State<'_, AppState>) -> CmdResult<PeopleDto> {
    let guard = state.store()?;
    people(guard.as_ref().expect("sjekket"))
}

/// Gir en gruppe navn og rolle. Finnes personen fra før (`person_id`), slås gruppen inn i
/// den. Etterpå grupperes ansiktene på nytt, så like grupper uten navn finner personen.
#[tauri::command]
pub fn name_face_group(
    state: State<'_, AppState>,
    group: i64,
    name: String,
    role: String,
    person_id: Option<i64>,
) -> CmdResult<PeopleDto> {
    let role: Role = role
        .parse()
        .map_err(|_| CommandError::new("ukjent_rolle", role.clone()))?;
    let name = name.trim();
    if name.is_empty() && person_id.is_none() {
        return Err(CommandError::new("mangler_navn", ""));
    }
    let mut guard = state.store()?;
    let store = guard.as_mut().expect("sjekket");
    let id = match person_id {
        Some(id) => {
            if !name.is_empty() {
                store.update_person(id, name, role)?;
            }
            id
        }
        None => store.add_person(name, role, false, None)?,
    };
    store.name_face_group(group, id)?;
    group_faces(store)?;
    people(store)
}

/// «Ikke viktig»: gruppen teller ikke i albumet (fremmede, folk i bakgrunnen).
#[tauri::command]
pub fn ignore_face_group(state: State<'_, AppState>, group: i64) -> CmdResult<PeopleDto> {
    let mut guard = state.store()?;
    let store = guard.as_mut().expect("sjekket");
    store.ignore_face_group(group)?;
    people(store)
}

/// «Ikke denne personen»: ett ansikt tas ut av gruppen (eller flyttes til en annen person).
#[tauri::command]
pub fn move_face(
    state: State<'_, AppState>,
    face_id: i64,
    person_id: Option<i64>,
) -> CmdResult<PeopleDto> {
    let mut guard = state.store()?;
    let store = guard.as_mut().expect("sjekket");
    store.move_face(face_id, person_id)?;
    people(store)
}
