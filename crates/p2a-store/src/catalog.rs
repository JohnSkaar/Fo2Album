//! Katalogen: bildemapper, filene i dem og de unike bildene.
//!
//! Én rad i `photos` per unikt innhold. Samme fil i Dropbox og iCloud gir to rader i
//! `files`, men ett bilde (eksakt dublett). Transkodede kopier (samme bilde i HEIC og
//! nedskalert JPEG) er egne bilder med `duplicate_of` satt til den beste kopien.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use p2a_core::{BasicQuality, ContentHash, DateSource, FileStatus, PhotoMeta, SourceKind, TakenAt};
use rusqlite::{params, OptionalExtension};

use crate::{now_iso, Store, StoreError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub id: i64,
    pub kind: SourceKind,
    pub path: PathBuf,
    pub label: String,
}

/// En fil slik skanningen fant den.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub rel_path: String,
    pub size: u64,
    pub modified: i64,
    pub status: FileStatus,
}

/// En lokal fil som må leses (ny, eller endret siden sist).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileToRead {
    pub file_id: i64,
    pub source_id: i64,
    pub rel_path: String,
    pub modified: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncReport {
    pub added: usize,
    pub changed: usize,
    pub removed: usize,
    pub unchanged: usize,
    pub cloud_only: usize,
}

/// Resultatet av å lese én fil.
#[derive(Debug, Clone, PartialEq)]
pub enum ReadOutcome {
    /// Hashen er kjent fra før (eksakt dublett eller uendret innhold).
    Known(ContentHash),
    /// Nytt bilde.
    New(Box<PhotoMeta>),
    Unreadable,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct CatalogSummary {
    pub sources: usize,
    pub files: usize,
    pub cloud_only: usize,
    pub unreadable: usize,
    /// Unike bilder, uten transkodede kopier.
    pub photos: usize,
    /// Filer som er eksakte kopier av en annen fil.
    pub exact_duplicates: usize,
    /// Bilder som er transkodede kopier av et bedre bilde.
    pub near_duplicates: usize,
    /// Bilder der datoen bare kommer fra filens endringstid.
    pub uncertain_dates: usize,
    /// Bilder uten miniatyr (f.eks. HEIC på en maskin uten HEIC-dekoder).
    pub without_preview: usize,
}

/// Et bilde slik grensesnittet trenger det.
#[derive(Debug, Clone, PartialEq)]
pub struct PhotoSummary {
    pub hash: ContentHash,
    pub taken_at: Option<TakenAt>,
    pub date_source: Option<DateSource>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub orientation: Option<u16>,
    pub has_thumbnail: bool,
    pub sources: Vec<SourceKind>,
}

fn conv<E: std::error::Error + Send + Sync + 'static>(e: E) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
}

pub(crate) fn hash_from(blob: Vec<u8>) -> rusqlite::Result<ContentHash> {
    let arr: [u8; 32] = blob
        .try_into()
        .map_err(|_| conv(std::io::Error::other("hash har feil lengde")))?;
    Ok(ContentHash(arr))
}

impl Store {
    /// Legger til en bildemappe. Finnes stien fra før, returneres den eksisterende id-en.
    pub fn add_source(
        &self,
        kind: SourceKind,
        path: &Path,
        label: &str,
    ) -> Result<i64, StoreError> {
        let path_s = path.to_string_lossy();
        if let Some(id) = self
            .conn
            .query_row("SELECT id FROM sources WHERE path = ?1", [&*path_s], |r| {
                r.get(0)
            })
            .optional()?
        {
            return Ok(id);
        }
        self.conn.execute(
            "INSERT INTO sources (kind, path, label, added_at) VALUES (?1, ?2, ?3, ?4)",
            params![kind.as_str(), &*path_s, label, now_iso()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn sources(&self) -> Result<Vec<Source>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, kind, path, label FROM sources ORDER BY id")?;
        let rows = stmt.query_map([], |r| {
            Ok(Source {
                id: r.get(0)?,
                kind: r.get::<_, String>(1)?.parse().map_err(conv)?,
                path: PathBuf::from(r.get::<_, String>(2)?),
                label: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Fjerner en bildemappe fra appen (ikke fra disken) og rydder bilder som ikke
    /// lenger finnes i noen kilde. Returnerer hashene til bildene som ble fjernet.
    pub fn remove_source(&mut self, id: i64) -> Result<Vec<ContentHash>, StoreError> {
        self.conn
            .execute("DELETE FROM sources WHERE id = ?1", [id])?;
        self.prune_orphans()
    }

    /// Avstemmer filene i en kilde mot en ny skanning. Returnerer filene som må leses.
    pub fn sync_files(
        &mut self,
        source_id: i64,
        scanned: &[FileEntry],
    ) -> Result<(SyncReport, Vec<FileToRead>), StoreError> {
        let tx = self.conn.transaction()?;
        let mut known: HashMap<String, (i64, i64, i64, String, bool)> = HashMap::new();
        {
            let mut stmt = tx.prepare(
                "SELECT id, rel_path, size, modified, status, content_hash IS NOT NULL
                 FROM files WHERE source_id = ?1",
            )?;
            let rows = stmt.query_map([source_id], |r| {
                Ok((
                    r.get::<_, String>(1)?,
                    (r.get(0)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?),
                ))
            })?;
            for row in rows {
                let (k, v) = row?;
                known.insert(k, v);
            }
        }

        let mut report = SyncReport::default();
        let mut to_read = Vec::new();
        for f in scanned {
            let status = f.status.as_str();
            if f.status == FileStatus::CloudOnly {
                report.cloud_only += 1;
            }
            match known.remove(&f.rel_path) {
                Some((id, size, modified, old_status, hashed))
                    if size == f.size as i64 && modified == f.modified && old_status == status =>
                {
                    report.unchanged += 1;
                    if f.status == FileStatus::Local && !hashed {
                        to_read.push(FileToRead {
                            file_id: id,
                            source_id,
                            rel_path: f.rel_path.clone(),
                            modified: f.modified,
                        });
                    }
                }
                Some((id, ..)) => {
                    report.changed += 1;
                    tx.execute(
                        "UPDATE files SET size = ?2, modified = ?3, status = ?4, content_hash = NULL
                         WHERE id = ?1",
                        params![id, f.size as i64, f.modified, status],
                    )?;
                    if f.status == FileStatus::Local {
                        to_read.push(FileToRead {
                            file_id: id,
                            source_id,
                            rel_path: f.rel_path.clone(),
                            modified: f.modified,
                        });
                    }
                }
                None => {
                    report.added += 1;
                    tx.execute(
                        "INSERT INTO files (source_id, rel_path, size, modified, status)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![source_id, f.rel_path, f.size as i64, f.modified, status],
                    )?;
                    if f.status == FileStatus::Local {
                        to_read.push(FileToRead {
                            file_id: tx.last_insert_rowid(),
                            source_id,
                            rel_path: f.rel_path.clone(),
                            modified: f.modified,
                        });
                    }
                }
            }
        }
        for (id, ..) in known.values() {
            tx.execute("DELETE FROM files WHERE id = ?1", [id])?;
            report.removed += 1;
        }
        tx.commit()?;
        Ok((report, to_read))
    }

    pub fn has_photo(&self, hash: &ContentHash) -> Result<bool, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT 1 FROM photos WHERE content_hash = ?1",
                [&hash.0[..]],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    /// Lagrer resultatene fra en runde med lesing, i én transaksjon.
    pub fn record_reads(&mut self, reads: &[(i64, ReadOutcome)]) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        for (file_id, outcome) in reads {
            match outcome {
                ReadOutcome::Unreadable => {
                    tx.execute(
                        "UPDATE files SET status = ?2, content_hash = NULL WHERE id = ?1",
                        params![file_id, FileStatus::Unreadable.as_str()],
                    )?;
                }
                ReadOutcome::Known(hash) => {
                    tx.execute(
                        "UPDATE files SET content_hash = ?2 WHERE id = ?1",
                        params![file_id, &hash.0[..]],
                    )?;
                }
                ReadOutcome::New(p) => {
                    tx.execute(
                        "INSERT OR IGNORE INTO photos (content_hash, format, width, height,
                            orientation, taken_at, taken_offset, date_source, camera_make,
                            camera_model, gps_lat, gps_lon, phash, q_sharp, q_exposure, q_color)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                        params![
                            &p.hash.0[..],
                            p.format,
                            p.width,
                            p.height,
                            p.orientation,
                            p.taken_at.map(|t| t.to_iso()),
                            p.taken_at.and_then(|t| t.offset_minutes),
                            p.date_source.map(|d| d.as_str()),
                            p.camera_make,
                            p.camera_model,
                            p.gps.map(|g| g.0),
                            p.gps.map(|g| g.1),
                            p.phash.map(|h| h as i64),
                            p.quality.map(|q| q.sharp),
                            p.quality.map(|q| q.exposure),
                            p.quality.map(|q| q.color),
                        ],
                    )?;
                    tx.execute(
                        "UPDATE files SET content_hash = ?2 WHERE id = ?1",
                        params![file_id, &p.hash.0[..]],
                    )?;
                }
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn set_has_thumbnail(&self, hash: &ContentHash) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE photos SET has_thumbnail = 1 WHERE content_hash = ?1",
            [&hash.0[..]],
        )?;
        Ok(())
    }

    /// Bilder med perseptuell hash, til dublettsøk.
    pub fn phash_candidates(&self) -> Result<Vec<PhotoMeta>, StoreError> {
        self.load_photos("WHERE phash IS NOT NULL", &[])
    }

    /// Unike bilder (uten transkodede kopier) fra et år, med alle metadata. Brukes av
    /// utvalget og evalueringen.
    pub fn photo_metas_in_year(&self, year: i32) -> Result<Vec<PhotoMeta>, StoreError> {
        let (from, to) = (format!("{year:04}"), format!("{:04}", year + 1));
        self.load_photos(
            "WHERE duplicate_of IS NULL AND taken_at >= ?1 AND taken_at < ?2 ORDER BY taken_at, content_hash",
            &[&from, &to],
        )
    }

    fn load_photos(
        &self,
        filter: &str,
        args: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<PhotoMeta>, StoreError> {
        let sql = format!(
            "SELECT content_hash, phash, taken_at, taken_offset, date_source, width, height,
                    camera_make, camera_model, format, orientation, gps_lat, gps_lon,
                    q_sharp, q_exposure, q_color
             FROM photos {filter}"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(args, |r| {
            let mut p = PhotoMeta::new(hash_from(r.get(0)?)?);
            p.phash = r.get::<_, Option<i64>>(1)?.map(|h| h as u64);
            p.taken_at = r
                .get::<_, Option<String>>(2)?
                .and_then(|s| TakenAt::from_iso(&s));
            if let Some(t) = p.taken_at.as_mut() {
                t.offset_minutes = r.get(3)?;
            }
            p.date_source = r.get::<_, Option<String>>(4)?.and_then(|s| s.parse().ok());
            p.width = r.get(5)?;
            p.height = r.get(6)?;
            p.camera_make = r.get(7)?;
            p.camera_model = r.get(8)?;
            p.format = r.get(9)?;
            p.orientation = r.get(10)?;
            p.gps = match (r.get::<_, Option<f64>>(11)?, r.get::<_, Option<f64>>(12)?) {
                (Some(a), Some(b)) => Some((a, b)),
                _ => None,
            };
            p.quality = match (r.get(13)?, r.get(14)?, r.get(15)?) {
                (Some(sharp), Some(exposure), Some(color)) => Some(BasicQuality {
                    sharp,
                    exposure,
                    color,
                }),
                _ => None,
            };
            Ok(p)
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Bilder som mangler kvalitetsmål, med en lokal fil de kan leses fra:
    /// (hash, kildemappe, relativ sti, format, orientering).
    #[allow(clippy::type_complexity)]
    pub fn photos_missing_quality(
        &self,
    ) -> Result<Vec<(ContentHash, PathBuf, String, Option<String>, Option<u16>)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.content_hash, s.path, f.rel_path, p.format, p.orientation
             FROM photos p
             JOIN files f ON f.id = (SELECT id FROM files WHERE content_hash = p.content_hash
                                     AND status = 'lokal' LIMIT 1)
             JOIN sources s ON s.id = f.source_id
             WHERE p.q_sharp IS NULL AND p.has_thumbnail = 1",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                hash_from(r.get(0)?)?,
                PathBuf::from(r.get::<_, String>(1)?),
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
            ))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn set_quality(&mut self, items: &[(ContentHash, BasicQuality)]) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        for (h, q) in items {
            tx.execute(
                "UPDATE photos SET q_sharp = ?2, q_exposure = ?3, q_color = ?4 WHERE content_hash = ?1",
                params![&h.0[..], q.sharp, q.exposure, q.color],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Lagrer et dokument (f.eks. et gullsett) i den krypterte databasen.
    pub fn put_document(&self, name: &str, data: &[u8]) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO documents (name, data, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(name) DO UPDATE SET data = excluded.data, updated_at = excluded.updated_at",
            params![name, data, now_iso()],
        )?;
        Ok(())
    }

    pub fn document(&self, name: &str) -> Result<Option<Vec<u8>>, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT data FROM documents WHERE name = ?1", [name], |r| {
                r.get(0)
            })
            .optional()?)
    }

    /// Navn på dokumenter som starter med `prefix`.
    pub fn document_names(&self, prefix: &str) -> Result<Vec<String>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM documents WHERE name LIKE ?1 || '%' ORDER BY name")?;
        let rows = stmt.query_map([prefix], |r| r.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Setter `duplicate_of` for transkodede kopier. Alle tidligere markeringer erstattes.
    pub fn set_near_duplicates(
        &mut self,
        pairs: &[(ContentHash, ContentHash)],
    ) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        tx.execute("UPDATE photos SET duplicate_of = NULL", [])?;
        for (copy, best) in pairs {
            tx.execute(
                "UPDATE photos SET duplicate_of = ?2 WHERE content_hash = ?1",
                params![&copy.0[..], &best.0[..]],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Sletter bilder som ikke lenger har noen fil, og miniatyrene deres.
    pub fn prune_orphans(&mut self) -> Result<Vec<ContentHash>, StoreError> {
        let orphans: Vec<ContentHash> = {
            let mut stmt = self.conn.prepare(
                "SELECT content_hash FROM photos p
                 WHERE NOT EXISTS (SELECT 1 FROM files f WHERE f.content_hash = p.content_hash)",
            )?;
            let rows = stmt.query_map([], |r| hash_from(r.get(0)?))?;
            rows.collect::<Result<_, _>>()?
        };
        let tx = self.conn.transaction()?;
        for h in &orphans {
            tx.execute(
                "UPDATE photos SET duplicate_of = NULL WHERE duplicate_of = ?1",
                [&h.0[..]],
            )?;
            tx.execute("DELETE FROM photos WHERE content_hash = ?1", [&h.0[..]])?;
        }
        tx.commit()?;
        for h in &orphans {
            self.delete_thumbnail(h)?;
        }
        Ok(orphans)
    }

    pub fn summary(&self) -> Result<CatalogSummary, StoreError> {
        let one = |sql: &str| -> Result<usize, StoreError> {
            Ok(self.conn.query_row(sql, [], |r| r.get::<_, i64>(0))? as usize)
        };
        Ok(CatalogSummary {
            sources: one("SELECT count(*) FROM sources")?,
            files: one("SELECT count(*) FROM files")?,
            cloud_only: one("SELECT count(*) FROM files WHERE status = 'bare_i_skyen'")?,
            unreadable: one("SELECT count(*) FROM files WHERE status = 'uleselig'")?,
            photos: one("SELECT count(*) FROM photos WHERE duplicate_of IS NULL")?,
            exact_duplicates: one(
                "SELECT count(*) - count(DISTINCT content_hash) FROM files
                 WHERE content_hash IS NOT NULL",
            )?,
            near_duplicates: one("SELECT count(*) FROM photos WHERE duplicate_of IS NOT NULL")?,
            uncertain_dates: one(
                "SELECT count(*) FROM photos WHERE duplicate_of IS NULL AND date_source = 'endringstid'",
            )?,
            without_preview: one(
                "SELECT count(*) FROM photos WHERE duplicate_of IS NULL AND has_thumbnail = 0",
            )?,
        })
    }

    /// År med bilder og antall unike bilder per år (uten transkodede kopier).
    pub fn years(&self) -> Result<Vec<(i32, usize)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT CAST(substr(taken_at, 1, 4) AS INTEGER) AS y, count(*)
             FROM photos WHERE duplicate_of IS NULL AND taken_at IS NOT NULL
             GROUP BY y ORDER BY y",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as usize)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Unike bilder fra et år, sortert på opptakstid.
    pub fn photos_in_year(&self, year: i32) -> Result<Vec<PhotoSummary>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.content_hash, p.taken_at, p.taken_offset, p.date_source, p.width, p.height,
                    p.orientation, p.has_thumbnail,
                    (SELECT group_concat(DISTINCT s.kind) FROM files f JOIN sources s ON s.id = f.source_id
                     WHERE f.content_hash = p.content_hash
                        OR f.content_hash IN (SELECT content_hash FROM photos d WHERE d.duplicate_of = p.content_hash))
             FROM photos p
             WHERE p.duplicate_of IS NULL AND p.taken_at >= ?1 AND p.taken_at < ?2
             ORDER BY p.taken_at, p.content_hash",
        )?;
        let rows = stmt.query_map(
            params![format!("{year:04}"), format!("{:04}", year + 1)],
            |r| {
                let mut taken_at = r
                    .get::<_, Option<String>>(1)?
                    .and_then(|s| TakenAt::from_iso(&s));
                if let Some(t) = taken_at.as_mut() {
                    t.offset_minutes = r.get(2)?;
                }
                let mut sources: Vec<SourceKind> = r
                    .get::<_, Option<String>>(8)?
                    .unwrap_or_default()
                    .split(',')
                    .filter_map(|s| s.parse().ok())
                    .collect();
                sources.sort_by_key(|k| k.as_str());
                Ok(PhotoSummary {
                    hash: hash_from(r.get(0)?)?,
                    taken_at,
                    date_source: r.get::<_, Option<String>>(3)?.and_then(|s| s.parse().ok()),
                    width: r.get(4)?,
                    height: r.get(5)?,
                    orientation: r.get(6)?,
                    has_thumbnail: r.get(7)?,
                    sources,
                })
            },
        )?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}
