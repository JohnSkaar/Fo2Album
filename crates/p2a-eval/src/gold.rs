//! Gullsett: hvilke bilder familien selv valgte til albumet for et år.
//!
//! Lagres som JSON i den krypterte databasen (`documents`, navn `gullsett/<navn>`), aldri
//! i repoet. Inneholder bare innholdshasher og sidenumre, ikke bilder.

use p2a_core::ContentHash;
use p2a_store::Store;
use serde::{Deserialize, Serialize};

use crate::matching::Match;
use crate::EvalError;

const PREFIX: &str = "gullsett/";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldEntry {
    /// Innholdshash (hex) for bildet i biblioteket.
    pub bilde: String,
    pub side: u32,
    pub indeks: u32,
    pub avstand: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldSet {
    pub navn: String,
    pub aar: i32,
    /// Filnavnet til album-PDF-en (ikke hele stien).
    pub kilde: String,
    pub sider: u32,
    pub bilder_i_album: usize,
    /// Sikre treff: familiens utvalg.
    pub valgt: Vec<GoldEntry>,
    /// Mulige treff som bør kontrolleres; regnes ikke med.
    pub usikre: Vec<GoldEntry>,
    /// Albumbilder uten treff i biblioteket: (side, indeks).
    pub uten_treff: Vec<(u32, u32)>,
}

impl GoldSet {
    pub fn from_matches(navn: &str, aar: i32, kilde: &str, sider: u32, matches: &[Match]) -> Self {
        let mut g = GoldSet {
            navn: navn.to_string(),
            aar,
            kilde: kilde.to_string(),
            sider,
            bilder_i_album: matches.len(),
            valgt: Vec::new(),
            usikre: Vec::new(),
            uten_treff: Vec::new(),
        };
        for m in matches {
            match m.best {
                Some((h, d)) => {
                    let e = GoldEntry {
                        bilde: h.to_hex(),
                        side: m.page,
                        indeks: m.index,
                        avstand: d,
                    };
                    if m.is_sure() {
                        // Samme bibliotekbilde brukt to ganger i albumet telles én gang.
                        if !g.valgt.iter().any(|x| x.bilde == e.bilde) {
                            g.valgt.push(e);
                        }
                    } else {
                        g.usikre.push(e);
                    }
                }
                None => g.uten_treff.push((m.page, m.index)),
            }
        }
        g
    }

    pub fn chosen(&self) -> Vec<ContentHash> {
        self.valgt
            .iter()
            .filter_map(|e| parse_hex(&e.bilde))
            .collect()
    }

    /// Andel albumbilder som fikk et sikkert treff.
    pub fn match_rate(&self) -> f64 {
        if self.bilder_i_album == 0 {
            return 0.0;
        }
        (self.bilder_i_album - self.usikre.len() - self.uten_treff.len()) as f64
            / self.bilder_i_album as f64
    }

    pub fn save(&self, store: &Store) -> Result<(), EvalError> {
        let json = serde_json::to_vec_pretty(self).map_err(|e| EvalError::Other(e.to_string()))?;
        store.put_document(&format!("{PREFIX}{}", self.navn), &json)?;
        Ok(())
    }

    pub fn load_all(store: &Store) -> Result<Vec<GoldSet>, EvalError> {
        let mut out = Vec::new();
        for name in store.document_names(PREFIX)? {
            if let Some(bytes) = store.document(&name)? {
                out.push(
                    serde_json::from_slice(&bytes)
                        .map_err(|e| EvalError::Other(format!("{name}: {e}")))?,
                );
            }
        }
        Ok(out)
    }
}

pub fn parse_hex(hex: &str) -> Option<ContentHash> {
    if hex.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(ContentHash(out))
}
