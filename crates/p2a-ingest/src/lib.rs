//! Innlesing: kilder, skanning, EXIF, datoer, hashing og dubletter.
//!
//! Leser bare fra lokale mapper brukeren har gitt tilgang til. Ingen
//! nettverkstilgang (håndheves av `p2a-policy` og `deny.toml`).

pub mod dates;
pub mod decode;
pub mod exif;
pub mod hash;
pub mod phash;
pub mod pipeline;
pub mod scan;
pub mod sources;
pub mod synth;
