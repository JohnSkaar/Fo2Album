//! Henter bildene ut av et ferdig album som PDF (f.eks. fra Blurb BookSmart).
//!
//! Bildene leses direkte ut av PDF-en (JPEG-strømmer eller rå piksler), uten å tegne sidene.
//! Pynt som går igjen på mange sider (bakgrunner, logoer) og små bilder hoppes over.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use image::DynamicImage;
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

use crate::EvalError;

/// Bilder mindre enn dette (på korteste side) regnes som pynt.
const MIN_SIDE: u32 = 120;
/// Et bilde som brukes på flere sider enn dette, er pynt (bakgrunn, logo).
const MAX_PAGES_PER_IMAGE: usize = 3;

#[derive(Debug, Clone)]
pub struct AlbumImage {
    /// Sidenummer i PDF-en, fra 1.
    pub page: u32,
    /// Rekkefølge på siden, fra 0.
    pub index: u32,
    pub image: DynamicImage,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ExtractStats {
    pub pages: u32,
    pub images: usize,
    /// Bilder i formater vi ikke leser (JPEG 2000, CCITT …).
    pub unsupported: usize,
    pub decorations: usize,
}

pub fn extract(path: &Path) -> Result<(Vec<AlbumImage>, ExtractStats), EvalError> {
    let doc = Document::load(path).map_err(|e| EvalError::Pdf(e.to_string()))?;
    extract_doc(&doc)
}

pub fn extract_doc(doc: &Document) -> Result<(Vec<AlbumImage>, ExtractStats), EvalError> {
    let pages = doc.get_pages();
    let mut stats = ExtractStats {
        pages: pages.len() as u32,
        ..Default::default()
    };

    // Først: hvilke bilder brukes på hvilke sider?
    let mut per_page: Vec<(u32, Vec<ObjectId>)> = Vec::new();
    let mut usage: HashMap<ObjectId, HashSet<u32>> = HashMap::new();
    for (&page_no, &page_id) in &pages {
        let mut found = Vec::new();
        let (direct, ids) = doc
            .get_page_resources(page_id)
            .map_err(|e| EvalError::Pdf(e.to_string()))?;
        let mut seen_forms = HashSet::new();
        if let Some(res) = direct {
            collect_images(doc, res, &mut found, &mut seen_forms);
        }
        for id in ids {
            if let Ok(res) = doc.get_dictionary(id) {
                collect_images(doc, res, &mut found, &mut seen_forms);
            }
        }
        let mut unique = Vec::new();
        for id in found {
            if !unique.contains(&id) {
                unique.push(id);
                usage.entry(id).or_default().insert(page_no);
            }
        }
        per_page.push((page_no, unique));
    }

    let mut out = Vec::new();
    let mut done: HashSet<ObjectId> = HashSet::new();
    for (page_no, ids) in per_page {
        let mut index = 0;
        for id in ids {
            if usage[&id].len() > MAX_PAGES_PER_IMAGE {
                if done.insert(id) {
                    stats.decorations += 1;
                }
                continue;
            }
            if !done.insert(id) {
                continue; // samme bilde brukt to ganger: tell det én gang
            }
            let Ok(stream) = doc.get_object(id).and_then(Object::as_stream) else {
                continue;
            };
            match decode_image(stream) {
                Some(img) if img.width().min(img.height()) >= MIN_SIDE => {
                    out.push(AlbumImage {
                        page: page_no,
                        index,
                        image: img,
                    });
                    index += 1;
                }
                Some(_) => stats.decorations += 1,
                None => stats.unsupported += 1,
            }
        }
    }
    stats.images = out.len();
    Ok((out, stats))
}

/// Bilde-XObjects i en ressursordbok, også inne i Form-XObjects.
fn collect_images(
    doc: &Document,
    resources: &Dictionary,
    out: &mut Vec<ObjectId>,
    seen_forms: &mut HashSet<ObjectId>,
) {
    let xobjects = match resources.get(b"XObject") {
        Ok(Object::Reference(id)) => doc.get_dictionary(*id).ok(),
        Ok(Object::Dictionary(d)) => Some(d),
        _ => None,
    };
    let Some(xobjects) = xobjects else { return };
    for (_, obj) in xobjects.iter() {
        let Ok(id) = obj.as_reference() else { continue };
        let Ok(stream) = doc.get_object(id).and_then(Object::as_stream) else {
            continue;
        };
        match stream.dict.get(b"Subtype").and_then(Object::as_name) {
            Ok(b"Image") => out.push(id),
            Ok(b"Form") if seen_forms.insert(id) => {
                let res = match stream.dict.get(b"Resources") {
                    Ok(Object::Reference(r)) => doc.get_dictionary(*r).ok(),
                    Ok(Object::Dictionary(d)) => Some(d),
                    _ => None,
                };
                if let Some(res) = res {
                    collect_images(doc, res, out, seen_forms);
                }
            }
            _ => {}
        }
    }
}

fn filters(stream: &Stream) -> Vec<Vec<u8>> {
    match stream.dict.get(b"Filter") {
        Ok(Object::Name(n)) => vec![n.clone()],
        Ok(Object::Array(a)) => a
            .iter()
            .filter_map(|o| o.as_name().ok().map(<[u8]>::to_vec))
            .collect(),
        _ => vec![],
    }
}

fn decode_image(stream: &Stream) -> Option<DynamicImage> {
    let f = filters(stream);
    if f.last().map(Vec::as_slice) == Some(b"DCTDecode") {
        // JPEG, ev. bak en FlateDecode.
        let bytes = if f.len() > 1 {
            stream.decompressed_content().ok()?
        } else {
            stream.content.clone()
        };
        return p2a_ingest::decode::decode_jpeg_bytes(&bytes)
            .ok()
            .or_else(|| {
                image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg).ok()
            });
    }
    if f.iter().all(|x| x == b"FlateDecode") {
        let w = stream.dict.get(b"Width").and_then(Object::as_i64).ok()? as u32;
        let h = stream.dict.get(b"Height").and_then(Object::as_i64).ok()? as u32;
        let bpc = stream
            .dict
            .get(b"BitsPerComponent")
            .and_then(Object::as_i64)
            .unwrap_or(8);
        if bpc != 8 {
            return None;
        }
        let raw = if f.is_empty() {
            stream.content.clone()
        } else {
            stream.decompressed_content().ok()?
        };
        let cs = stream
            .dict
            .get(b"ColorSpace")
            .and_then(Object::as_name)
            .unwrap_or(b"DeviceRGB");
        return match cs {
            b"DeviceRGB" => image::RgbImage::from_raw(w, h, raw).map(DynamicImage::ImageRgb8),
            b"DeviceGray" => image::GrayImage::from_raw(w, h, raw).map(DynamicImage::ImageLuma8),
            _ => None,
        };
    }
    None
}
