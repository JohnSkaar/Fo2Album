//! `miniatyr://`-protokollen: grensesnittet henter miniatyrer som `miniatyr://localhost/<hash>`
//! (Windows: `http://miniatyr.localhost/<hash>`). De dekrypteres i minnet og skrives aldri
//! ukryptert til disk.

use p2a_core::ContentHash;
use tauri::http::{Request, Response, StatusCode};
use tauri::{Manager, Runtime, UriSchemeContext};

use crate::commands::AppState;

pub const SCHEME: &str = "miniatyr";

fn parse_hash(path: &str) -> Option<ContentHash> {
    ContentHash::from_hex(path.trim_start_matches('/'))
}

pub fn handle<R: Runtime>(
    ctx: UriSchemeContext<'_, R>,
    req: Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let not_found = || {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Vec::new())
            .expect("gyldig svar")
    };
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
