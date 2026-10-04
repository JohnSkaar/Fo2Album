//! Familieprofilen: personer, kommentarer og innstillinger som lever fra år til år.

use p2a_core::{CommentKind, Role};
use rusqlite::{params, OptionalExtension};

use crate::{now_iso, Store, StoreError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub id: i64,
    pub name: String,
    pub role: Role,
    pub important: bool,
    /// ÅÅÅÅ-MM-DD
    pub birth_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewComment {
    pub album_year: i32,
    /// bilde | side | person | hendelse | album
    pub target_kind: String,
    pub target_id: Option<String>,
    pub kind: CommentKind,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub id: i64,
    pub created_at: String,
    pub comment: NewComment,
}

fn bad_value(e: p2a_core::model::UnknownValue) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
}

impl Store {
    pub fn add_person(
        &self,
        name: &str,
        role: Role,
        important: bool,
        birth_date: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn.execute(
            "INSERT INTO persons (name, role, important, birth_date, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![name, role.as_str(), important, birth_date, now_iso()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn persons(&self) -> Result<Vec<Person>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, role, important, birth_date FROM persons ORDER BY id")?;
        let rows = stmt.query_map([], |r| {
            Ok(Person {
                id: r.get(0)?,
                name: r.get(1)?,
                role: r.get::<_, String>(2)?.parse().map_err(bad_value)?,
                important: r.get(3)?,
                birth_date: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn add_comment(&self, c: &NewComment) -> Result<i64, StoreError> {
        self.conn.execute(
            "INSERT INTO comments (album_year, target_kind, target_id, kind, text, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                c.album_year,
                c.target_kind,
                c.target_id,
                c.kind.as_str(),
                c.text,
                now_iso()
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Kommentarer for et album-år. Neste års album viser fjorårets kommentarer.
    pub fn comments(&self, album_year: i32) -> Result<Vec<Comment>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, created_at, album_year, target_kind, target_id, kind, text
             FROM comments WHERE album_year = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map([album_year], |r| {
            Ok(Comment {
                id: r.get(0)?,
                created_at: r.get(1)?,
                comment: NewComment {
                    album_year: r.get(2)?,
                    target_kind: r.get(3)?,
                    target_id: r.get(4)?,
                    kind: r.get::<_, String>(5)?.parse().map_err(bad_value)?,
                    text: r.get(6)?,
                },
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }
}
