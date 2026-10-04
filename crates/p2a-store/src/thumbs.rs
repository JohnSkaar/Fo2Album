//! Krypterte miniatyrer. Én fil per bilde, navngitt etter innholdshashen.
//! Hashen er også AAD, så en miniatyr ikke kan byttes med en annen.

use std::fs;
use std::path::PathBuf;

use p2a_core::ContentHash;

use crate::{crypto, write_atomic, Store, StoreError, THUMB_DIR};

impl Store {
    fn thumb_path(&self, hash: &ContentHash) -> PathBuf {
        let hex = hash.to_hex();
        self.dir.join(THUMB_DIR).join(&hex[..2]).join(&hex)
    }

    pub fn put_thumbnail(&self, hash: &ContentHash, image_bytes: &[u8]) -> Result<(), StoreError> {
        let path = self.thumb_path(hash);
        fs::create_dir_all(path.parent().expect("har foreldermappe"))?;
        let sealed = crypto::seal(&self.thumb_key, image_bytes, &hash.0)?;
        write_atomic(&path, &sealed)
    }

    /// `Ok(None)` hvis miniatyren ikke er laget ennå.
    pub fn get_thumbnail(&self, hash: &ContentHash) -> Result<Option<Vec<u8>>, StoreError> {
        match fs::read(self.thumb_path(hash)) {
            Ok(sealed) => crypto::open(&self.thumb_key, &sealed, &hash.0).map(Some),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
