//! Kryptert lokal lagring for Fo2Album: katalog, familieprofil og miniatyrer.
//!
//! Alt som kan være personopplysninger ligger her, kryptert, på brukerens maskin
//! (docs/ARCHITECTURE.md, «Personvern og sikkerhet»):
//!
//! - `katalog.db`: SQLCipher-database (hele filen kryptert) med kilder, filer, bilder
//!   og familieprofilen.
//! - `miniatyrer/`: én kryptert fil per bilde (XChaCha20-Poly1305).
//! - `nokkel.json`: hovednøkkelen, pakket inn med gjenopprettingsnøkkelen. Uten
//!   gjenopprettingsnøkkelen er filen verdiløs.
//!
//! Til daglig ligger hovednøkkelen i operativsystemets nøkkelring ([`KeyStore`]).
//! Ingen nettverkstilgang (håndheves av `p2a-policy` og `deny.toml`).

mod catalog;
mod choices;
mod crypto;
mod faces;
mod keystore;
mod profile;
mod recovery;
mod schema;
mod thumbs;

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

pub use catalog::{
    CatalogSummary, FileEntry, FileToRead, PhotoSummary, ReadOutcome, Source, SyncReport,
};
pub use crypto::MasterKey;
pub use faces::{FaceGroup, FaceSample, NewFace, StoredFace, GROUP_IGNORED, GROUP_UNNAMED};
pub use keystore::{FileKeyStore, KeyStore, KeyStoreError, MemoryKeyStore};
pub use profile::{Comment, NewComment, Person, Relation};
pub use recovery::{RecoveryKey, RecoveryParseError};

use crypto::{Purpose, KEY_LEN};

const DB_FILE: &str = "katalog.db";
const VAULT_FILE: &str = "nokkel.json";
const THUMB_DIR: &str = "miniatyrer";
/// Beholder det gamle navnet med vilje (se `crypto::Purpose::info`).
const VAULT_AAD: &[u8] = b"pho2album/hovednokkel/v1";

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("databasefeil: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("filfeil: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    KeyStore(#[from] KeyStoreError),
    #[error("kunne ikke hente tilfeldige tall: {0}")]
    Random(String),
    #[error("dataene finnes allerede i {0}")]
    AlreadyExists(PathBuf),
    #[error("fant ingen data i {0}")]
    NotFound(PathBuf),
    #[error("nøkkelen mangler i nøkkelringen; gjenopprettingsnøkkelen trengs")]
    NeedsRecovery,
    #[error("feil nøkkel, eller dataene er skadet")]
    WrongKey,
    #[error("kunne ikke dekryptere")]
    Decrypt,
    #[error("nøkkelfilen er skadet")]
    CorruptKey,
    #[error("databasen er laget av en nyere versjon av appen (skjema {0})")]
    NewerSchema(i64),
}

/// Tilstanden til lagringen i en mappe, uten å åpne den.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultStatus {
    /// Første oppstart: ingen data.
    Empty,
    /// Data finnes, men nøkkelen mangler i nøkkelringen (ny maskin, eller nøkkelringen er tømt).
    NeedsRecovery,
    /// Klar til å åpnes.
    Ready,
}

#[derive(Serialize, Deserialize)]
struct VaultFile {
    versjon: u32,
    /// Hovednøkkelen kryptert med gjenopprettingsnøkkelen (hex: nonce || chiffertekst).
    innpakket_hovednokkel: String,
}

/// Åpen, dekryptert lagring. Nøklene nullstilles i minnet når den slippes.
pub struct Store {
    dir: PathBuf,
    conn: Connection,
    thumb_key: Zeroizing<[u8; KEY_LEN]>,
}

impl Store {
    pub fn status(dir: &Path, keys: &dyn KeyStore) -> Result<VaultStatus, StoreError> {
        if !dir.join(VAULT_FILE).exists() {
            return Ok(VaultStatus::Empty);
        }
        Ok(match keys.load()? {
            Some(_) => VaultStatus::Ready,
            None => VaultStatus::NeedsRecovery,
        })
    }

    /// Første oppstart: lager hovednøkkel, gjenopprettingsnøkkel og en tom database.
    /// Gjenopprettingsnøkkelen returneres slik at den kan vises for brukeren én gang.
    pub fn create(dir: &Path, keys: &dyn KeyStore) -> Result<(Store, RecoveryKey), StoreError> {
        if dir.join(VAULT_FILE).exists() || dir.join(DB_FILE).exists() {
            return Err(StoreError::AlreadyExists(dir.to_path_buf()));
        }
        fs::create_dir_all(dir)?;
        let master = MasterKey::generate()?;
        let recovery = RecoveryKey::generate()?;
        let wrapped = crypto::seal(&recovery.wrapping_key(), master.as_bytes(), VAULT_AAD)?;
        let vault = VaultFile {
            versjon: 1,
            innpakket_hovednokkel: crypto::to_hex(&wrapped),
        };
        write_atomic(
            &dir.join(VAULT_FILE),
            serde_json::to_vec_pretty(&vault)
                .expect("serialiserbar")
                .as_slice(),
        )?;
        let store = Store::open_with_key(dir, &master)?;
        keys.save(master.as_bytes())?;
        Ok((store, recovery))
    }

    /// Vanlig oppstart: henter hovednøkkelen fra nøkkelringen.
    pub fn open(dir: &Path, keys: &dyn KeyStore) -> Result<Store, StoreError> {
        if !dir.join(VAULT_FILE).exists() {
            return Err(StoreError::NotFound(dir.to_path_buf()));
        }
        let bytes = keys.load()?.ok_or(StoreError::NeedsRecovery)?;
        let master = MasterKey::from_bytes(&bytes)?;
        Store::open_with_key(dir, &master)
    }

    /// Ny maskin eller tømt nøkkelring: åpner med gjenopprettingsnøkkelen og legger
    /// hovednøkkelen tilbake i nøkkelringen.
    pub fn recover(
        dir: &Path,
        keys: &dyn KeyStore,
        recovery: &RecoveryKey,
    ) -> Result<Store, StoreError> {
        let path = dir.join(VAULT_FILE);
        if !path.exists() {
            return Err(StoreError::NotFound(dir.to_path_buf()));
        }
        let vault: VaultFile =
            serde_json::from_slice(&fs::read(&path)?).map_err(|_| StoreError::CorruptKey)?;
        let wrapped =
            crypto::from_hex(&vault.innpakket_hovednokkel).ok_or(StoreError::CorruptKey)?;
        let master_bytes = Zeroizing::new(
            crypto::open(&recovery.wrapping_key(), &wrapped, VAULT_AAD)
                .map_err(|_| StoreError::WrongKey)?,
        );
        let master = MasterKey::from_bytes(&master_bytes)?;
        let store = Store::open_with_key(dir, &master)?;
        keys.save(master.as_bytes())?;
        Ok(store)
    }

    /// «Slett alle data»: database, miniatyrer, nøkkelfil og nøkkelen i nøkkelringen.
    /// Bildene i kildemappene røres ikke.
    pub fn delete_all(dir: &Path, keys: &dyn KeyStore) -> Result<(), StoreError> {
        for name in [
            DB_FILE,
            &format!("{DB_FILE}-wal"),
            &format!("{DB_FILE}-shm"),
            VAULT_FILE,
        ] {
            match fs::remove_file(dir.join(name)) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
                _ => {}
            }
        }
        match fs::remove_dir_all(dir.join(THUMB_DIR)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
            _ => {}
        }
        keys.delete()?;
        Ok(())
    }

    fn open_with_key(dir: &Path, master: &MasterKey) -> Result<Store, StoreError> {
        let mut conn = Connection::open(dir.join(DB_FILE))?;
        // Feil nøkkel rapporteres som StoreError::WrongKey; SQLCipher skal ikke skrive til stderr.
        conn.execute_batch("PRAGMA cipher_log_level = NONE;")?;
        let db_key = crypto::secret_hex(master.derive(Purpose::Database).as_ref());
        conn.execute_batch(&format!("PRAGMA key = \"x'{}'\";", db_key.as_str()))?;
        // Første lesing avslører feil nøkkel («file is not a database»).
        conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| {
            r.get::<_, i64>(0)
        })
        .map_err(|_| StoreError::WrongKey)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
        schema::migrate(&mut conn)?;
        Ok(Store {
            dir: dir.to_path_buf(),
            conn,
            thumb_key: master.derive(Purpose::Thumbnails),
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn schema_version(&self) -> Result<i64, StoreError> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }
}

/// Skriver til en midlertidig fil og bytter navn, så en halvskrevet fil aldri blir liggende.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Nåtid som ISO 8601 i UTC.
pub(crate) fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    p2a_core::TakenAt::from_unix_utc(secs)
        .map(|t| format!("{}Z", t.to_iso()))
        .unwrap_or_default()
}
