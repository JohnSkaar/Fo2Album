//! Innholdshash (BLAKE3). Samme bytes gir samme hash uansett filnavn eller kilde,
//! så eksakte dubletter på tvers av Dropbox og iCloud blir ett bilde (SCORING.md §2).

use std::path::Path;

use p2a_core::ContentHash;

pub fn hash_file(path: &Path) -> std::io::Result<ContentHash> {
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(std::fs::File::open(path)?)?;
    Ok(ContentHash(*hasher.finalize().as_bytes()))
}

pub fn hash_bytes(bytes: &[u8]) -> ContentHash {
    ContentHash(*blake3::hash(bytes).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_and_bytes_agree() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.jpg");
        std::fs::write(&p, b"innhold").unwrap();
        assert_eq!(hash_file(&p).unwrap(), hash_bytes(b"innhold"));
        assert_ne!(hash_bytes(b"innhold"), hash_bytes(b"innhold2"));
    }
}
