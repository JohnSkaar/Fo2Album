//! `miniatyr://`-protokollen: grensesnittet henter miniatyrer som `miniatyr://localhost/<hash>`
//! (Windows: `http://miniatyr.localhost/<hash>`). De dekrypteres i minnet og skrives aldri
//! ukryptert til disk.
//!
//! `stort://` gir stor visning: bildet leses fra originalfilen og skaleres i minnet. Ingenting
//! lagres, og ingenting forlater maskinen.

use p2a_core::ContentHash;
use tauri::http::{Request, Response, StatusCode};
use tauri::{AppHandle, Manager, Runtime, UriSchemeContext, UriSchemeResponder};

use crate::commands::AppState;

pub const SCHEME: &str = "miniatyr";
pub const LARGE_SCHEME: &str = "stort";
/// Lengste side i stor visning.
const LARGE_PX: u32 = 1800;

fn not_found() -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Vec::new())
        .expect("gyldig svar")
}

/// Stor visning lages i en egen tråd, så grensesnittet ikke venter.
pub fn handle_large<R: Runtime>(
    ctx: UriSchemeContext<'_, R>,
    req: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = ctx.app_handle().clone();
    let path = req.uri().path().to_string();
    std::thread::spawn(move || responder.respond(large(&app, &path).unwrap_or_else(not_found)));
}

fn large<R: Runtime>(app: &AppHandle<R>, path: &str) -> Option<Response<Vec<u8>>> {
    let hash = parse_hash(path)?;
    let file = {
        let state = app.state::<AppState>();
        let guard = state.store.lock().unwrap_or_else(|e| e.into_inner());
        guard.as_ref()?.photo_files(&[hash]).ok()?.remove(&hash)?
    };
    let decoded = p2a_ingest::decode::decode_file_at(
        &file.path,
        file.format.as_deref(),
        file.orientation,
        LARGE_PX,
    )
    .ok()?;
    let bytes = p2a_ingest::decode::preview_jpeg(&decoded, LARGE_PX, 85);
    Some(
        Response::builder()
            .header("Content-Type", "image/jpeg")
            .header("Cache-Control", "private, max-age=3600")
            .body(bytes)
            .expect("gyldig svar"),
    )
}

fn parse_hash(path: &str) -> Option<ContentHash> {
    ContentHash::from_hex(path.trim_start_matches('/'))
}

pub fn handle<R: Runtime>(
    ctx: UriSchemeContext<'_, R>,
    req: Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let Some(hash) = parse_hash(req.uri().path()) else {
        return not_found();
    };
    let state = ctx.app_handle().state::<AppState>();
    let guard = state.store.lock().unwrap_or_else(|e| e.into_inner());
    let Some(store) = guard.as_ref() else {
        return not_found();
    };
    match store.get_thumbnail(&hash) {
        Ok(Some(bytes)) => Response::builder()
            .header("Content-Type", "image/jpeg")
            // Innholdet er bestemt av hashen og endres aldri.
            .header("Cache-Control", "private, max-age=31536000, immutable")
            .body(bytes)
            .expect("gyldig svar"),
        _ => not_found(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hash_paths() {
        let hex = "ab".repeat(32);
        assert_eq!(
            parse_hash(&format!("/{hex}")),
            Some(ContentHash([0xab; 32]))
        );
        assert_eq!(parse_hash("/abc"), None);
        assert_eq!(parse_hash(&format!("/{}", "zz".repeat(32))), None);
    }
}
