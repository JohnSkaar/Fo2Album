//! Evaluering av utvalget mot familiens egne album (SCORING.md §10).
//!
//! 1. [`album_pdf`]: hent bildene ut av et ferdig album (PDF).
//! 2. [`matching`]: finn hvilke bilder i biblioteket albumbildene er laget av.
//! 3. [`gold`]: lagre resultatet som et gullsett (kryptert, bare lokalt).
//! 4. [`metrics`]: mål et utvalg mot gullsettet, og skriv rapport.
//!
//! Alt skjer lokalt. Gullsett og album forlater aldri maskinen; bare tallene skrives til
//! `eval/RESULTS.md`. Ingen nettverkstilgang (håndheves av `p2a-policy`).

pub mod album_pdf;
pub mod gold;
pub mod matching;
pub mod metrics;
pub mod run;

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("kunne ikke lese PDF-en: {0}")]
    Pdf(String),
    #[error(transparent)]
    Store(#[from] p2a_store::StoreError),
    #[error("{0}")]
    Other(String),
}
