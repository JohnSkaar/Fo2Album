//! Kobling mellom `p2a_store::KeyStore` og operativsystemets nøkkelring. Brukes av appen
//! og av `p2a`-verktøyet, så begge åpner den samme krypterte katalogen.

use p2a_store::{KeyStore, KeyStoreError};
use zeroize::Zeroizing;

#[cfg(any(target_os = "macos", windows))]
pub use os::OsKeyStore as PlatformKeyStore;

#[cfg(not(any(target_os = "macos", windows)))]
pub use dev::DevFileKeyStore as PlatformKeyStore;

#[cfg(any(target_os = "macos", windows))]
mod os {
    use super::*;

    const SERVICE: &str = "no.fo2album.app";
    const ACCOUNT: &str = "hovednokkel";

    /// Nøkkelring på Mac, Credential Manager på Windows.
    pub struct OsKeyStore;

    impl OsKeyStore {
        pub fn new(_data_dir: &std::path::Path) -> Self {
            OsKeyStore
        }

        fn entry() -> Result<keyring::Entry, KeyStoreError> {
            keyring::Entry::new(SERVICE, ACCOUNT).map_err(|e| KeyStoreError(e.to_string()))
        }
    }

    impl KeyStore for OsKeyStore {
        fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, KeyStoreError> {
            match Self::entry()?.get_secret() {
                Ok(secret) => Ok(Some(Zeroizing::new(secret))),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(e) => Err(KeyStoreError(e.to_string())),
            }
        }

        fn save(&self, key: &[u8]) -> Result<(), KeyStoreError> {
            Self::entry()?
                .set_secret(key)
                .map_err(|e| KeyStoreError(e.to_string()))
        }

        fn delete(&self) -> Result<(), KeyStoreError> {
            match Self::entry()?.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(e) => Err(KeyStoreError(e.to_string())),
            }
        }
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
mod dev {
    use super::*;

    /// Bare for utvikling på Linux: nøkkelen ligger i en fil ved siden av dataene.
    /// Appen leveres bare for Mac og Windows, som bruker OS-ets nøkkelring.
    pub struct DevFileKeyStore(p2a_store::FileKeyStore);

    impl DevFileKeyStore {
        pub fn new(data_dir: &std::path::Path) -> Self {
            eprintln!("ADVARSEL: utviklingsmodus på Linux – hovednøkkelen lagres i en fil, ikke i en nøkkelring");
            DevFileKeyStore(p2a_store::FileKeyStore::in_dir(data_dir))
        }
    }

    impl KeyStore for DevFileKeyStore {
        fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, KeyStoreError> {
            self.0.load()
        }
        fn save(&self, key: &[u8]) -> Result<(), KeyStoreError> {
            self.0.save(key)
        }
        fn delete(&self) -> Result<(), KeyStoreError> {
            self.0.delete()
        }
    }
}
