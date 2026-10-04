//! Ansikter (M4): hvor de er i bildene, kjennetegnene, og hvilken person de hører til.
//!
//! Grupperingen (samme person) lagres i `grp`. Gruppenummer under [`GROUP_UNNAMED`] er
//! personer i familieprofilen (`persons.id`); [`GROUP_IGNORED`] er ansikter brukeren har sagt
//! ikke skal telle; nummer fra [`GROUP_UNNAMED`] og oppover er personer uten navn ennå.

use std::collections::HashMap;
use std::path::PathBuf;

use p2a_core::{ContentHash, FaceInfo};
use rusqlite::params;

use crate::catalog::hash_from;
use crate::{Store, StoreError};

/// Første gruppenummer for personer uten navn.
pub const GROUP_UNNAMED: i64 = 1_000_000;
/// Gruppen for ansikter som ikke skal telle.
pub const GROUP_IGNORED: i64 = 999_999;

/// Et ansikt som skal lagres. Boks og landemerker er andeler (0–1) av bildet.
#[derive(Debug, Clone, PartialEq)]
pub struct NewFace {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub landmarks: [f32; 10],
    pub score: f32,
    pub sharpness: f32,
    /// Kjennetegnet, som bytes (se `p2a_faces::Embedding::to_bytes`).
    pub embedding: Vec<u8>,
}

/// Et lagret ansikt, slik grupperingen trenger det.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredFace {
    pub id: i64,
    pub embedding: Vec<u8>,
    /// Ansiktets størrelse (andel av bildehøyden) ganger skarphet: de beste grupperes først.
    pub quality: f32,
    /// Gruppen brukeren har plassert ansiktet i (person eller ignorert), hvis noen.
    pub fixed: Option<i64>,
    /// Bildet ansiktet er i (rad-id), så to ansikter i samme bilde aldri blir samme person.
    pub photo: i64,
    /// Ansiktets høyde i punkter i originalbildet (0 hvis bildets høyde ikke er kjent).
    pub pixels: f32,
}

/// Et utsnitt å vise for en gruppe: bildet og boksen.
#[derive(Debug, Clone, PartialEq)]
pub struct FaceSample {
    pub face_id: i64,
    pub hash: ContentHash,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// En gruppe ansikter (én person), til «Hvem er med?».
#[derive(Debug, Clone, PartialEq)]
pub struct FaceGroup {
    pub group: i64,
    pub person_id: Option<i64>,
    /// Antall ansikter, og antall bilder de er i.
    pub faces: usize,
    pub photos: usize,
    /// De beste ansiktene, høyst `samples`.
    pub samples: Vec<FaceSample>,
}

impl Store {
    /// Bilder der ansiktene ikke er funnet ennå: (hash, kilde, sti, format, orientering).
    #[allow(clippy::type_complexity)]
    pub fn photos_missing_faces(
        &self,
    ) -> Result<Vec<(ContentHash, PathBuf, String, Option<String>, Option<u16>)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.content_hash, s.path, f.rel_path, p.format, p.orientation
             FROM photos p
             JOIN files f ON f.id = (SELECT id FROM files WHERE content_hash = p.content_hash
                                     AND status = 'lokal' LIMIT 1)
             JOIN sources s ON s.id = f.source_id
             WHERE p.faces_done = 0 AND p.has_thumbnail = 1 AND p.duplicate_of IS NULL
             ORDER BY p.taken_at",
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

    /// Lagrer ansiktene i et bilde (erstatter de gamle) og merker bildet som ferdig.
    pub fn put_faces(&mut self, items: &[(ContentHash, Vec<NewFace>)]) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        for (h, faces) in items {
            tx.execute("DELETE FROM faces WHERE content_hash = ?1", [&h.0[..]])?;
            for f in faces {
                let lm: Vec<u8> = f.landmarks.iter().flat_map(|v| v.to_le_bytes()).collect();
                tx.execute(
                    "INSERT INTO faces (content_hash, x, y, w, h, landmarks, score, sharpness, embedding)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![&h.0[..], f.x, f.y, f.w, f.h, lm, f.score, f.sharpness, f.embedding],
                )?;
            }
            tx.execute(
                "UPDATE photos SET faces_done = 1 WHERE content_hash = ?1",
                [&h.0[..]],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Alle ansiktene, til grupperingen. Sortert på id, så grupperingen blir lik hver gang.
    pub fn stored_faces(&self) -> Result<Vec<StoredFace>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT f.id, f.embedding, f.h * f.sharpness, f.confirmed, f.ignored, f.person_id,
                    p.rowid, f.h * COALESCE(p.height, 0)
             FROM faces f JOIN photos p ON p.content_hash = f.content_hash ORDER BY f.id",
        )?;
        let rows = stmt.query_map([], |r| {
            let confirmed: bool = r.get(3)?;
            let ignored: bool = r.get(4)?;
            let person: Option<i64> = r.get(5)?;
            Ok(StoredFace {
                id: r.get(0)?,
                embedding: r.get(1)?,
                quality: r.get::<_, f64>(2)? as f32,
                fixed: if ignored {
                    Some(GROUP_IGNORED)
                } else if confirmed {
                    person
                } else {
                    None
                },
                photo: r.get(6)?,
                pixels: r.get::<_, f64>(7)? as f32,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Lagrer grupperingen. Ansikter brukeren ikke har plassert, og som havner i en navngitt
    /// persons gruppe, får personen som forslag (ikke bekreftet).
    pub fn set_face_groups(&mut self, groups: &[(i64, i64)]) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        for (id, g) in groups {
            tx.execute(
                "UPDATE faces SET grp = ?2,
                    person_id = CASE WHEN confirmed = 1 THEN person_id
                                     WHEN ?2 < ?3 AND ?2 <> ?4 THEN ?2 ELSE NULL END
                 WHERE id = ?1",
                params![id, g, GROUP_UNNAMED, GROUP_IGNORED],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Gruppene, de største først, med de beste ansiktene som utsnitt. Ignorerte gruppen tas
    /// ikke med. `min_faces` skjuler grupper med få ansikter (tilfeldige folk i bakgrunnen).
    pub fn face_groups(
        &self,
        min_faces: usize,
        samples: usize,
    ) -> Result<Vec<FaceGroup>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT grp, MIN(CASE WHEN grp < ?1 THEN grp END), COUNT(*), COUNT(DISTINCT content_hash)
             FROM faces WHERE grp IS NOT NULL AND grp <> ?2 AND ignored = 0
             GROUP BY grp HAVING COUNT(*) >= ?3
             ORDER BY COUNT(*) DESC, grp",
        )?;
        let mut groups: Vec<FaceGroup> = stmt
            .query_map(
                params![GROUP_UNNAMED, GROUP_IGNORED, min_faces as i64],
                |r| {
                    Ok(FaceGroup {
                        group: r.get(0)?,
                        person_id: r.get(1)?,
                        faces: r.get::<_, i64>(2)? as usize,
                        photos: r.get::<_, i64>(3)? as usize,
                        samples: Vec::new(),
                    })
                },
            )?
            .collect::<Result<_, _>>()?;
        let mut pick = self.conn.prepare(
            "SELECT id, content_hash, x, y, w, h FROM faces WHERE grp = ?1
             ORDER BY h * sharpness * score DESC, id LIMIT ?2",
        )?;
        for g in &mut groups {
            g.samples = pick
                .query_map(params![g.group, samples as i64], |r| {
                    Ok(FaceSample {
                        face_id: r.get(0)?,
                        hash: hash_from(r.get(1)?)?,
                        x: r.get(2)?,
                        y: r.get(3)?,
                        w: r.get(4)?,
                        h: r.get(5)?,
                    })
                })?
                .collect::<Result<_, _>>()?;
        }
        Ok(groups)
    }

    /// Brukeren gir en gruppe navn: alle ansiktene i gruppen hører til personen.
    pub fn name_face_group(&mut self, group: i64, person_id: i64) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE faces SET person_id = ?2, confirmed = 1, ignored = 0, grp = ?2 WHERE grp = ?1",
            params![group, person_id],
        )?;
        Ok(())
    }

    /// Brukeren sier at gruppen ikke skal telle (fremmede, folk i bakgrunnen).
    pub fn ignore_face_group(&mut self, group: i64) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE faces SET person_id = NULL, confirmed = 1, ignored = 1, grp = ?2 WHERE grp = ?1",
            params![group, GROUP_IGNORED],
        )?;
        Ok(())
    }

    /// Brukeren flytter ett ansikt til en person (eller ut av alle med `None`).
    pub fn move_face(&mut self, face_id: i64, person_id: Option<i64>) -> Result<(), StoreError> {
        match person_id {
            Some(p) => self.conn.execute(
                "UPDATE faces SET person_id = ?2, confirmed = 1, ignored = 0, grp = ?2 WHERE id = ?1",
                params![face_id, p],
            )?,
            None => self.conn.execute(
                "UPDATE faces SET person_id = NULL, confirmed = 1, ignored = 1, grp = ?2 WHERE id = ?1",
                params![face_id, GROUP_IGNORED],
            )?,
        };
        Ok(())
    }

    /// Ansiktene i årets bilder, per bilde. Bilder uten ansikter, men der ansiktene er
    /// funnet, får en tom liste; bilder som ikke er gått gjennom ennå, er ikke med.
    pub fn faces_in_year(
        &self,
        year: i32,
    ) -> Result<HashMap<ContentHash, Vec<FaceInfo>>, StoreError> {
        let (from, to) = (format!("{year:04}-01-01"), format!("{:04}-01-01", year + 1));
        let mut out: HashMap<ContentHash, Vec<FaceInfo>> = HashMap::new();
        let mut done = self.conn.prepare(
            "SELECT content_hash FROM photos WHERE faces_done = 1 AND taken_at >= ?1 AND taken_at < ?2",
        )?;
        for h in done.query_map([&from, &to], |r| hash_from(r.get(0)?))? {
            out.insert(h?, Vec::new());
        }
        let mut stmt = self.conn.prepare(
            "SELECT f.content_hash, f.x, f.y, f.w, f.h, f.score, f.sharpness, f.person_id, f.grp, f.ignored
             FROM faces f JOIN photos p ON p.content_hash = f.content_hash
             WHERE p.taken_at >= ?1 AND p.taken_at < ?2 ORDER BY f.id",
        )?;
        let rows = stmt.query_map([&from, &to], |r| {
            Ok((
                hash_from(r.get(0)?)?,
                FaceInfo {
                    x: r.get(1)?,
                    y: r.get(2)?,
                    w: r.get(3)?,
                    h: r.get(4)?,
                    score: r.get(5)?,
                    sharpness: r.get(6)?,
                    person: r.get(7)?,
                    group: r.get(8)?,
                    ignored: r.get(9)?,
                },
            ))
        })?;
        for row in rows {
            let (h, f) = row?;
            out.entry(h).or_default().push(f);
        }
        Ok(out)
    }
}
