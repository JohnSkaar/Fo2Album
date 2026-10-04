//! Nøkler og autentisert kryptering.
//!
//! Én tilfeldig hovednøkkel (256 bit) per installasjon. Undernøkler avledes med
//! HKDF-SHA256, så hver bruk (database, miniatyrer, sikkerhetskopi) har sin egen nøkkel.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::{Zeroize, Zeroizing};

use crate::StoreError;

pub const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 24;

/// Formål for avledede nøkler. Strengen må aldri endres for en eksisterende versjon.
#[derive(Debug, Clone, Copy)]
pub enum Purpose {
    Database,
    Thumbnails,
}

impl Purpose {
    /// Etikettene beholder det gamle navnet med vilje: de er en del av krypteringen, og
    /// endres de, kan eksisterende data ikke åpnes.
    fn info(self) -> &'static [u8] {
        match self {
            Purpose::Database => b"pho2album/database/v1",
            Purpose::Thumbnails => b"pho2album/miniatyrer/v1",
        }
    }
}

/// Hovednøkkelen. Nullstilles i minnet når den slippes.
pub struct MasterKey(Zeroizing<[u8; KEY_LEN]>);

impl MasterKey {
    pub fn generate() -> Result<Self, StoreError> {
        let mut k = Zeroizing::new([0u8; KEY_LEN]);
        random(k.as_mut())?;
        Ok(MasterKey(k))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StoreError> {
        let arr: [u8; KEY_LEN] = bytes.try_into().map_err(|_| StoreError::CorruptKey)?;
        Ok(MasterKey(Zeroizing::new(arr)))
    }

    pub fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.0
    }

    pub fn derive(&self, purpose: Purpose) -> Zeroizing<[u8; KEY_LEN]> {
        derive(self.as_bytes(), purpose.info())
    }
}

pub(crate) fn derive(ikm: &[u8], info: &[u8]) -> Zeroizing<[u8; KEY_LEN]> {
    let hk = Hkdf::<Sha256>::new(None, ikm);
    let mut out = Zeroizing::new([0u8; KEY_LEN]);
    hk.expand(info, out.as_mut())
        .expect("32 byte er en gyldig HKDF-lengde");
    out
}

pub(crate) fn random(buf: &mut [u8]) -> Result<(), StoreError> {
    getrandom::fill(buf).map_err(|e| StoreError::Random(e.to_string()))
}

/// Krypterer med XChaCha20-Poly1305. Utdata: nonce (24 byte) || chiffertekst || tag.
/// `aad` bindes til chifferteksten (f.eks. innholdshashen), så filer ikke kan byttes om.
pub fn seal(key: &[u8; KEY_LEN], plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, StoreError> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let mut nonce = [0u8; NONCE_LEN];
    random(&mut nonce)?;
    let ct = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| StoreError::Decrypt)?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn open(key: &[u8; KEY_LEN], sealed: &[u8], aad: &[u8]) -> Result<Vec<u8>, StoreError> {
    if sealed.len() < NONCE_LEN + 16 {
        return Err(StoreError::Decrypt);
    }
    let (nonce, ct) = sealed.split_at(NONCE_LEN);
    let cipher = XChaCha20Poly1305::new(key.into());
    cipher
        .decrypt(XNonce::from_slice(nonce), Payload { msg: ct, aad })
        .map_err(|_| StoreError::Decrypt)
}

pub(crate) fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

pub(crate) fn from_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Hex-streng som nullstilles i minnet etter bruk (for `PRAGMA key`).
pub(crate) fn secret_hex(bytes: &[u8]) -> Zeroizing<String> {
    let mut s = to_hex(bytes);
    let z = Zeroizing::new(s.clone());
    s.zeroize();
    z
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_and_open_round_trip() {
        let k = MasterKey::generate().unwrap();
        let key = k.derive(Purpose::Thumbnails);
        let sealed = seal(&key, b"hemmelig", b"aad").unwrap();
        assert_eq!(open(&key, &sealed, b"aad").unwrap(), b"hemmelig");
    }

    #[test]
    fn open_fails_with_wrong_key_aad_or_tampering() {
        let k1 = MasterKey::generate().unwrap().derive(Purpose::Thumbnails);
        let k2 = MasterKey::generate().unwrap().derive(Purpose::Thumbnails);
        let mut sealed = seal(&k1, b"hemmelig", b"aad").unwrap();
        assert!(open(&k2, &sealed, b"aad").is_err());
        assert!(open(&k1, &sealed, b"annen").is_err());
        let last = sealed.len() - 1;
        sealed[last] ^= 1;
        assert!(open(&k1, &sealed, b"aad").is_err());
        assert!(open(&k1, &[0u8; 10], b"aad").is_err());
    }

    #[test]
    fn purposes_give_different_keys() {
        let k = MasterKey::generate().unwrap();
        assert_ne!(*k.derive(Purpose::Database), *k.derive(Purpose::Thumbnails));
    }

    #[test]
    fn hex_round_trip() {
        assert_eq!(
            from_hex(&to_hex(&[0, 1, 254, 255])),
            Some(vec![0, 1, 254, 255])
        );
        assert_eq!(from_hex("abc"), None);
        assert_eq!(from_hex("zz"), None);
    }
}
