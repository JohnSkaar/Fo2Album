//! Tauri-skallet for Fo2Album. Tynt lag: kommandoene kaller videre inn i
//! `p2a-*`-pakkene, som gjør selve arbeidet.

mod commands;
mod keys;
mod people;
mod print;
mod thumbs;

use tauri::Manager;

use commands::AppState;

/// Miljøvariabel for røyktesten i CI: appen avslutter med kode 0 så snart
/// grensesnittet har lastet og fått kontakt med Rust-kjernen.
const SMOKE_TEST_ENV: &str = "P2A_SMOKE_TEST";
/// Overstyrer datamappen (utvikling og tester).
const DATA_DIR_ENV: &str = "P2A_DATA_DIR";

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: &'static str,
    core_version: &'static str,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        core_version: p2a_core::VERSION,
    }
}

/// Kalles av grensesnittet når det har lastet ferdig.
#[tauri::command]
fn frontend_ready(app: tauri::AppHandle) {
    if std::env::var_os(SMOKE_TEST_ENV).is_some() {
        println!("røyktest: grensesnittet lastet, avslutter");
        app.exit(0);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol(thumbs::SCHEME, thumbs::handle)
        .register_asynchronous_uri_scheme_protocol(thumbs::LARGE_SCHEME, thumbs::handle_large)
        .setup(|app| {
            let data_dir = match std::env::var_os(DATA_DIR_ENV) {
                Some(dir) => dir.into(),
                None => app.path().app_data_dir()?,
            };
            app.manage(AppState::new(data_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            frontend_ready,
            commands::vault_status,
            commands::vault_create,
            commands::vault_open,
            commands::vault_recover,
            commands::delete_all_data,
            commands::list_sources,
            commands::suggested_sources,
            commands::add_source,
            commands::remove_source,
            commands::start_ingest,
            commands::cancel_ingest,
            commands::ingest_running,
            commands::catalog_summary,
            commands::list_years,
            commands::photos_in_year,
            commands::make_album_draft,
            commands::choose_photo,
            commands::answer_why,
            commands::album_choice,
            commands::page_templates,
            people::face_groups,
            people::name_face_group,
            people::ignore_face_group,
            people::move_face,
            print::album_text,
            print::export_album,
        ])
        .run(tauri::generate_context!())
        .expect("kunne ikke starte Fo2Album");
}
