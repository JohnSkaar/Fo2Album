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
