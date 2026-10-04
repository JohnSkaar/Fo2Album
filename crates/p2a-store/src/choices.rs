//! Brukerens valg i utkastet (overstyringer) og loggen appen lærer av (SCORING.md §6.3).

use std::collections::HashMap;

use p2a_core::learn::{Action, Feedback, FeedbackReason};
use p2a_core::select::draft::Decision;
use p2a_core::{BasicQuality, ContentHash};
use rusqlite::params;

use crate::catalog::hash_from;
use crate::{now_iso, Store, StoreError};

fn decision_str(d: Decision) -> &'static str {
    match d {
        Decision::Med => "lagt_til",
        Decision::IkkeMed => "fjernet",
        Decision::Fremhev => "fremhevet",
        Decision::Demp => "dempet",
        Decision::Opp => "opprioritert",
    }
}

fn quality(r: &rusqlite::Row, at: usize) -> rusqlite::Result<Option<BasicQuality>> {
    Ok(
        match (r.get(at)?, r.get(at + 1)?, r.get(at + 2)?, r.get(at + 3)?) {
            (Some(sharp), Some(exposure), Some(color), Some(skin)) => Some(BasicQuality {
                sharp,
                exposure,
                color,
                skin,
            }),
            _ => None,
        },
    )
}

impl Store {
    /// Setter brukerens valg for et bilde i årets album. `None` gir valget tilbake til appen.
    pub fn set_decision(
        &self,
        album_year: i32,
        hash: &ContentHash,
        decision: Option<Decision>,
    ) -> Result<(), StoreError> {
        match decision {
            Some(d) => self.conn.execute(
                "INSERT INTO overrides (album_year, content_hash, action, created_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(album_year, content_hash)
                 DO UPDATE SET action = excluded.action, created_at = excluded.created_at",
                params![album_year, &hash.0[..], decision_str(d), now_iso()],
            )?,
            None => self.conn.execute(
                "DELETE FROM overrides WHERE album_year = ?1 AND content_hash = ?2",
                params![album_year, &hash.0[..]],
            )?,
        };
        Ok(())
    }

    pub fn decisions(&self, album_year: i32) -> Result<HashMap<ContentHash, Decision>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT content_hash, action FROM overrides WHERE album_year = ?1")?;
        let rows = stmt.query_map([album_year], |r| {
            let d = match r.get::<_, String>(1)?.as_str() {
                "lagt_til" => Decision::Med,
                "fremhevet" => Decision::Fremhev,
                "opprioritert" => Decision::Opp,
                "dempet" => Decision::Demp,
                _ => Decision::IkkeMed,
            };
            Ok((hash_from(r.get(0)?)?, d))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Logger en handling. Svaret på «Hvorfor?» kommer eventuelt senere (`set_feedback_reason`).
    pub fn add_feedback(
        &self,
        album_year: i32,
        action: Action,
        hash: &ContentHash,
        other: Option<&ContentHash>,
    ) -> Result<i64, StoreError> {
        self.conn.execute(
            "INSERT INTO feedback (album_year, action, content_hash, other_hash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                album_year,
                action.as_str(),
                &hash.0[..],
                other.map(|h| h.0.to_vec()),
                now_iso()
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn set_feedback_reason(
        &self,
        id: i64,
        reason: Option<FeedbackReason>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE feedback SET reason = ?2 WHERE id = ?1",
            params![id, reason.map(|r| r.as_str())],
        )?;
        Ok(())
    }

    /// Hele loggen, alle år, eldste først, med kvaliteten til bildene som ble byttet.
    pub fn feedback_log(&self) -> Result<Vec<Feedback>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT f.action, f.reason,
                    a.q_sharp, a.q_exposure, a.q_color, a.q_skin,
                    b.q_sharp, b.q_exposure, b.q_color, b.q_skin
             FROM feedback f
             LEFT JOIN photos a ON a.content_hash = f.content_hash
             LEFT JOIN photos b ON b.content_hash = f.other_hash
             ORDER BY f.id",
        )?;
        let rows = stmt.query_map([], |r| {
            let action: Action = r.get::<_, String>(0)?.parse().unwrap_or(Action::TaMed);
            let reason = r
                .get::<_, Option<String>>(1)?
                .and_then(|s| s.parse::<FeedbackReason>().ok());
            let first = quality(r, 2)?;
            let second = quality(r, 6)?;
            let (added, removed) = match action {
                Action::TaMed | Action::Fremhev | Action::Demp | Action::Opp => (first, None),
                Action::FjernDag | Action::SlaaSammen => (None, None),
                Action::TaBort => (None, first),
                Action::Bytt => (first, second),
            };
            Ok(Feedback {
                action,
                reason,
                added,
                removed,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}
