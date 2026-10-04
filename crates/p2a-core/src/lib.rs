//! Kjernen i Pho2Album: domenetyper, poengsetting, utvalg og sideoppsett.
//!
//! Denne pakken er ren og deterministisk. Den gjør ingen I/O og har aldri
//! nettverkstilgang (håndheves av `p2a-policy` og `deny.toml`).

/// Versjonen av kjernen, vist i appen og logget i `eval/RESULTS.md`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_set() {
        assert!(!super::VERSION.is_empty());
    }
}
