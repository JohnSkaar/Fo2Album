//! Databaseskjema og migrasjoner. Versjonen ligger i `PRAGMA user_version`.
//!
//! Regel: en migrasjon som er utgitt, endres aldri. Nye endringer blir en ny migrasjon.

use rusqlite::Connection;

use crate::StoreError;

const MIGRATIONS: &[&str] = &[
    // 1: katalog (bilder og kilder) og familieprofil.
    r#"
    -- Bildemapper brukeren har gitt tilgang til.
    CREATE TABLE sources (
        id          INTEGER PRIMARY KEY,
        kind        TEXT NOT NULL,              -- SourceKind::as_str
        path        TEXT NOT NULL UNIQUE,
        label       TEXT NOT NULL,
        added_at    TEXT NOT NULL
    );

    -- Hver fil i en kilde. Samme innhold i flere kilder gir flere filer, men ett bilde.
    CREATE TABLE files (
        id            INTEGER PRIMARY KEY,
        source_id     INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
        rel_path      TEXT NOT NULL,
        size          INTEGER NOT NULL,
        modified      INTEGER NOT NULL,          -- sekunder siden 1970 (UTC)
        status        TEXT NOT NULL,             -- lokal | bare_i_skyen | uleselig
        content_hash  BLOB,                      -- NULL til filen er lest
        UNIQUE (source_id, rel_path)
    );
    CREATE INDEX files_hash ON files(content_hash);

    -- Ett bilde per unikt innhold (eksakte dubletter er dermed slått sammen).
    CREATE TABLE photos (
        content_hash    BLOB PRIMARY KEY,
        format          TEXT,
        width           INTEGER,
        height          INTEGER,
        orientation     INTEGER,
        taken_at        TEXT,                    -- TakenAt::to_iso, lokal tid
        taken_offset    INTEGER,                 -- minutter fra UTC, hvis kjent
        date_source     TEXT,                    -- DateSource::as_str
        camera_make     TEXT,
        camera_model    TEXT,
        gps_lat         REAL,
        gps_lon         REAL,
        phash           INTEGER,                 -- perseptuell hash (64 bit)
        duplicate_of    BLOB REFERENCES photos(content_hash),  -- transkodet kopi av et bedre bilde
        has_thumbnail   INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX photos_taken ON photos(taken_at);

    -- Familieprofilen: personer, kommentarer og overstyringer. Lever fra år til år.
    CREATE TABLE persons (
        id          INTEGER PRIMARY KEY,
        name        TEXT NOT NULL,
        role        TEXT NOT NULL,               -- Role::as_str
        important   INTEGER NOT NULL DEFAULT 0,  -- «spesielt viktig»
        birth_date  TEXT,                        -- ÅÅÅÅ-MM-DD, valgfritt
        created_at  TEXT NOT NULL
    );

    CREATE TABLE comments (
        id           INTEGER PRIMARY KEY,
        album_year   INTEGER NOT NULL,
        target_kind  TEXT NOT NULL,              -- bilde | side | person | hendelse | album
        target_id    TEXT,
        kind         TEXT NOT NULL,              -- CommentKind::as_str
        text         TEXT,
        created_at   TEXT NOT NULL
    );

    CREATE TABLE overrides (
        id            INTEGER PRIMARY KEY,
        album_year    INTEGER NOT NULL,
        content_hash  BLOB NOT NULL,
        action        TEXT NOT NULL,             -- lagt_til | fjernet
        created_at    TEXT NOT NULL
    );

    CREATE TABLE settings (
        key    TEXT PRIMARY KEY,
        value  TEXT NOT NULL
    );
    "#,
    // 2: enkle kvalitetsmål (M2-utgangspunktet) og krypterte dokumenter (bl.a. gullsett).
    r#"
    ALTER TABLE photos ADD COLUMN q_sharp REAL;
    ALTER TABLE photos ADD COLUMN q_exposure REAL;
    ALTER TABLE photos ADD COLUMN q_color REAL;

    -- Små dokumenter som hører til familien (gullsett, albumhistorikk). Ligger i den
    -- krypterte databasen som alt annet.
    CREATE TABLE documents (
        name        TEXT PRIMARY KEY,
        data        BLOB NOT NULL,
        updated_at  TEXT NOT NULL
    );
    "#,
    // 3: brukerens valg i utkastet og det appen lærer av dem (SCORING.md §6.3).
    r#"
    -- Ett gjeldende valg per bilde og album-år (lagt_til | fjernet).
    DELETE FROM overrides WHERE id NOT IN
        (SELECT MAX(id) FROM overrides GROUP BY album_year, content_hash);
    CREATE UNIQUE INDEX overrides_photo ON overrides(album_year, content_hash);

    -- Logg over handlinger og svarene på «Hvorfor?». Lever fra år til år.
    CREATE TABLE feedback (
        id            INTEGER PRIMARY KEY,
        album_year    INTEGER NOT NULL,
        action        TEXT NOT NULL,             -- learn::Action::as_str
        content_hash  BLOB NOT NULL,             -- bildet som ble tatt med eller tatt bort
        other_hash    BLOB,                      -- ved bytte: bildet som ble byttet ut
        reason        TEXT,                      -- learn::FeedbackReason::as_str
        created_at    TEXT NOT NULL
    );

    -- Relasjoner mellom personer i familieprofilen, f.eks. hvem som er fadder for hvem.
    -- Appen foreslår (ut fra hendelser som dåp), brukeren bekrefter.
    CREATE TABLE person_relations (
        id            INTEGER PRIMARY KEY,
        person_id     INTEGER NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
        kind          TEXT NOT NULL,             -- RelationKind::as_str
        other_id      INTEGER NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
        status        TEXT NOT NULL,             -- RelationStatus::as_str
        learned_year  INTEGER,                   -- albumåret relasjonen ble lært
        created_at    TEXT NOT NULL,
        UNIQUE (person_id, kind, other_id)
    );
    "#,
    // 4: hudtoner (stedfortreder for personer til M4) og skarphet målt på motivet. Alle
    // kvalitetsmål regnes ut på nytt ved neste innlesing.
    r#"
    ALTER TABLE photos ADD COLUMN q_skin REAL;
    UPDATE photos SET q_sharp = NULL, q_exposure = NULL, q_color = NULL;
    "#,
];

pub const CURRENT_VERSION: i64 = MIGRATIONS.len() as i64;

pub fn migrate(conn: &mut Connection) -> Result<(), StoreError> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version > CURRENT_VERSION {
        return Err(StoreError::NewerSchema(version));
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}
