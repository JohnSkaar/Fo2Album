//! Kommandoene grensesnittet kan kalle. Tynt lag over `p2a-store` og `p2a-ingest`.
//!
//! Feil returneres som en stabil kode (`kode`) som grensesnittet oversetter til norsk tekst.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use p2a_core::config::DedupConfig;
use p2a_core::SourceKind;
use p2a_ingest::pipeline::{ingest_all, IngestReport, Progress};
use p2a_ingest::sources;
use p2a_store::{CatalogSummary, PhotoSummary, RecoveryKey, Store, StoreError, VaultStatus};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::keys::PlatformKeyStore;

pub struct AppState {
    pub data_dir: PathBuf,
    pub keys: Arc<PlatformKeyStore>,
    pub store: Mutex<Option<Store>>,
    /// Satt mens innlesing pågår; `true` ber den stoppe.
    pub ingest_cancel: Mutex<Option<Arc<AtomicBool>>>,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        let keys = Arc::new(PlatformKeyStore::new(&data_dir));
        AppState {
            data_dir,
            keys,
            store: Mutex::new(None),
            ingest_cancel: Mutex::new(None),
        }
    }

    fn store(&self) -> Result<MutexGuard<'_, Option<Store>>, CommandError> {
        let guard = self.store.lock().unwrap_or_else(|e| e.into_inner());
        if guard.is_none() {
            return Err(CommandError::new("ikke_aapen", "lagringen er ikke åpnet"));
        }
        Ok(guard)
    }

    fn ingest_running(&self) -> bool {
        self.ingest_cancel
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }
}

#[derive(Debug, Serialize)]
pub struct CommandError {
    pub kode: &'static str,
    pub melding: String,
}

impl CommandError {
    fn new(kode: &'static str, melding: impl Into<String>) -> Self {
        CommandError {
            kode,
            melding: melding.into(),
        }
    }
}

impl From<StoreError> for CommandError {
    fn from(e: StoreError) -> Self {
        let kode = match &e {
            StoreError::NeedsRecovery => "trenger_gjenoppretting",
            StoreError::WrongKey => "feil_nokkel",
            StoreError::AlreadyExists(_) => "finnes_allerede",
            StoreError::NewerSchema(_) => "nyere_versjon",
            _ => "lagringsfeil",
        };
        CommandError::new(kode, e.to_string())
    }
}

type CmdResult<T> = Result<T, CommandError>;

// ---------- Lagring og nøkler ----------

#[tauri::command]
pub fn vault_status(state: State<'_, AppState>) -> CmdResult<&'static str> {
    if state
        .store
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some()
    {
        return Ok("klar");
    }
    Ok(match Store::status(&state.data_dir, state.keys.as_ref())? {
        VaultStatus::Empty => "tom",
        VaultStatus::NeedsRecovery => "trenger_gjenoppretting",
        VaultStatus::Ready => "laast",
    })
}

/// Første oppstart. Returnerer gjenopprettingsnøkkelen, som vises én gang.
#[tauri::command]
pub fn vault_create(state: State<'_, AppState>) -> CmdResult<String> {
    let (store, recovery) = Store::create(&state.data_dir, state.keys.as_ref())?;
    *state.store.lock().unwrap_or_else(|e| e.into_inner()) = Some(store);
    Ok(recovery.display_code().to_string())
}

#[tauri::command]
pub fn vault_open(state: State<'_, AppState>) -> CmdResult<()> {
    let store = Store::open(&state.data_dir, state.keys.as_ref())?;
    *state.store.lock().unwrap_or_else(|e| e.into_inner()) = Some(store);
    Ok(())
}

#[tauri::command]
pub fn vault_recover(state: State<'_, AppState>, kode: String) -> CmdResult<()> {
    let recovery =
        RecoveryKey::parse(&kode).map_err(|e| CommandError::new("ugyldig_kode", e.to_string()))?;
    let store = Store::recover(&state.data_dir, state.keys.as_ref(), &recovery)?;
    *state.store.lock().unwrap_or_else(|e| e.into_inner()) = Some(store);
    Ok(())
}

/// «Slett alle data». Bildene i kildemappene røres ikke.
#[tauri::command]
pub fn delete_all_data(state: State<'_, AppState>) -> CmdResult<()> {
    if state.ingest_running() {
        return Err(CommandError::new(
            "innlesing_pagar",
            "vent til innlesingen er ferdig",
        ));
    }
    *state.store.lock().unwrap_or_else(|e| e.into_inner()) = None;
    Store::delete_all(&state.data_dir, state.keys.as_ref())?;
    Ok(())
}

// ---------- Kilder ----------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDto {
    id: i64,
    kind: &'static str,
    path: String,
    label: String,
    finnes: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionDto {
    kind: &'static str,
    path: String,
    label: String,
}

fn label_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn list_sources(state: State<'_, AppState>) -> CmdResult<Vec<SourceDto>> {
    let guard = state.store()?;
    let store = guard.as_ref().expect("sjekket");
    Ok(store
        .sources()?
        .into_iter()
        .map(|s| SourceDto {
            id: s.id,
            kind: s.kind.as_str(),
            finnes: s.path.is_dir(),
            path: s.path.to_string_lossy().to_string(),
            label: s.label,
        })
        .collect())
}

/// Vanlige mapper som finnes på maskinen. Kalles bare når brukeren ber om forslag.
#[tauri::command]
pub fn suggested_sources(app: AppHandle) -> CmdResult<Vec<SuggestionDto>> {
    use tauri::Manager;
    let home = app
        .path()
        .home_dir()
        .map_err(|e| CommandError::new("ingen_hjemmemappe", e.to_string()))?;
    Ok(sources::suggested_sources(&home)
        .into_iter()
        .map(|s| SuggestionDto {
            kind: s.kind.as_str(),
            label: label_of(&s.path),
            path: s.path.to_string_lossy().to_string(),
        })
        .collect())
}

#[tauri::command]
pub fn add_source(
    state: State<'_, AppState>,
    path: String,
    kind: Option<String>,
) -> CmdResult<i64> {
    if state.ingest_running() {
        return Err(CommandError::new(
            "innlesing_pagar",
            "vent til innlesingen er ferdig",
        ));
    }
    let path = PathBuf::from(path);
    if !path.is_dir() {
        return Err(CommandError::new("ikke_mappe", "fant ikke mappen"));
    }
    // Stien vinner over brukerens valg av kort, hvis den tydelig hører til en kilde.
    let detected = sources::detect_kind(&path);
    let kind = match (detected, kind.and_then(|k| k.parse::<SourceKind>().ok())) {
        (SourceKind::Pc, Some(chosen)) => chosen,
        (detected, _) => detected,
    };
    let guard = state.store()?;
    Ok(guard
        .as_ref()
        .expect("sjekket")
        .add_source(kind, &path, &label_of(&path))?)
}

#[tauri::command]
pub fn remove_source(state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    if state.ingest_running() {
        return Err(CommandError::new(
            "innlesing_pagar",
            "vent til innlesingen er ferdig",
        ));
    }
    let mut guard = state.store()?;
    guard.as_mut().expect("sjekket").remove_source(id)?;
    Ok(())
}

// ---------- Innlesing ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestDoneDto {
    report: Option<IngestReport>,
    feil: Option<String>,
}

/// Starter innlesing i bakgrunnen. Fremdrift sendes som hendelsen `innlesing`, og
/// resultatet som `innlesing-ferdig`.
#[tauri::command]
pub fn start_ingest(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    let mut running = state
        .ingest_cancel
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if running.is_some() {
        return Ok(());
    }
    drop(state.store()?); // må være åpnet
    let cancel = Arc::new(AtomicBool::new(false));
    *running = Some(cancel.clone());
    drop(running);

    let data_dir = state.data_dir.clone();
    let keys = state.keys.clone();
    std::thread::spawn(move || {
        use tauri::Manager;
        // Egen tilkobling, så grensesnittet kan lese mens innlesingen skriver.
        let result = Store::open(&data_dir, keys.as_ref())
            .map_err(|e| e.to_string())
            .and_then(|mut store| {
                ingest_all(
                    &mut store,
                    &DedupConfig::default(),
                    &cancel,
                    &mut |p: Progress| {
                        let _ = app.emit("innlesing", p);
                    },
                )
                .map_err(|e| e.to_string())
            });
        let done = match result {
            Ok(report) => IngestDoneDto {
                report: Some(report),
                feil: None,
            },
            Err(e) => IngestDoneDto {
                report: None,
                feil: Some(e),
            },
        };
        let state = app.state::<AppState>();
        *state
            .ingest_cancel
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        let _ = app.emit("innlesing-ferdig", done);
    });
    Ok(())
}

#[tauri::command]
pub fn cancel_ingest(state: State<'_, AppState>) {
    if let Some(c) = state
        .ingest_cancel
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
    {
        c.store(true, Ordering::Relaxed);
    }
}

#[tauri::command]
pub fn ingest_running(state: State<'_, AppState>) -> bool {
    state.ingest_running()
}

// ---------- Katalog ----------

#[tauri::command]
pub fn catalog_summary(state: State<'_, AppState>) -> CmdResult<CatalogSummary> {
    let guard = state.store()?;
    Ok(guard.as_ref().expect("sjekket").summary()?)
}

#[derive(Debug, Serialize)]
pub struct YearDto {
    year: i32,
    count: usize,
}

#[tauri::command]
pub fn list_years(state: State<'_, AppState>) -> CmdResult<Vec<YearDto>> {
    let guard = state.store()?;
    Ok(guard
        .as_ref()
        .expect("sjekket")
        .years()?
        .into_iter()
        .map(|(year, count)| YearDto { year, count })
        .collect())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotoDto {
    id: String,
    taken_at: Option<String>,
    date_source: Option<&'static str>,
    width: Option<u32>,
    height: Option<u32>,
    has_thumbnail: bool,
    sources: Vec<&'static str>,
}

impl From<PhotoSummary> for PhotoDto {
    fn from(p: PhotoSummary) -> Self {
        PhotoDto {
            id: p.hash.to_hex(),
            taken_at: p.taken_at.map(|t| t.to_iso()),
            date_source: p.date_source.map(|d| d.as_str()),
            width: p.width,
            height: p.height,
            has_thumbnail: p.has_thumbnail,
            sources: p.sources.into_iter().map(|s| s.as_str()).collect(),
        }
    }
}

#[tauri::command]
pub fn photos_in_year(state: State<'_, AppState>, year: i32) -> CmdResult<Vec<PhotoDto>> {
    let guard = state.store()?;
    Ok(guard
        .as_ref()
        .expect("sjekket")
        .photos_in_year(year)?
        .into_iter()
        .map(PhotoDto::from)
        .collect())
}
