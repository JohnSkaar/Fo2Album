//! Trykkfilen: albumet som PDF, laget på maskinen og lagret der brukeren velger.
//! Ingenting sendes noe sted (bestilling kommer senere).

use p2a_print::{album, AlbumText};
use p2a_store::Store;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::commands::{AppState, CmdResult, CommandError};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDto {
    title: String,
    subtitle: String,
    intro: String,
    back: String,
}

impl From<AlbumText> for TextDto {
    fn from(t: AlbumText) -> Self {
        TextDto {
            title: t.title,
            subtitle: t.subtitle,
            intro: t.intro,
            back: t.back,
        }
    }
}

impl From<TextDto> for AlbumText {
    fn from(t: TextDto) -> Self {
        AlbumText {
            title: t.title,
            subtitle: t.subtitle,
            intro: t.intro,
            back: t.back,
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PrintProgressDto {
    done: usize,
    total: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintDoneDto {
    pages: usize,
    photos: usize,
    missing: usize,
    low_resolution: usize,
    megabytes: f64,
}

/// Tekstene for årets album: det brukeren har skrevet, ellers forslaget
/// («Øyeblikk fra 2011» og navnene i familien).
#[tauri::command]
pub fn album_text(state: State<'_, AppState>, year: i32) -> CmdResult<TextDto> {
    let guard = state.store()?;
    Ok(album::load_text(guard.as_ref().expect("sjekket"), year)?.into())
}

/// Lagrer tekstene og lager trykkfilen for utkastet slik det er nå (med sidetaket brukeren
/// har valgt). Fremdrift sendes som hendelsen `trykk`.
#[tauri::command]
pub async fn export_album(
    app: AppHandle,
    year: i32,
    page_cap: Option<usize>,
    text: TextDto,
    path: String,
) -> CmdResult<PrintDoneDto> {
    use tauri::Manager;
    let (data_dir, keys) = {
        let state = app.state::<AppState>();
        drop(state.store()?); // må være åpnet
        (state.data_dir.clone(), state.keys.clone())
    };
    let text: AlbumText = text.into();
    tauri::async_runtime::spawn_blocking(move || {
        let store = Store::open(&data_dir, keys.as_ref())?;
        album::save_text(&store, year, &text)?;
        let (draft, metas) = album::draft_for_year(&store, year, page_cap)?;
        let a = album::from_draft(&store, year, &draft, &metas, text)?;
        let (pdf, report) = p2a_print::render(&a, &mut |done, total| {
            let _ = app.emit("trykk", PrintProgressDto { done, total });
        })
        .map_err(|e| CommandError::new("tomt_album", e.to_string()))?;
        std::fs::write(&path, &pdf)
            .map_err(|e| CommandError::new("kunne_ikke_lagre", e.to_string()))?;
        Ok(PrintDoneDto {
            pages: report.pages,
            photos: report.photos,
            missing: report.missing.len(),
            low_resolution: report.low_resolution.len(),
            megabytes: pdf.len() as f64 / 1e6,
        })
    })
    .await
    .map_err(|e| CommandError::new("ukjent", e.to_string()))?
}
