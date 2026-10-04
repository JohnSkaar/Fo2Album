//! Gjenopprettingsnøkkel: lar familien åpne familieprofilen på en ny maskin.
//!
//! 128 tilfeldige bit pluss én kontrollbyte, skrevet som 28 tegn i Crockford base32,
//! gruppert fire og fire: `ABCD-EFGH-JKMN-PQRS-TVWX-YZ01-2345`. Kontrollbyten fanger
//! skrivefeil. Tegn som lett forveksles (O/0, I/L/1) tolkes likt, og U brukes ikke.

use std::fmt;

use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::crypto::{derive, random, KEY_LEN};
use crate::StoreError;

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const SECRET_LEN: usize = 16;
const CHARS: usize = 28;

pub struct RecoveryKey(Zeroizing<[u8; SECRET_LEN]>);

impl RecoveryKey {
    pub fn generate() -> Result<Self, StoreError> {
        let mut s = Zeroizing::new([0u8; SECRET_LEN]);
        random(s.as_mut())?;
        Ok(RecoveryKey(s))
    }

    /// Nøkkelen som pakker inn hovednøkkelen. Hemmeligheten er 128 tilfeldige bit,
    /// så en treg passord-KDF trengs ikke.
    pub(crate) fn wrapping_key(&self) -> Zeroizing<[u8; KEY_LEN]> {
        derive(self.0.as_ref(), b"pho2album/gjenoppretting/v1")
    }

    /// Teksten brukeren skriver ned. Vises bare én gang.
    pub fn display_code(&self) -> Zeroizing<String> {
        let mut payload = [0u8; SECRET_LEN + 1];
        payload[..SECRET_LEN].copy_from_slice(self.0.as_ref());
        payload[SECRET_LEN] = checksum(self.0.as_ref());
        let chars = encode(&payload);
        let mut out = String::with_capacity(CHARS + CHARS / 4);
        for (i, c) in chars.iter().enumerate() {
            if i > 0 && i % 4 == 0 {
                out.push('-');
            }
            out.push(*c as char);
        }
        Zeroizing::new(out)
    }

    /// Tolker en innskrevet kode. Godtar små bokstaver, mellomrom, bindestreker og forvekslinger.
    pub fn parse(input: &str) -> Result<Self, RecoveryParseError> {
        let mut vals = Vec::with_capacity(CHARS);
        for ch in input.chars() {
            if ch == '-' || ch.is_whitespace() {
                continue;
            }
            vals.push(decode_char(ch).ok_or(RecoveryParseError::InvalidCharacter(ch))?);
        }
        if vals.len() != CHARS {
            return Err(RecoveryParseError::WrongLength(vals.len()));
        }
        let bytes = decode(&vals);
        let mut secret = Zeroizing::new([0u8; SECRET_LEN]);
        secret.copy_from_slice(&bytes[..SECRET_LEN]);
        if bytes[SECRET_LEN] != checksum(secret.as_ref()) {
            return Err(RecoveryParseError::Checksum);
        }
        Ok(RecoveryKey(secret))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryParseError {
    InvalidCharacter(char),
    WrongLength(usize),
    Checksum,
}

impl fmt::Display for RecoveryParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryParseError::InvalidCharacter(c) => write!(f, "ugyldig tegn «{c}»"),
            RecoveryParseError::WrongLength(n) => {
                write!(f, "koden skal ha {CHARS} tegn, fant {n}")
            }
            RecoveryParseError::Checksum => write!(f, "koden er skrevet feil (kontrollsum)"),
        }
    }
}

impl std::error::Error for RecoveryParseError {}

fn checksum(secret: &[u8]) -> u8 {
    Sha256::digest([b"pho2album/kontroll/v1".as_slice(), secret].concat())[0]
}

/// 17 byte (136 bit) → 28 tegn à 5 bit (140 bit, de siste 4 er null).
fn encode(bytes: &[u8]) -> [u8; CHARS] {
    let mut out = [0u8; CHARS];
    let (mut acc, mut bits, mut i) = (0u32, 0u32, 0usize);
    for &b in bytes {
        acc = (acc << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out[i] = ALPHABET[((acc >> bits) & 31) as usize];
            i += 1;
        }
    }
    if bits > 0 {
        out[i] = ALPHABET[((acc << (5 - bits)) & 31) as usize];
    }
    out
}

fn decode(vals: &[u8]) -> [u8; SECRET_LEN + 1] {
    let mut out = [0u8; SECRET_LEN + 1];
    let (mut acc, mut bits, mut i) = (0u32, 0u32, 0usize);
    for &v in vals {
        acc = (acc << 5) | v as u32;
        bits += 5;
        if bits >= 8 && i < out.len() {
            bits -= 8;
            out[i] = (acc >> bits) as u8;
            i += 1;
        }
    }
    out
}

fn decode_char(c: char) -> Option<u8> {
    let c = match c.to_ascii_uppercase() {
        'O' => '0',
        'I' | 'L' => '1',
        c => c,
    };
    ALPHABET
        .iter()
        .position(|&a| a as char == c)
        .map(|p| p as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_has_expected_format() {
        let code = RecoveryKey::generate().unwrap().display_code();
        assert_eq!(code.len(), 34);
        assert_eq!(code.split('-').count(), 7);
        assert!(code
            .chars()
            .all(|c| c == '-' || ALPHABET.contains(&(c as u8))));
    }

    #[test]
    fn code_round_trips_and_tolerates_typing_variations() {
        let key = RecoveryKey::generate().unwrap();
        let code = key.display_code();
        let parsed = RecoveryKey::parse(&code).unwrap();
        assert_eq!(*parsed.0, *key.0);
        let sloppy = code.to_lowercase().replace('-', " ").replace('0', "o");
        assert_eq!(*RecoveryKey::parse(&sloppy).unwrap().0, *key.0);
    }

    #[test]
    fn typos_are_detected() {
        let code = RecoveryKey::generate().unwrap().display_code().to_string();
        let mut chars: Vec<char> = code.chars().collect();
        // Bytt ett tegn i hemmeligheten; kontrollbyten skal avsløre det (1/256 risiko for å
        // ikke merke det, så vi prøver flere posisjoner og krever at de fleste fanges).
        let mut caught = 0;
        for pos in [0usize, 5, 10, 15, 20] {
            let orig = chars[pos];
            chars[pos] = if orig == 'A' { 'B' } else { 'A' };
            let typed: String = chars.iter().collect();
            if matches!(
                RecoveryKey::parse(&typed),
                Err(RecoveryParseError::Checksum)
            ) {
                caught += 1;
            }
            chars[pos] = orig;
        }
        assert!(caught >= 4, "fanget bare {caught} av 5 skrivefeil");
        assert_eq!(
            RecoveryKey::parse("ABCD").err(),
            Some(RecoveryParseError::WrongLength(4))
        );
        assert_eq!(
            RecoveryKey::parse(&code.replacen(|c: char| c.is_ascii_alphanumeric(), "U", 1)).err(),
            Some(RecoveryParseError::InvalidCharacter('U'))
        );
    }
}
