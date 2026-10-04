//! Utvalg: hvilke bilder som kommer med i albumet.
//!
//! - [`baseline`]: prototypens enkle algoritme, utgangspunktet M3 skal slå (ROADMAP M3).
//! - [`draft`]: hendelsesstyrt utkast med begrunnelse for hvert bilde, med og ikke med.
//!   Bruker det appen har lært av brukerens bytter (`learn`).
//! - [`cover`]: forslag til forside og bakside fra hele året.
//! - Den ekte målfunksjonen (SCORING.md §6) kommer i M3–M5.

pub mod baseline;
pub mod cover;
pub mod draft;
