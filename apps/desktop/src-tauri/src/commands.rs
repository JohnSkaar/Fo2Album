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
use p2a_core::select::draft::{make_draft, Decision, DraftConfig, DraftHints, Phase, Reason};
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
    /// Endringer i utkastet som kan angres.
    pub undo: crate::undo::History,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        let keys = Arc::new(PlatformKeyStore::new(&data_dir));
        AppState {
            data_dir,
            keys,
            store: Mutex::new(None),
            ingest_cancel: Mutex::new(None),
            undo: Default::default(),
        }
    }

    pub(crate) fn store(&self) -> Result<MutexGuard<'_, Option<Store>>, CommandError> {
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
    pub(crate) fn new(kode: &'static str, melding: impl Into<String>) -> Self {
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

pub(crate) type CmdResult<T> = Result<T, CommandError>;

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
            Reason::Fornoyd => ("fornoyd", None),
            Reason::AlleErMed => ("alle_er_med", None),
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
    /// Brukerens rotering med klokka (0, 90, 180, 270).
    rotation: u16,
    /// Brukerens størrelse: -1 mindre, 0 like stort, 1–2 større, 3 egen side, 4 hele siden.
    size: Option<i8>,
    /// Utsnittet i en ramme som fylles: midtpunktet som andeler (0–1).
    focus: [f32; 2],
    /// Hele bildet vises i rammen.
    whole: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftEventDto {
    /// Et bilde i hendelsen, som nøkkel for valgene (tur, sider, slå sammen).
    key: String,
    /// Dager brukeren har slått sammen.
    merged: bool,
    /// Brukerens «presenter på x sider».
    page_target: Option<usize>,
    /// Brukeren har svart på om dette var en tur uten barn.
    trip_answered: bool,
    start: String,
    end: String,
    photos: usize,
    included: usize,
    pages: usize,
    everyday: bool,
    /// Ser ut som en tur over flere dager; appen kan spørre om den var uten barn.
    looks_like_trip: bool,
    adult_trip: bool,
    /// Appen så selv at ingen av barna er med (M4).
    adult_trip_guess: bool,
    /// Brukeren har laget historien selv («Egen historie»).
    own_story: bool,
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
    /// Brukeren er fornøyd med siden; den beholdes i neste utkast.
    locked: bool,
    /// Faste rammer brukeren har valgt.
    mal: Option<String>,
    photos: Vec<String>,
    /// Hvor bildene står, i prosent av den ferdige siden (samme som trykkfilen).
    frames: Vec<FrameDto>,
}

/// Et felt på siden i prosent av bredden og høyden. Kan gå litt utenfor (utfallende kant).
#[derive(Debug, Serialize)]
pub struct FrameDto {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    /// Bildet fyller feltet (beskjæres); ellers har feltet bildets form.
    fill: bool,
}

impl From<p2a_print::layout::Frame> for FrameDto {
    fn from(f: p2a_print::layout::Frame) -> Self {
        use p2a_print::layout::{TRIM_H, TRIM_W};
        let r = |v: f32| (v * 1000.0).round() / 1000.0;
        FrameDto {
            x: r(f.x / TRIM_W * 100.0),
            y: r(f.y / TRIM_H * 100.0),
            w: r(f.w / TRIM_W * 100.0),
            h: r(f.h / TRIM_H * 100.0),
            fill: f.fill,
        }
    }
}

/// En fast ramme brukeren kan velge for en side.
#[derive(Debug, Serialize)]
pub struct TemplateDto {
    id: &'static str,
    photos: usize,
    frames: Vec<FrameDto>,
}

/// Rammene brukeren kan velge for en side, med feltene til forhåndsvisning.
#[tauri::command]
pub fn page_templates() -> Vec<TemplateDto> {
    p2a_print::layout::TEMPLATES
        .iter()
        .map(|&(id, photos)| TemplateDto {
            id,
            photos,
            frames: p2a_print::layout::template_frames(id)
                .unwrap_or_default()
                .into_iter()
                .map(FrameDto::from)
                .collect(),
        })
        .collect()
}

/// Forside og bakside: det som brukes nå, og forslagene.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverDto {
    front: Option<String>,
    back: Option<String>,
    /// Brukeren har valgt selv (ellers er det appens forslag).
    chosen_front: bool,
    chosen_back: bool,
    people: Vec<String>,
    overview: Vec<String>,
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
    /// Forslag om å slå sammen korte dager tett etter hverandre (indekser i `events`).
    merge_suggestions: Vec<Vec<usize>>,
    pages: usize,
    /// Sidene albumet får uten sidetak (hele historien), så prisvalgene kan vises.
    full_pages: usize,
    page_cap: Option<usize>,
    cover: CoverDto,
    events: Vec<DraftEventDto>,
    photos: Vec<DraftPhotoDto>,
    learned: LearnedDto,
    /// Kvalitetssjekk før trykk.
    checks: Vec<CheckDto>,
}

/// Et funn i kvalitetssjekken. `kind`: lav_opplosning, mangler eller uskarpt_stort.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckDto {
    photo: String,
    kind: &'static str,
    /// Oppløsningen bildet får i albumet (ved lav oppløsning).
    ppi: Option<u32>,
    /// forside, bakside eller side.
    place: &'static str,
    event: Option<usize>,
    page: Option<usize>,
}

impl From<p2a_print::album::Check> for CheckDto {
    fn from(c: p2a_print::album::Check) -> Self {
        use p2a_print::album::{CheckKind, Place};
        let (kind, ppi) = match c.kind {
            CheckKind::LowResolution { ppi } => ("lav_opplosning", Some(ppi)),
            CheckKind::Missing => ("mangler", None),
            CheckKind::BlurryBig => ("uskarpt_stort", None),
        };
        let (place, event, page) = match c.place {
            Place::Cover => ("forside", None, None),
            Place::Back => ("bakside", None, None),
            Place::Page { event, page } => ("side", Some(event), Some(page)),
        };
        CheckDto {
            photo: c.photo.to_hex(),
            kind,
            ppi,
            place,
            event,
            page,
        }
    }
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
    // Personene (M4) og brukerens valg for albumet: de samme som trykkfilen bruker.
    let people = p2a_print::album::hints_for(store, year, None)?;
    let chosen = p2a_print::choices::load(store, year)?;
    // Med sidetak regnes også hele historien ut, så brukeren ser hva den ville kostet.
    let full_pages = page_cap.map(|_| {
        make_draft(
            &metas,
            &decisions,
            &prefs,
            &DraftConfig::default(),
            &people,
            &mut |_| {},
        )
        .pages()
    });
    let draft = make_draft(
        &metas,
        &decisions,
        &prefs,
        &DraftConfig::default(),
        &DraftHints {
            album_pages: page_cap,
            ..people.clone()
        },
        &mut |phase| {
            progress(match phase {
                Phase::Hendelser => "hendelser",
                Phase::Serier => "serier",
                Phase::Velger => "velger",
                Phase::Begrunnelser => "begrunnelser",
            })
        },
    );
    let checks = p2a_print::album::checks(store, year, &draft, &metas)?
        .into_iter()
        .map(CheckDto::from)
        .collect();
    let id = |i: usize| draft.photos[i].hash.to_hex();
    // Formen på bildet slik det vises (etter rotering), til feltene på sidene.
    let aspect = |h: &ContentHash| {
        let a = summaries
            .get(h)
            .and_then(|s| Some(s.width? as f32 / s.height?.max(1) as f32))
            .filter(|a| *a > 0.0)
            .unwrap_or(4.0 / 3.0);
        if chosen.rotation_of(h) % 180 == 90 {
            1.0 / a
        } else {
            a
        }
    };
    Ok(DraftDto {
        year,
        merge_suggestions: draft.merge_suggestions.clone(),
        pages: draft.pages(),
        full_pages: full_pages.unwrap_or_else(|| draft.pages()),
        page_cap,
        cover: {
            let (front, back) = p2a_print::album::covers(&metas, &chosen);
            let c = p2a_core::select::cover::candidates(&metas, &Default::default(), 3, 5);
            let hex = |v: Vec<ContentHash>| v.into_iter().map(|h| h.to_hex()).collect();
            CoverDto {
                front: front.map(|h| h.to_hex()),
                back: back.map(|h| h.to_hex()),
                chosen_front: chosen.cover.is_some(),
                chosen_back: chosen.back.is_some(),
                people: hex(c.people),
                overview: hex(c.overview),
            }
        },
        events: draft
            .events
            .iter()
            .enumerate()
            .map(|(k, e)| {
                // En hendelse er nøklet med det første bildet i den (for valgene).
                let key = draft
                    .photos
                    .iter()
                    .find(|p| p.event == k)
                    .map(|p| p.hash.to_hex())
                    .unwrap_or_default();
                let ids: Vec<String> = draft
                    .photos
                    .iter()
                    .filter(|p| p.event == k)
                    .map(|p| p.hash.to_hex())
                    .collect();
                let merged = chosen
                    .merged
                    .iter()
                    .any(|m| m.iter().any(|id| ids.contains(id)));
                let page_target = chosen
                    .page_targets
                    .iter()
                    .find(|(p, _)| ids.contains(p))
                    .map(|(_, n)| *n);
                let trip_answered = chosen
                    .adult_trips
                    .iter()
                    .chain(&chosen.family_trips)
                    .any(|p| ids.contains(p));
                let own_story = ids.iter().any(|id| chosen.in_story(id));
                (key, merged, page_target, trip_answered, own_story, e)
            })
            .map(
                |(key, merged, page_target, trip_answered, own_story, e)| DraftEventDto {
                    own_story,
                    key,
                    merged,
                    page_target,
                    trip_answered,
                    start: e.start.to_iso(),
                    end: e.end.to_iso(),
                    photos: e.photos,
                    included: e.included,
                    pages: e.pages,
                    everyday: e.everyday,
                    looks_like_trip: e.looks_like_trip,
                    adult_trip: e.adult_trip,
                    adult_trip_guess: e.adult_trip_guess,
                    layout: e
                        .layout
                        .iter()
                        .enumerate()
                        .map(|(n, p)| {
                            let (kind, kolonner) = match p.kind {
                                PageKind::Helside => ("helside", None),
                                PageKind::Luft => ("luft", None),
                                PageKind::Rutenett { kolonner } => ("rutenett", Some(kolonner)),
                            };
                            let hashes: Vec<ContentHash> =
                                p.photos.iter().map(|&i| draft.photos[i].hash).collect();
                            let mal = chosen.template_of(&hashes).map(str::to_string);
                            let looks: Vec<(f32, p2a_print::layout::Look)> = hashes
                                .iter()
                                .map(|h| (aspect(h), chosen.look_of(h)))
                                .collect();
                            let frames =
                                p2a_print::layout::page_frames(p.kind, mal.as_deref(), &looks)
                                    .into_iter()
                                    .map(FrameDto::from)
                                    .collect();
                            PageDto {
                                kind,
                                kolonner,
                                locked: e.locked_pages.contains(&n),
                                mal,
                                photos: p.photos.iter().map(|&i| id(i)).collect(),
                                frames,
                            }
                        })
                        .collect(),
                },
            )
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
                    rotation: chosen.rotation_of(&p.hash),
                    size: chosen.size.get(&p.hash.to_hex()).copied(),
                    focus: {
                        let l = chosen.look_of(&p.hash);
                        [l.focus.0, l.focus.1]
                    },
                    whole: chosen.whole.contains(&p.hash.to_hex()),
                }
            })
            .collect(),
        learned,
        checks,
    })
}

/// «Lag utkast»: venter til bildene er lest inn, går gjennom hele året og lager et komplett
/// forslag. Fremdrift sendes som hendelsen `analyse`.
#[tauri::command]
pub async fn make_album_draft(
    app: AppHandle,
    year: i32,
    page_cap: Option<usize>,
    quiet: Option<bool>,
) -> CmdResult<DraftDto> {
    use tauri::Manager;
    // Etter en endring i utkastet lages det på nytt uten gjennomgangen på skjermen.
    let quiet = quiet.unwrap_or(false);
    let emit = {
        let app = app.clone();
        move |fase: &'static str| {
            if !quiet {
                let _ = app.emit("analyse", AnalysePhaseDto { fase });
            }
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
    let decisions = store.decisions(year)?;
    let before = std::iter::once(hash)
        .chain(other)
        .map(|h| (h, decisions.get(&h).copied()))
        .collect();
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
        (Action::Opp, _) => store.set_decision(year, &hash, Some(Decision::Opp))?,
        // En dag tas bort bilde for bilde av grensesnittet; her logges bare handlingen.
        (Action::FjernDag | Action::SlaaSammen, _) => {}
    }
    let id = store.add_feedback(year, action, &hash, other.as_ref())?;
    state.undo.push(crate::undo::Step::Photos {
        year,
        before,
        feedback: vec![id],
    });
    Ok(id)
}

/// Tar med eller tar bort mange bilder på en gang (markerte bilder). Angres samlet.
#[tauri::command]
pub fn choose_photos(
    state: State<'_, AppState>,
    year: i32,
    action: String,
    ids: Vec<String>,
) -> CmdResult<()> {
    let (action, decision) = match action.as_str() {
        "ta_med" => (Action::TaMed, Decision::Med),
        "ta_bort" => (Action::TaBort, Decision::IkkeMed),
        _ => return Err(CommandError::new("ukjent_handling", action)),
    };
    let hashes = ids
        .iter()
        .map(|id| parse_id(id))
        .collect::<CmdResult<Vec<_>>>()?;
    let guard = state.store()?;
    let store = guard.as_ref().expect("sjekket");
    let decisions = store.decisions(year)?;
    let before = hashes
        .iter()
        .map(|h| (*h, decisions.get(h).copied()))
        .collect();
    let mut feedback = Vec::with_capacity(hashes.len());
    for h in &hashes {
        store.set_decision(year, h, Some(decision))?;
        feedback.push(store.add_feedback(year, action, h, None)?);
    }
    state.undo.push(crate::undo::Step::Photos {
        year,
        before,
        feedback,
    });
    Ok(())
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

/// Valg for årets album (fornøyd med en side, rammer, turtype, slå sammen dager, sider per
/// historie, forside og bakside, rotering, størrelse, utsnitt, samle på én side, egen
/// historie). Lagres kryptert og angres samlet; grensesnittet lager utkastet på nytt etterpå.
#[tauri::command]
pub fn album_choice(
    state: State<'_, AppState>,
    year: i32,
    changes: Vec<p2a_print::choices::Change>,
) -> CmdResult<()> {
    let guard = state.store()?;
    let store = guard.as_ref().expect("sjekket");
    let before = p2a_print::choices::load(store, year)?;
    let mut c = before.clone();
    for change in changes {
        c.apply(change);
    }
    if c != before {
        p2a_print::choices::save(store, year, &c)?;
        state.undo.push(crate::undo::Step::Choices {
            year,
            before: Box::new(before),
        });
    }
    Ok(())
}
