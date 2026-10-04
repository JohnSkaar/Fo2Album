//! Parametre for innlesing og poengsetting (SCORING.md §11). Samlet her, ikke spredt i
//! koden, så de kan kalibreres mot evalueringssettet og logges i `eval/RESULTS.md`.

/// Parametre for dublettsøk (SCORING.md §2).
#[derive(Debug, Clone, PartialEq)]
pub struct DedupConfig {
    /// Maks Hamming-avstand mellom pHash for transkodede kopier når begge har EXIF-tid.
    pub near_dup_hamming: u32,
    /// Maks forskjell i opptakstid (sekunder) når begge har EXIF-tid.
    pub near_dup_seconds: i64,
    /// Strengere pHash-grense når minst ett av bildene mangler EXIF-tid (f.eks. WhatsApp-
    /// kopier), fordi tiden da ikke kan skille bilder i samme serie.
    pub near_dup_hamming_without_time: u32,
}

impl Default for DedupConfig {
    fn default() -> Self {
        DedupConfig {
            near_dup_hamming: 6,
            near_dup_seconds: 2,
            near_dup_hamming_without_time: 4,
        }
    }
}
