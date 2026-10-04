//! Kommandoene grensesnittet kan kalle. Tynt lag over `p2a-store` og `p2a-ingest`.
//!
//! Feil returneres som en stabil kode (`kode`) som grensesnittet oversetter til norsk tekst.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use p2a_core::config::DedupConfig;
use p2a_core::layout::PageKind;
use p2a_core::learn::{learn, Action, Feedback, FeedbackReason, Lesson, Preferences};
use p2a_core::select::draft::{make_draft, Decision, DraftConfig, Phase, Reason};
use p2a_core::{ContentHash, SourceKind};
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

// ---------- Utkast ----------

/// Fremdrift mens utkastet lages, sendt som hendelsen `analyse`.
#[derive(Debug, Clone, Serialize)]
pub struct AnalysePhaseDto {
    fase: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasonDto {
    kode: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    antall: Option<usize>,
}

impl From<Reason> for ReasonDto {
    fn from(r: Reason) -> Self {
        let (kode, antall) = match r {
            Reason::ValgtAvDeg => ("valgt_av_deg", None),
            Reason::BesteFraHendelsen => ("beste_fra_hendelsen", None),
            Reason::EnesteFraHendelsen => ("eneste_fra_hendelsen", None),
            Reason::SvaktMenEneste => ("svakt_men_eneste", None),
            Reason::BesteISerie { antall } => ("beste_i_serie", Some(antall)),
            Reason::AnnenDelAvHendelsen => ("annen_del_av_hendelsen", None),
            Reason::GodKvalitet => ("god_kvalitet", None),
            Reason::Stemningsbilde => ("stemningsbilde", None),
            Reason::Gjenstand => ("gjenstand", None),
            Reason::EnStemningHolder => ("en_stemning_holder", None),
            Reason::ValgtBortAvDeg => ("valgt_bort_av_deg", None),
            Reason::SammeSerie { antall } => ("samme_serie", Some(antall)),
            Reason::NestenLikt => ("nesten_likt", None),
            Reason::Uskarpt => ("uskarpt", None),
            Reason::MorktEllerUtbrent => ("morkt_eller_utbrent", None),
            Reason::Skjermbilde => ("skjermbilde", None),
            Reason::IkkePlass { med } => ("ikke_plass", Some(med)),
        };
        ReasonDto { kode, antall }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftPhotoDto {
    id: String,
    taken_at: String,
    event: usize,
    included: bool,
    reason: ReasonDto,
    /// Uskarpt sammenlignet med resten av året.
    blurry: bool,
    related: Option<String>,
    has_thumbnail: bool,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftEventDto {
    start: String,
    end: String,
    photos: usize,
    included: usize,
    pages: usize,
    everyday: bool,
    layout: Vec<PageDto>,
}

/// En side i historien. `kind` er helside (hele rammen), luft (ett bilde med marg) eller
/// rutenett.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDto {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    kolonner: Option<u8>,
    photos: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearnedDto {
    /// Det appen har lært, som koder grensesnittet gjør om til setninger.
    lessons: Vec<&'static str>,
    /// Antall bytter og antall svar på «Hvorfor?», alle år.
    choices: usize,
    answers: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftDto {
    year: i32,
    pages: usize,
    /// Sidene albumet får uten sidetak (hele historien), så prisvalgene kan vises.
    full_pages: usize,
    page_cap: Option<usize>,
    events: Vec<DraftEventDto>,
    photos: Vec<DraftPhotoDto>,
    learned: LearnedDto,
}

fn lesson_code(l: Lesson) -> &'static str {
    match l {
        Lesson::SkarphetTellerMer => "skarphet_teller_mer",
        Lesson::LysTellerMer => "lys_teller_mer",
        Lesson::FargerTellerMer => "farger_teller_mer",
        Lesson::OyeblikkFremforKvalitet => "oyeblikk_fremfor_kvalitet",
        Lesson::KvalitetFremforOyeblikk => "kvalitet_fremfor_oyeblikk",
        Lesson::FaerreLikeBilder => "faerre_like_bilder",
        Lesson::FlereFraHverHendelse => "flere_fra_hver_hendelse",
        Lesson::FaerreFraHverHendelse => "faerre_fra_hver_hendelse",
        Lesson::TingBareNaarFlotte => "ting_bare_naar_flotte",
        Lesson::LikerStemningsbilder => "liker_stemningsbilder",
    }
}

fn learned_from(log: &[Feedback]) -> (Preferences, LearnedDto) {
    let prefs = learn(log);
    let dto = LearnedDto {
        lessons: prefs.lessons().into_iter().map(lesson_code).collect(),
        choices: log.len(),
        answers: log.iter().filter(|f| f.reason.is_some()).count(),
    };
    (prefs, dto)
}

fn draft_for(
    store: &Store,
    year: i32,
    page_cap: Option<usize>,
    progress: &mut dyn FnMut(&'static str),
) -> Result<DraftDto, StoreError> {
    progress("henter");
    let metas = store.photo_metas_in_year(year)?;
    let summaries: HashMap<ContentHash, PhotoSummary> = store
        .photos_in_year(year)?
        .into_iter()
        .map(|p| (p.hash, p))
        .collect();
    let decisions = store.decisions(year)?;
    let (prefs, learned) = learned_from(&store.feedback_log()?);
    // Med sidetak regnes også hele historien ut, så brukeren ser hva den ville kostet.
    let full_pages = page_cap.map(|_| {
        make_draft(
            &metas,
            &decisions,
            &prefs,
            &DraftConfig::default(),
            None,
            &mut |_| {},
        )
        .pages()
    });
    let draft = make_draft(
        &metas,
        &decisions,
        &prefs,
        &DraftConfig::default(),
        page_cap,
        &mut |phase| {
            progress(match phase {
                Phase::Hendelser => "hendelser",
                Phase::Serier => "serier",
                Phase::Velger => "velger",
                Phase::Begrunnelser => "begrunnelser",
            })
        },
    );
    let id = |i: usize| draft.photos[i].hash.to_hex();
    Ok(DraftDto {
        year,
        pages: draft.pages(),
        full_pages: full_pages.unwrap_or_else(|| draft.pages()),
        page_cap,
        events: draft
            .events
            .iter()
            .map(|e| DraftEventDto {
                start: e.start.to_iso(),
                end: e.end.to_iso(),
                photos: e.photos,
                included: e.included,
                pages: e.pages,
                everyday: e.everyday,
                layout: e
                    .layout
                    .iter()
                    .map(|p| {
                        let (kind, kolonner) = match p.kind {
                            PageKind::Helside => ("helside", None),
                            PageKind::Luft => ("luft", None),
                            PageKind::Rutenett { kolonner } => ("rutenett", Some(kolonner)),
                        };
                        PageDto {
                            kind,
                            kolonner,
                            photos: p.photos.iter().map(|&i| id(i)).collect(),
                        }
                    })
                    .collect(),
            })
            .collect(),
        photos: draft
            .photos
            .into_iter()
            .map(|p| {
                let s = summaries.get(&p.hash);
                DraftPhotoDto {
                    id: p.hash.to_hex(),
                    taken_at: p.taken_at.to_iso(),
                    event: p.event,
                    included: p.included,
                    reason: p.reason.into(),
                    blurry: p.blurry,
                    related: p.related.map(|h| h.to_hex()),
                    has_thumbnail: s.is_some_and(|s| s.has_thumbnail),
                    width: s.and_then(|s| s.width),
                    height: s.and_then(|s| s.height),
                }
            })
            .collect(),
        learned,
    })
}

/// «Lag utkast»: venter til bildene er lest inn, går gjennom hele året og lager et komplett
/// forslag. Fremdrift sendes som hendelsen `analyse`.
#[tauri::command]
pub async fn make_album_draft(
    app: AppHandle,
    year: i32,
    page_cap: Option<usize>,
) -> CmdResult<DraftDto> {
    use tauri::Manager;
    let emit = {
        let app = app.clone();
        move |fase: &'static str| {
            let _ = app.emit("analyse", AnalysePhaseDto { fase });
        }
    };
    let (data_dir, keys) = {
        let state = app.state::<AppState>();
        drop(state.store()?); // må være åpnet
        (state.data_dir.clone(), state.keys.clone())
    };
    tauri::async_runtime::spawn_blocking(move || {
        let mut emit = emit;
        // Utkastet skal bygge på alle bildene, så det venter på innlesingen.
        let mut waited = false;
        while app.state::<AppState>().ingest_running() {
            if !waited {
                emit("venter");
                waited = true;
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        // Egen tilkobling, så grensesnittet kan vise miniatyrer mens analysen går.
        let store = Store::open(&data_dir, keys.as_ref())?;
        let draft = draft_for(&store, year, page_cap, &mut emit)?;
        emit("ferdig");
        Ok(draft)
    })
    .await
    .map_err(|e| CommandError::new("ukjent", e.to_string()))?
}

fn parse_id(id: &str) -> CmdResult<ContentHash> {
    ContentHash::from_hex(id).ok_or_else(|| CommandError::new("ukjent_bilde", id.to_string()))
}

/// Brukeren tar med, tar bort eller bytter et bilde. Lagrer valget og logger handlingen.
/// Returnerer id-en til loggføringen, så svaret på «Hvorfor?» kan legges til.
#[tauri::command]
pub fn choose_photo(
    state: State<'_, AppState>,
    year: i32,
    action: String,
    id: String,
    other: Option<String>,
) -> CmdResult<i64> {
    let action: Action = action
        .parse()
        .map_err(|_| CommandError::new("ukjent_handling", action.clone()))?;
    let hash = parse_id(&id)?;
    let other = other.as_deref().map(parse_id).transpose()?;
    let guard = state.store()?;
    let store = guard.as_ref().expect("sjekket");
    match (action, other) {
        (Action::TaMed, _) => store.set_decision(year, &hash, Some(Decision::Med))?,
        (Action::TaBort, _) => store.set_decision(year, &hash, Some(Decision::IkkeMed))?,
        (Action::Bytt, Some(out)) => {
            store.set_decision(year, &hash, Some(Decision::Med))?;
            store.set_decision(year, &out, Some(Decision::IkkeMed))?;
        }
        (Action::Bytt, None) => {
            return Err(CommandError::new("ukjent_handling", "bytte mangler bilde"))
        }
        (Action::Fremhev, _) => store.set_decision(year, &hash, Some(Decision::Fremhev))?,
        (Action::Demp, _) => store.set_decision(year, &hash, Some(Decision::Demp))?,
        // En dag tas bort bilde for bilde av grensesnittet; her logges bare handlingen.
        (Action::FjernDag, _) => {}
    }
    Ok(store.add_feedback(year, action, &hash, other.as_ref())?)
}

/// Svaret på «Hvorfor?». `None` betyr at brukeren hoppet over.
#[tauri::command]
pub fn answer_why(
    state: State<'_, AppState>,
    feedback_id: i64,
    reason: Option<String>,
) -> CmdResult<LearnedDto> {
    let reason = reason
        .map(|r| {
            r.parse::<FeedbackReason>()
                .map_err(|_| CommandError::new("ukjent_svar", r.clone()))
        })
        .transpose()?;
    let guard = state.store()?;
    let store = guard.as_ref().expect("sjekket");
    store.set_feedback_reason(feedback_id, reason)?;
    Ok(learned_from(&store.feedback_log()?).1)
}
