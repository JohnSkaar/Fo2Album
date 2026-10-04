//! Innlesingsløpet: skann kildene → avstem med katalogen → les nye og endrede filer.
//!
//! Inkrementelt og gjenopptakbart: alt lagres per runde, og en uendret fil (samme
//! størrelse og endringstid) leses aldri på nytt. Tunge steg kjøres i parallell, mens
//! all skriving til den krypterte databasen skjer på én tråd.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use p2a_core::config::DedupConfig;
use p2a_core::{dedup, ContentHash, DateSource, PhotoMeta, TakenAt};
use p2a_store::{
    CatalogSummary, FileEntry, FileToRead, ReadOutcome, Store, StoreError, SyncReport,
};
use rayon::prelude::*;

use crate::decode::{self, DecodeError};
use crate::{dates, exif, hash, phash, scan};

/// Antall filer per runde. Styrer hvor ofte fremdrift rapporteres og data lagres.
const BATCH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Går gjennom mappene.
    Skanner,
    /// Leser filer: hash, EXIF, dato.
    Leser,
    /// Leter etter transkodede dubletter.
    Dubletter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Progress {
    pub phase: Phase,
    pub done: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct IngestReport {
    pub added: usize,
    pub changed: usize,
    pub removed: usize,
    pub read: usize,
    pub new_photos: usize,
    pub unreadable: usize,
    /// Kilder som ikke finnes (f.eks. frakoblet disk). Hoppes over, slettes ikke.
    pub missing_sources: Vec<String>,
    pub cancelled: bool,
    pub summary: CatalogSummary,
}

#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("kunne ikke lese mappen {0}: {1}")]
    Scan(PathBuf, std::io::Error),
}

/// Det som kommer ut av å analysere en ny fil.
struct Analyzed {
    meta: PhotoMeta,
    /// JPEG-miniatyr, krypteres før den lagres. `None` hvis formatet ikke kan dekodes ennå.
    thumbnail: Option<Vec<u8>>,
}

/// Leser alle kildene i katalogen.
pub fn ingest_all(
    store: &mut Store,
    dedup_cfg: &DedupConfig,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(Progress),
) -> Result<IngestReport, IngestError> {
    let mut report = IngestReport::default();
    let sources = store.sources()?;

    // 1. Skann og avstem.
    let mut to_read: Vec<(PathBuf, FileToRead)> = Vec::new();
    for (i, source) in sources.iter().enumerate() {
        progress(Progress {
            phase: Phase::Skanner,
            done: i,
            total: sources.len(),
        });
        if !source.path.is_dir() {
            report.missing_sources.push(source.label.clone());
            continue;
        }
        let scanned =
            scan::scan(&source.path).map_err(|e| IngestError::Scan(source.path.clone(), e))?;
        let entries: Vec<FileEntry> = scanned
            .into_iter()
            .map(|f| FileEntry {
                rel_path: f.rel_path,
                size: f.size,
                modified: f.modified,
                status: f.status,
            })
            .collect();
        let (sync, files): (SyncReport, _) = store.sync_files(source.id, &entries)?;
        report.added += sync.added;
        report.changed += sync.changed;
        report.removed += sync.removed;
        to_read.extend(files.into_iter().map(|f| (source.path.clone(), f)));
    }
    progress(Progress {
        phase: Phase::Skanner,
        done: sources.len(),
        total: sources.len(),
    });
    if report.removed > 0 {
        store.prune_orphans()?;
    }

    // 2. Les nye og endrede filer, runde for runde.
    let total = to_read.len();
    let mut seen: HashSet<ContentHash> = HashSet::new();
    for (n, chunk) in to_read.chunks(BATCH).enumerate() {
        if cancel.load(Ordering::Relaxed) {
            report.cancelled = true;
            break;
        }
        progress(Progress {
            phase: Phase::Leser,
            done: n * BATCH,
            total,
        });

        let hashed: Vec<(i64, PathBuf, i64, Option<ContentHash>)> = chunk
            .par_iter()
            .map(|(root, f)| {
                let path = scan::full_path(root, &f.rel_path);
                let h = hash::hash_file(&path).ok();
                (f.file_id, path, f.modified, h)
            })
            .collect();

        // Bare innhold vi ikke har sett før, analyseres.
        let mut fresh: Vec<(PathBuf, i64, ContentHash)> = Vec::new();
        for (_, path, modified, h) in &hashed {
            if let Some(h) = h {
                if !store.has_photo(h)? && seen.insert(*h) {
                    fresh.push((path.clone(), *modified, *h));
                }
            }
        }
        let analyzed: Vec<(ContentHash, Option<Analyzed>)> = fresh
            .par_iter()
            .map(|(path, modified, h)| (*h, analyze(path, *modified, *h)))
            .collect();

        let mut outcomes: Vec<(i64, ReadOutcome)> = Vec::with_capacity(hashed.len());
        let mut done_new: HashSet<ContentHash> = HashSet::new();
        for (file_id, _, _, h) in hashed {
            let outcome = match h {
                None => ReadOutcome::Unreadable,
                Some(h) => match analyzed.iter().find(|(ah, _)| *ah == h) {
                    Some((_, Some(a))) if done_new.insert(h) => {
                        ReadOutcome::New(Box::new(a.meta.clone()))
                    }
                    Some((_, None)) => ReadOutcome::Unreadable,
                    _ => ReadOutcome::Known(h),
                },
            };
            match &outcome {
                ReadOutcome::Unreadable => report.unreadable += 1,
                ReadOutcome::New(_) => report.new_photos += 1,
                ReadOutcome::Known(_) => {}
            }
            report.read += 1;
            outcomes.push((file_id, outcome));
        }
        store.record_reads(&outcomes)?;
        for (h, a) in &analyzed {
            if let Some(thumb) = a.as_ref().and_then(|a| a.thumbnail.as_deref()) {
                store.put_thumbnail(h, thumb)?;
                store.set_has_thumbnail(h)?;
            }
        }
    }
    progress(Progress {
        phase: Phase::Leser,
        done: report.read,
        total,
    });

    // 3. Transkodede dubletter, på tvers av alle kilder.
    if !report.cancelled {
        progress(Progress {
            phase: Phase::Dubletter,
            done: 0,
            total: 1,
        });
        let candidates = store.phash_candidates()?;
        let pairs = dedup::find_near_duplicates(&candidates, dedup_cfg);
        store.set_near_duplicates(&pairs)?;
        progress(Progress {
            phase: Phase::Dubletter,
            done: 1,
            total: 1,
        });
    }

    report.summary = store.summary()?;
    Ok(report)
}

/// Leser metadata, dekoder, lager miniatyr og pHash. `None` hvis filen er skadet.
fn analyze(path: &Path, modified: i64, hash: ContentHash) -> Option<Analyzed> {
    let name = path.file_name()?.to_string_lossy().to_string();
    let exif = exif::read_file(path);
    let mut meta = PhotoMeta::new(hash);
    meta.format = format_of(&name).map(str::to_string);
    let (taken_at, source) = resolve_date(exif.as_ref(), &name, modified);
    meta.taken_at = taken_at;
    meta.date_source = source;
    if let Some(e) = exif {
        meta.camera_make = e.make;
        meta.camera_model = e.model;
        meta.orientation = e.orientation;
        meta.gps = e.gps;
        meta.width = e.width;
        meta.height = e.height;
    }
    let thumbnail = match decode::decode_file(path, meta.format.as_deref(), meta.orientation) {
        Ok(decoded) => {
            // Mål etter rotering, så stående og liggende bilder får riktig form i albumet.
            meta.width = Some(decoded.width);
            meta.height = Some(decoded.height);
            meta.phash = Some(phash::phash(&decoded.image));
            Some(decode::thumbnail_jpeg(&decoded))
        }
        Err(DecodeError::Unsupported) => None,
        Err(DecodeError::Corrupt(_) | DecodeError::Io(_)) => return None,
    };
    Some(Analyzed { meta, thumbnail })
}

fn format_of(name: &str) -> Option<&'static str> {
    let ext = name.rsplit_once('.')?.1.to_ascii_lowercase();
    Some(match ext.as_str() {
        "jpg" | "jpeg" | "jpe" => "jpeg",
        "heic" | "heif" | "hif" => "heic",
        "png" => "png",
        "webp" => "webp",
        _ => return None,
    })
}

/// EXIF → filnavn → endringstid (SCORING.md §2). Endringstid markeres som usikker.
pub fn resolve_date(
    exif: Option<&exif::ExifData>,
    file_name: &str,
    modified: i64,
) -> (Option<TakenAt>, Option<DateSource>) {
    if let Some(t) = exif.and_then(|e| e.taken_at) {
        return (Some(t), Some(DateSource::Exif));
    }
    if let Some(t) = dates::from_filename(file_name) {
        return (Some(t), Some(DateSource::Filename));
    }
    match TakenAt::from_unix_utc(modified) {
        Some(t) if modified > 0 => (Some(t), Some(DateSource::FileModified)),
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_priority() {
        let e = exif::ExifData {
            taken_at: TakenAt::new(2011, 7, 14, 12, 0, 0),
            ..Default::default()
        };
        let (t, s) = resolve_date(Some(&e), "IMG_20100101_000000.jpg", 1);
        assert_eq!((t.unwrap().year, s), (2011, Some(DateSource::Exif)));
        let (t, s) = resolve_date(None, "IMG_20100101_000000.jpg", 1);
        assert_eq!((t.unwrap().year, s), (2010, Some(DateSource::Filename)));
        let (t, s) = resolve_date(None, "IMG_0001.jpg", 1_310_646_896);
        assert_eq!((t.unwrap().year, s), (2011, Some(DateSource::FileModified)));
        assert_eq!(resolve_date(None, "IMG_0001.jpg", 0), (None, None));
    }

    #[test]
    fn formats() {
        assert_eq!(format_of("a.JPG"), Some("jpeg"));
        assert_eq!(format_of("a.heif"), Some("heic"));
        assert_eq!(format_of("a.mov"), None);
    }
}
