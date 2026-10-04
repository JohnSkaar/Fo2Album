//! Rask gjennomgang av en bildemappe. Leser bare metadata fra filsystemet, ikke innholdet,
//! så filer som bare ligger i skyen ikke blir lastet ned (ARCHITECTURE.md, «Kilder»).

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use walkdir::WalkDir;

/// Filtyper vi leser i v1 (PRODUCT.md). Videoer og RAW hoppes over.
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "jpe", "heic", "heif", "hif", "png", "webp"];

/// Mapper som aldri inneholder familiebilder, eller som leses på annen måte.
const SKIP_DIRS: &[&str] = &[
    "@eaDir",
    "$RECYCLE.BIN",
    "System Volume Information",
    "node_modules",
];

/// Apple Bilder-biblioteket leses via PhotoKit (M1b), aldri som mappe.
const SKIP_DIR_SUFFIXES: &[&str] = &[".photoslibrary", ".app", ".aplibrary", ".lrdata"];

pub use p2a_core::FileStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFile {
    /// Relativ sti med `/` som skilletegn, uavhengig av OS.
    pub rel_path: String,
    pub size: u64,
    /// Sekunder siden 1970 (UTC).
    pub modified: i64,
    pub status: FileStatus,
}

pub fn is_image_name(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, ext)| IMAGE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

fn skip_dir(name: &str) -> bool {
    name.starts_with('.')
        || SKIP_DIRS.contains(&name)
        || SKIP_DIR_SUFFIXES
            .iter()
            .any(|s| name.to_ascii_lowercase().ends_with(s))
}

/// Gammel iCloud Drive-plassholder: `.IMG_0001.JPG.icloud` står for `IMG_0001.JPG`.
fn icloud_placeholder_target(name: &str) -> Option<&str> {
    name.strip_prefix('.')?.strip_suffix(".icloud")
}

/// Går gjennom `root` og returnerer bildefiler, sortert på sti.
pub fn scan(root: &Path) -> std::io::Result<Vec<ScannedFile>> {
    let mut out = Vec::new();
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            e.depth() == 0
                || !(e.file_type().is_dir() && skip_dir(&e.file_name().to_string_lossy()))
        });
    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            // Mapper vi ikke har tilgang til, hoppes over.
            Err(e) if e.io_error().is_some() && e.depth() > 0 => continue,
            Err(e) => return Err(e.into()),
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        let (name, forced_cloud) = match icloud_placeholder_target(&name) {
            Some(target) => (target.to_string(), true),
            None if name.starts_with('.') => continue,
            None => (name.to_string(), false),
        };
        if !is_image_name(&name) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let rel_dir = entry
            .path()
            .parent()
            .and_then(|p| p.strip_prefix(root).ok())
            .map(to_slash)
            .unwrap_or_default();
        let rel_path = if rel_dir.is_empty() {
            name
        } else {
            format!("{rel_dir}/{name}")
        };
        out.push(ScannedFile {
            rel_path,
            size: if forced_cloud { 0 } else { meta.len() },
            modified: meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            status: if forced_cloud || platform::is_cloud_placeholder(&meta) {
                FileStatus::CloudOnly
            } else {
                FileStatus::Local
            },
        });
    }
    out.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(out)
}

fn to_slash(p: &Path) -> String {
    p.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Full sti for en relativ sti fra [`scan`].
pub fn full_path(root: &Path, rel_path: &str) -> PathBuf {
    rel_path
        .split('/')
        .fold(root.to_path_buf(), |p, part| p.join(part))
}

/// Gjenkjenning av sky-plassholdere uten å lese innholdet.
mod platform {
    #[cfg(windows)]
    pub fn is_cloud_placeholder(meta: &std::fs::Metadata) -> bool {
        use std::os::windows::fs::MetadataExt;
        super::windows_attrs_are_placeholder(meta.file_attributes())
    }

    #[cfg(target_os = "macos")]
    pub fn is_cloud_placeholder(meta: &std::fs::Metadata) -> bool {
        use std::os::macos::fs::MetadataExt;
        super::macos_flags_are_placeholder(meta.st_flags())
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    pub fn is_cloud_placeholder(_meta: &std::fs::Metadata) -> bool {
        false
    }
}

/// Windows Cloud Files (OneDrive, Dropbox, Google Drive, iCloud for Windows).
#[cfg_attr(not(windows), allow(dead_code))]
fn windows_attrs_are_placeholder(attrs: u32) -> bool {
    const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000;
    const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
    const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
    attrs
        & (FILE_ATTRIBUTE_OFFLINE
            | FILE_ATTRIBUTE_RECALL_ON_OPEN
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
        != 0
}

/// macOS File Provider (Dropbox og Google Drive i `~/Library/CloudStorage`, iCloud Drive).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn macos_flags_are_placeholder(flags: u32) -> bool {
    const SF_DATALESS: u32 = 0x4000_0000;
    flags & SF_DATALESS != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_image_names() {
        for n in ["a.jpg", "B.JPEG", "c.heic", "d.HEIF", "e.png", "f.webp"] {
            assert!(is_image_name(n), "{n}");
        }
        for n in ["a.mov", "b.mp4", "c.txt", "d.CR2", "noext", "e.aae"] {
            assert!(!is_image_name(n), "{n}");
        }
    }

    #[test]
    fn placeholder_flags() {
        assert!(windows_attrs_are_placeholder(0x0040_0000 | 0x20));
        assert!(windows_attrs_are_placeholder(0x0004_0000));
        assert!(!windows_attrs_are_placeholder(0x20)); // ARCHIVE
        assert!(macos_flags_are_placeholder(0x4000_0000));
        assert!(!macos_flags_are_placeholder(0));
    }

    #[test]
    fn scans_images_and_skips_junk() {
        let dir = tempfile::tempdir().unwrap();
        let mk = |p: &str, bytes: &[u8]| {
            let path = dir.path().join(p);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        };
        mk("2011/juli/IMG_0001.JPG", b"abc");
        mk("2011/juli/IMG_0001.MOV", b"video");
        mk("2011/.skjult/IMG_0002.jpg", b"x");
        mk("2011/@eaDir/IMG_0001.JPG/SYNOFILE_THUMB_M.jpg", b"x");
        mk("Photos Library.photoslibrary/originals/A.heic", b"x");
        mk("2011/.IMG_0003.HEIC.icloud", b"plist");
        mk("2011/.DS_Store", b"x");
        mk("rot.png", b"png");

        let files = scan(dir.path()).unwrap();
        let paths: Vec<&str> = files.iter().map(|f| f.rel_path.as_str()).collect();
        assert_eq!(
            paths,
            ["2011/IMG_0003.HEIC", "2011/juli/IMG_0001.JPG", "rot.png"]
        );
        assert_eq!(files[0].status, FileStatus::CloudOnly);
        assert_eq!(files[1].status, FileStatus::Local);
        assert_eq!(files[1].size, 3);
        assert_eq!(
            full_path(dir.path(), &files[1].rel_path),
            dir.path().join("2011").join("juli").join("IMG_0001.JPG")
        );
    }
}
