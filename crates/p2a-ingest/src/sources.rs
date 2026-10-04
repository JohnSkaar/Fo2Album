//! Kildetyper og vanlige plasseringer for bildemapper.
//!
//! Appen kan foreslå kjente stier, men bare etter samtykke (PRODUCT.md, brukerflyt steg 1).
//! Ingen mapper leses før brukeren har valgt dem.

use std::path::{Path, PathBuf};

use p2a_core::SourceKind;

/// Gjetter kildetypen ut fra stien. Faller tilbake til `Pc`.
pub fn detect_kind(path: &Path) -> SourceKind {
    let p = path.to_string_lossy().to_lowercase().replace('\\', "/");
    if p.contains("dropbox") {
        SourceKind::Dropbox
    } else if p.contains("icloud") || p.contains("mobile documents") {
        SourceKind::Icloud
    } else if p.contains("googledrive")
        || p.contains("google drive")
        || p.contains("/my drive")
        || p.contains("/min disk")
    {
        SourceKind::GoogleDisk
    } else {
        SourceKind::Pc
    }
}

/// En foreslått mappe som finnes på maskinen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub kind: SourceKind,
    pub path: PathBuf,
}

/// Kjente plasseringer relativt til hjemmemappen, for Mac og Windows.
/// `*` matcher én mappe (f.eks. `GoogleDrive-navn@epost`).
const CANDIDATES: &[(SourceKind, &str)] = &[
    (SourceKind::Pc, "Pictures"),
    (SourceKind::Pc, "Bilder"),
    (SourceKind::Dropbox, "Dropbox/Camera Uploads"),
    (SourceKind::Dropbox, "Dropbox/Kameraopplastinger"),
    (SourceKind::Dropbox, "Dropbox"),
    (
        SourceKind::Dropbox,
        "Library/CloudStorage/Dropbox/Camera Uploads",
    ),
    (SourceKind::Dropbox, "Library/CloudStorage/Dropbox"),
    (
        SourceKind::GoogleDisk,
        "Library/CloudStorage/GoogleDrive-*/My Drive",
    ),
    (
        SourceKind::GoogleDisk,
        "Library/CloudStorage/GoogleDrive-*/Min disk",
    ),
    (SourceKind::GoogleDisk, "Google Drive"),
    (SourceKind::GoogleDisk, "My Drive"),
    (SourceKind::Icloud, "Pictures/iCloud Photos/Photos"),
    (SourceKind::Icloud, "Pictures/iCloud Photos"),
    (SourceKind::Icloud, "iCloudDrive"),
];

/// Foreslår mapper som finnes under `home`. Mer spesifikke stier vinner: finnes
/// `Dropbox/Camera Uploads`, foreslås ikke hele `Dropbox` i tillegg.
pub fn suggested_sources(home: &Path) -> Vec<Suggestion> {
    let mut found: Vec<Suggestion> = Vec::new();
    for (kind, rel) in CANDIDATES {
        for path in expand(home, rel) {
            if !path.is_dir() {
                continue;
            }
            let covered = found
                .iter()
                .any(|s| path.starts_with(&s.path) || s.path.starts_with(&path));
            if !covered {
                found.push(Suggestion { kind: *kind, path });
            }
        }
    }
    found
}

fn expand(base: &Path, rel: &str) -> Vec<PathBuf> {
    let mut paths = vec![base.to_path_buf()];
    for part in rel.split('/') {
        let mut next = Vec::new();
        for p in &paths {
            if let Some(prefix) = part.strip_suffix('*') {
                for e in std::fs::read_dir(p).into_iter().flatten().flatten() {
                    if e.file_name().to_string_lossy().starts_with(prefix) {
                        next.push(e.path());
                    }
                }
            } else {
                next.push(p.join(part));
            }
        }
        paths = next;
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_kind_from_path() {
        let k = |p: &str| detect_kind(Path::new(p));
        assert_eq!(k("/Users/kari/Dropbox/Camera Uploads"), SourceKind::Dropbox);
        assert_eq!(
            k("/Users/kari/Library/CloudStorage/Dropbox"),
            SourceKind::Dropbox
        );
        assert_eq!(
            k(r"C:\Users\kari\Pictures\iCloud Photos\Photos"),
            SourceKind::Icloud
        );
        assert_eq!(
            k("/Users/kari/Library/CloudStorage/GoogleDrive-kari@example.com/Min disk/Bilder"),
            SourceKind::GoogleDisk
        );
        assert_eq!(k(r"G:\My Drive\Bilder"), SourceKind::GoogleDisk);
        assert_eq!(k("/Users/kari/Pictures/2011"), SourceKind::Pc);
    }

    #[test]
    fn suggests_existing_folders_most_specific_first() {
        let home = tempfile::tempdir().unwrap();
        let mk = |p: &str| std::fs::create_dir_all(home.path().join(p)).unwrap();
        mk("Pictures/iCloud Photos/Photos");
        mk("Dropbox/Camera Uploads");
        mk("Library/CloudStorage/GoogleDrive-kari@example.com/Min disk");

        let s = suggested_sources(home.path());
        let rel: Vec<(SourceKind, String)> = s
            .iter()
            .map(|s| {
                (
                    s.kind,
                    s.path
                        .strip_prefix(home.path())
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                )
            })
            .collect();
        assert!(rel.contains(&(SourceKind::Pc, "Pictures".into())));
        assert!(rel.contains(&(SourceKind::Dropbox, "Dropbox/Camera Uploads".into())));
        assert!(rel.contains(&(
            SourceKind::GoogleDisk,
            "Library/CloudStorage/GoogleDrive-kari@example.com/Min disk".into()
        )));
        assert!(
            !rel.iter().any(|(_, p)| p == "Dropbox"),
            "hele Dropbox skal ikke foreslås i tillegg"
        );
    }
}
