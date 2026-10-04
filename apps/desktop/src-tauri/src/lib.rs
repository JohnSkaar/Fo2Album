//! Tauri-skallet for Pho2Album. Tynt lag: kommandoene kaller videre inn i
//! `p2a-*`-pakkene, som gjør selve arbeidet.

/// Miljøvariabel for røyktesten i CI: appen avslutter med kode 0 så snart
/// grensesnittet har lastet og fått kontakt med Rust-kjernen.
const SMOKE_TEST_ENV: &str = "P2A_SMOKE_TEST";

#[derive(serde::Serialize)]
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
        .invoke_handler(tauri::generate_handler![app_info, frontend_ready])
        .run(tauri::generate_context!())
        .expect("kunne ikke starte Pho2Album");
}
