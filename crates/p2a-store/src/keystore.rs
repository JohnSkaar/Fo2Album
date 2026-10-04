//! Hvor hovednøkkelen oppbevares til daglig: operativsystemets nøkkelring.
//!
//! Selve koblingen til Nøkkelring (Mac) og Credential Manager (Windows) ligger i
//! app-skallet. Denne pakken kjenner bare trait-et, så den kan testes uten OS-tjenester.

use std::sync::Mutex;

use zeroize::Zeroizing;

/// Feil fra nøkkelringen, som tekst fra OS-et.
#[derive(Debug, thiserror::Error)]
#[error("nøkkelringen: {0}")]
pub struct KeyStoreError(pub String);

pub trait KeyStore: Send + Sync {
    /// `Ok(None)` hvis det ikke finnes noen nøkkel (ny maskin eller slettet).
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, KeyStoreError>;
    fn save(&self, key: &[u8]) -> Result<(), KeyStoreError>;
    /// Skal lykkes også når det ikke finnes noen nøkkel.
    fn delete(&self) -> Result<(), KeyStoreError>;
}

/// Nøkkelring i minnet, for tester og utvikling.
#[derive(Default)]
pub struct MemoryKeyStore(Mutex<Option<Zeroizing<Vec<u8>>>>);

impl KeyStore for MemoryKeyStore {
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, KeyStoreError> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn save(&self, key: &[u8]) -> Result<(), KeyStoreError> {
        *self.0.lock().unwrap() = Some(Zeroizing::new(key.to_vec()));
        Ok(())
    }
    fn delete(&self) -> Result<(), KeyStoreError> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}

/// Nøkkel i en fil. **Bare for utvikling og testverktøy** (Linux, `p2a demo`): appen for
/// Mac og Windows bruker alltid operativsystemets nøkkelring.
pub struct FileKeyStore(std::path::PathBuf);

impl FileKeyStore {
    /// Filnavnet som brukes i datamappen.
    pub const FILE_NAME: &'static str = "utvikling-hovednokkel.bin";

    pub fn in_dir(data_dir: &std::path::Path) -> Self {
        FileKeyStore(data_dir.join(Self::FILE_NAME))
    }
}

impl KeyStore for FileKeyStore {
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, KeyStoreError> {
        match std::fs::read(&self.0) {
            Ok(b) => Ok(Some(Zeroizing::new(b))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(KeyStoreError(e.to_string())),
        }
    }
    fn save(&self, key: &[u8]) -> Result<(), KeyStoreError> {
        if let Some(dir) = self.0.parent() {
            std::fs::create_dir_all(dir).map_err(|e| KeyStoreError(e.to_string()))?;
        }
        std::fs::write(&self.0, key).map_err(|e| KeyStoreError(e.to_string()))
    }
    fn delete(&self) -> Result<(), KeyStoreError> {
        match std::fs::remove_file(&self.0) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(KeyStoreError(e.to_string())),
            _ => Ok(()),
        }
    }
}
