//! Innlesingsløpet: skann kildene → avstem med katalogen → les nye og endrede filer.
//!
//! Inkrementelt og gjenopptakbart: alt lagres per runde, og en uendret fil (samme
//! størrelse og endringstid) leses aldri på nytt. Tunge steg kjøres i parallell, mens
//! all skriving til den krypterte databasen skjer på én tråd.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use p2a_core::config::DedupConfig;
use p2a_core::{dedup, ContentHash, DateSource, PhotoMeta, TakenAt};
use p2a_faces::{cluster, Embedding, FaceAnalyzer};
use p2a_store::{
    CatalogSummary, FileEntry, FileToRead, NewFace, ReadOutcome, Store, StoreError, SyncReport,
    GROUP_UNNAMED,
};
use rayon::prelude::*;

use crate::decode::{self, DecodeError};
use crate::{dates, exif, features, hash, phash, scan};

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
    /// Finner ansikter i bilder som er lest av en eldre versjon.
    Ansikter,
    /// Grupperer ansiktene i personer.
    Personer,
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
    /// Antall ansikter i katalogen etter grupperingen.
    pub faces: usize,
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
    /// Ansiktene, hvis bildet kunne dekodes og modellene er lastet.
    faces: Option<Vec<NewFace>>,
}

/// Ansiktsmodellen lastes én gang per kjøring av programmet. `None` hvis den ikke kan lastes
/// (da blir ansiktene funnet neste gang). Byttes i `p2a_faces::default_analyzer`.
fn engine() -> Option<&'static dyn FaceAnalyzer> {
    static ENGINE: OnceLock<Option<Box<dyn FaceAnalyzer>>> = OnceLock::new();
    ENGINE
        .get_or_init(|| p2a_faces::default_analyzer().ok())
        .as_deref()
}

/// Navnet på ansiktsmodellen appen bruker. Bilder analysert med en annen modell analyseres på
/// nytt, og brukerens navngiving følger med (`Store::put_faces`).
pub fn face_model() -> &'static str {
    engine().map_or(p2a_faces::DEFAULT_MODEL, |e| e.model())
}

/// Ansiktene i et dekodet bilde, med boks og landemerker som andeler av bildet. Ser det ut
/// til å ha personer langt unna (små ansikter), leses bildet på nytt i høyere oppløsning med
/// `more` (filen, når den er kjent) og analyseres igjen.
fn find_faces(
    image: &image::DynamicImage,
    more: Option<&dyn Fn() -> Option<image::DynamicImage>>,
) -> Option<Vec<NewFace>> {
    let engine = engine()?;
    let rgb = image.to_rgb8();
    let mut scan = engine.scan(&rgb).ok()?;
    let mut size = rgb.dimensions();
    if scan.more_pixels {
        if let Some(big) = more.and_then(|f| f()) {
            let big = big.to_rgb8();
            if big.width().max(big.height()) > size.0.max(size.1) {
                if let Ok(s) = engine.scan(&big) {
                    scan = s;
                    size = big.dimensions();
                }
            }
        }
    }
    let (w, h) = (size.0.max(1) as f32, size.1.max(1) as f32);
    Some(
        scan.faces
            .into_iter()
            .map(|f| {
                let d = &f.detection;
                let mut landmarks = [0.0; 10];
                for (i, [x, y]) in d.landmarks.iter().enumerate() {
                    landmarks[2 * i] = x / w;
                    landmarks[2 * i + 1] = y / h;
                }
                NewFace {
                    x: d.x / w,
                    y: d.y / h,
                    w: d.w / w,
                    h: d.h / h,
                    landmarks,
                    score: d.score,
                    sharpness: f.sharpness,
                    embedding: f.embedding.to_bytes(),
                }
            })
            .collect(),
    )
}

/// Ansikter lavere enn dette (i punkter i originalbildet) grupperes ikke: kjennetegnet blir for
/// usikkert. De telles fortsatt som personer i bildet.
const MIN_FACE_PX: f32 = 36.0;

/// Grupperer alle ansiktene i personer på nytt. Ansikter brukeren har plassert, flyttes ikke.
pub fn group_faces(store: &mut Store) -> Result<usize, StoreError> {
    let stored = store.stored_faces(face_model())?;
    let embeddings: Vec<Option<Embedding>> = stored
        .iter()
        .map(|f| Embedding::from_bytes(&f.embedding))
        .collect();
    let mut ids = Vec::new();
    let mut items = Vec::new();
    for (f, e) in stored.iter().zip(&embeddings) {
        if let Some(e) = e
            .as_ref()
            .filter(|_| f.fixed.is_some() || f.pixels >= MIN_FACE_PX)
        {
            ids.push(f.id);
            items.push(cluster::Item {
                embedding: e,
                quality: f.quality,
                fixed: f.fixed.map(|g| g as u32),
                photo: f.photo as u64,
            });
        }
    }
    let same = engine().map_or(p2a_faces::SAME_PERSON, |e| e.same_person());
    let labels = cluster::cluster(&items, same, GROUP_UNNAMED as u32);
    let groups: Vec<(i64, i64)> = ids
        .into_iter()
        .zip(labels.into_iter().map(i64::from))
        .collect();
    store.set_face_groups(&groups)?;
    Ok(groups.len())
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
        let faces: Vec<(ContentHash, Vec<NewFace>)> = analyzed
            .iter()
            .filter_map(|(h, a)| Some((*h, a.as_ref()?.faces.clone()?)))
            .collect();
        store.put_faces(&faces, face_model())?;
    }
    progress(Progress {
        phase: Phase::Leser,
        done: report.read,
        total,
    });

    // Bilder lest inn av en eldre versjon mangler kvalitetsmål; regn dem ut nå.
    if !report.cancelled {
        let missing = store.photos_missing_quality()?;
        for chunk in missing.chunks(BATCH) {
            if cancel.load(Ordering::Relaxed) {
                report.cancelled = true;
                break;
            }
            let computed: Vec<(ContentHash, p2a_core::BasicQuality)> = chunk
                .par_iter()
                .filter_map(|(h, root, rel, format, orientation)| {
                    let path = scan::full_path(root, rel);
                    let d = decode::decode_file(&path, format.as_deref(), *orientation).ok()?;
                    Some((*h, features::basic_quality(&d.image)))
                })
                .collect();
            store.set_quality(&computed)?;
        }
    }

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

    // 4. Ansikter i bilder som er lest av en eldre versjon, med en annen ansiktsmodell, eller
    // der modellene manglet.
    if !report.cancelled && engine().is_some() {
        let missing = store.photos_missing_faces(face_model())?;
        let total = missing.len();
        for (n, chunk) in missing.chunks(BATCH).enumerate() {
            if cancel.load(Ordering::Relaxed) {
                report.cancelled = true;
                break;
            }
            progress(Progress {
                phase: Phase::Ansikter,
                done: n * BATCH,
                total,
            });
            let found: Vec<(ContentHash, Vec<NewFace>)> = chunk
                .par_iter()
                .filter_map(|(h, root, rel, format, orientation)| {
                    let path = scan::full_path(root, rel);
                    let d = decode::decode_file(&path, format.as_deref(), *orientation).ok()?;
                    let more = || larger(&path, format.as_deref(), *orientation);
                    Some((*h, find_faces(&d.image, Some(&more))?))
                })
                .collect();
            store.put_faces(&found, face_model())?;
        }
    }

    // 5. Personer: grupper ansiktene på nytt når noe er nytt.
    if !report.cancelled {
        progress(Progress {
            phase: Phase::Personer,
            done: 0,
            total: 1,
        });
        report.faces = group_faces(store)?;
        progress(Progress {
            phase: Phase::Personer,
            done: 1,
            total: 1,
        });
    }

    report.summary = store.summary()?;
    Ok(report)
}

/// Bildet i høyere oppløsning, til små ansikter (personer langt unna).
fn larger(
    path: &Path,
    format: Option<&str>,
    orientation: Option<u16>,
) -> Option<image::DynamicImage> {
    decode::decode_file_at(path, format, orientation, p2a_faces::MORE_PIXELS)
        .ok()
        .map(|d| d.image)
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
    let mut faces = None;
    let thumbnail = match decode::decode_file(path, meta.format.as_deref(), meta.orientation) {
        Ok(decoded) => {
            // Mål etter rotering, så stående og liggende bilder får riktig form i albumet.
            meta.width = Some(decoded.width);
            meta.height = Some(decoded.height);
            meta.phash = Some(phash::phash(&decoded.image));
            meta.quality = Some(features::basic_quality(&decoded.image));
            let more = || larger(path, meta.format.as_deref(), meta.orientation);
            faces = find_faces(&decoded.image, Some(&more));
            Some(decode::thumbnail_jpeg(&decoded))
        }
        Err(DecodeError::Unsupported) => None,
        Err(DecodeError::Corrupt(_) | DecodeError::Io(_)) => return None,
    };
    Some(Analyzed {
        meta,
        thumbnail,
        faces,
    })
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
