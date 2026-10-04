//! Arbeidsflyten: lag gullsett fra et album, og evaluer utvalget mot alle gullsettene.

use std::path::Path;

use image::DynamicImage;
use std::collections::HashMap;

use p2a_core::learn::Preferences;
use p2a_core::select::baseline;
use p2a_core::select::draft::{make_draft, DraftConfig, DraftHints};
use p2a_core::ContentHash;
use p2a_store::Store;
use rayon::prelude::*;

use crate::album_pdf::{self, AlbumImage, ExtractStats};
use crate::gold::{parse_hex, GoldSet};
use crate::matching::{self, Match};
use crate::metrics::{self, Metrics};
use crate::EvalError;

/// Albumet kan inneholde bilder med feil kameraklokke; søk også i årene rundt.
const YEAR_MARGIN: i32 = 1;

pub struct GoldReport {
    pub gold: GoldSet,
    pub stats: ExtractStats,
    pub library_size: usize,
    /// Til kontrollsiden: albumbildene og treffene.
    pub album: Vec<AlbumImage>,
    pub matches: Vec<Match>,
}

/// Miniatyrene (dekryptert i minnet) for bildene fra `year` ± margin.
fn library_thumbnails(
    store: &Store,
    year: i32,
) -> Result<Vec<(ContentHash, DynamicImage)>, EvalError> {
    let mut items = Vec::new();
    for y in year - YEAR_MARGIN..=year + YEAR_MARGIN {
        for p in store.photo_metas_in_year(y)? {
            if let Some(bytes) = store.get_thumbnail(&p.hash)? {
                items.push((p.hash, bytes));
            }
        }
    }
    Ok(items
        .into_par_iter()
        .filter_map(|(h, b)| Some((h, image::load_from_memory(&b).ok()?)))
        .collect())
}

/// Lager et gullsett fra familiens album (PDF) og lagrer det kryptert.
pub fn create_gold_set(
    store: &Store,
    pdf: &Path,
    year: i32,
    name: &str,
) -> Result<GoldReport, EvalError> {
    let (album, stats) = album_pdf::extract(pdf)?;
    if album.is_empty() {
        return Err(EvalError::Other(format!(
            "fant ingen bilder i PDF-en ({} sider, {} i formater som ikke støttes)",
            stats.pages, stats.unsupported
        )));
    }
    let library = library_thumbnails(store, year)?;
    if library.is_empty() {
        return Err(EvalError::Other(format!(
            "fant ingen bilder fra {year} i katalogen; legg til bildemappene og les dem inn først"
        )));
    }
    let matches = matching::match_album(&album, &library);
    let kilde = pdf
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let gold = GoldSet::from_matches(name, year, &kilde, stats.pages, &matches);
    gold.save(store)?;
    Ok(GoldReport {
        gold,
        stats,
        library_size: library.len(),
        album,
        matches,
    })
}

pub struct EvalResult {
    pub gold: GoldSet,
    /// Utgangspunktet (prototypens utvalg) med like mange bilder som familien valgte.
    pub metrics: Metrics,
    /// Det hendelsesstyrte utkastet, med så mange bilder det selv foreslår.
    pub draft_metrics: Metrics,
}

/// Kjører utgangspunktet (prototypens utvalg, med like mange bilder som familien valgte) og
/// det hendelsesstyrte utkastet mot hvert gullsett.
pub fn evaluate_all(store: &Store) -> Result<Vec<EvalResult>, EvalError> {
    let mut out = Vec::new();
    for gold in GoldSet::load_all(store)? {
        let library = store.photo_metas_in_year(gold.aar)?;
        let chosen = gold.chosen();
        let selection = baseline::select(&library, chosen.len());
        let metrics = metrics::evaluate(&library, &chosen, &selection);
        let draft = make_draft(
            &library,
            &HashMap::new(),
            &Preferences::default(),
            &DraftConfig::default(),
            &DraftHints::default(),
            &mut |_| {},
        );
        let draft_selection: Vec<ContentHash> = draft.included().map(|p| p.hash).collect();
        let draft_metrics = metrics::evaluate(&library, &chosen, &draft_selection);
        out.push(EvalResult {
            gold,
            metrics,
            draft_metrics,
        });
    }
    Ok(out)
}

/// Kontrollside (lokal HTML-fil) med usikre treff og albumbilder uten treff, så du kan se
/// om matchingen har gått riktig. Bildene er bakt inn; filen skal ikke deles.
pub fn review_html(store: &Store, report: &GoldReport) -> Result<String, EvalError> {
    let img = |i: &DynamicImage| -> String {
        let small = i.thumbnail(320, 320).to_rgb8();
        let mut buf = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 80)
            .encode_image(&small)
            .expect("JPEG i minnet");
        format!("<img src=\"data:image/jpeg;base64,{}\">", base64(&buf))
    };
    let album_img = |page: u32, index: u32| {
        report
            .album
            .iter()
            .find(|a| a.page == page && a.index == index)
            .map(|a| img(&a.image))
            .unwrap_or_default()
    };
    let g = &report.gold;
    let mut html = format!(
        "<!doctype html><html lang=\"nb\"><meta charset=\"utf-8\"><title>Kontroll av gullsett {navn}</title>\
         <style>body{{font:15px system-ui;margin:24px;background:#FAF6F0;color:#241E18}}\
         .row{{display:flex;gap:12px;align-items:center;margin:8px 0}}img{{max-height:200px;border-radius:8px}}</style>\
         <h1>Gullsett {navn} ({aar})</h1>\
         <p>{n} bilder i albumet, {sure} sikre treff ({rate:.0} %), {usikre} usikre, {uten} uten treff. \
         Søkte i {lib} bilder.</p>",
        navn = g.navn,
        aar = g.aar,
        n = g.bilder_i_album,
        sure = g.valgt.len(),
        rate = g.match_rate() * 100.0,
        usikre = g.usikre.len(),
        uten = g.uten_treff.len(),
        lib = report.library_size,
    );
    html.push_str("<h2>Usikre treff (regnes ikke med)</h2>");
    for e in &g.usikre {
        let thumb = parse_hex(&e.bilde)
            .and_then(|h| store.get_thumbnail(&h).ok().flatten())
            .and_then(|b| image::load_from_memory(&b).ok())
            .map(|i| img(&i))
            .unwrap_or_default();
        html.push_str(&format!(
            "<div class=\"row\">{}<span>side {}, avstand {}</span>{}</div>",
            album_img(e.side, e.indeks),
            e.side,
            e.avstand,
            thumb
        ));
    }
    html.push_str("<h2>Uten treff i biblioteket</h2><div class=\"row\" style=\"flex-wrap:wrap\">");
    for (page, index) in &g.uten_treff {
        html.push_str(&album_img(*page, *index));
    }
    html.push_str("</div></html>");
    Ok(html)
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_matches_reference() {
        assert_eq!(super::base64(b"Man"), "TWFu");
        assert_eq!(super::base64(b"Ma"), "TWE=");
        assert_eq!(super::base64(b"M"), "TQ==");
    }
}
