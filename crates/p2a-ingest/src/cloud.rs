//! «Hent bildene fra skyen»: filer som bare ligger i skyen (Dropbox, iCloud, OneDrive, Google
//! Disk), lastes ned til maskinen av synkroniseringsprogrammet selv. Appen åpner og leser filen,
//! slik et bildeprogram ville gjort, og da henter Windows (Cloud Files) og macOS (File Provider)
//! innholdet. Appen snakker ikke med noen server selv, og ingenting sendes fra maskinen.
//!
//! Bare årets bilder hentes: året gjettes fra filnavnet (IMG_20110812_…) eller endringstiden,
//! siden EXIF ikke kan leses før filen er lastet ned.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use p2a_core::TakenAt;

/// Hvor lenge appen venter på at en gammel iCloud-plassholder (`.navn.icloud`) blir lastet ned.
const ICLOUD_WAIT: Duration = Duration::from_secs(120);

/// Året bildet trolig er fra: datoen i filnavnet, ellers endringstiden.
pub fn year_guess(path: &Path, modified: i64) -> Option<i32> {
    let name = path.file_name()?.to_string_lossy();
    crate::dates::from_filename(&name)
        .or_else(|| TakenAt::from_unix_utc(modified))
        .map(|t| t.year)
}

/// Den gamle iCloud Drive-plassholderen for `path` (`.IMG_0001.JPG.icloud` ved siden av).
fn icloud_placeholder(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_string_lossy();
    Some(path.with_file_name(format!(".{name}.icloud")))
}

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("avbrutt")]
    Cancelled,
    #[error("filen finnes ikke på maskinen")]
    Missing,
    #[error("skyen svarte ikke: {0}")]
    Io(#[from] std::io::Error),
}

/// Får synkroniseringsprogrammet til å laste ned filen: les den til slutten. For gamle
/// iCloud-plassholdere på Mac bes iCloud om å laste ned (`brctl download`), og appen venter.
pub fn fetch(path: &Path, cancel: &AtomicBool) -> Result<(), FetchError> {
    if !path.exists() {
        let placeholder = icloud_placeholder(path).filter(|p| p.exists());
        let Some(_placeholder) = placeholder else {
            return Err(FetchError::Missing);
        };
        request_icloud_download(path)?;
        let start = Instant::now();
        while !path.exists() {
            if cancel.load(Ordering::Relaxed) {
                return Err(FetchError::Cancelled);
            }
            if start.elapsed() > ICLOUD_WAIT {
                return Err(FetchError::Io(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "iCloud lastet ikke ned filen i tide",
                )));
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }
    let mut file = std::fs::File::open(path)?;
    let mut buf = vec![0u8; 1 << 20];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(FetchError::Cancelled);
        }
        if file.read(&mut buf)? == 0 {
            return Ok(());
        }
    }
}

#[cfg(target_os = "macos")]
fn request_icloud_download(path: &Path) -> Result<(), FetchError> {
    let status = std::process::Command::new("brctl")
        .arg("download")
        .arg(path)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(FetchError::Io(std::io::Error::other(
            "brctl download feilet",
        )))
    }
}

#[cfg(not(target_os = "macos"))]
fn request_icloud_download(_path: &Path) -> Result<(), FetchError> {
    Err(FetchError::Missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn year_from_name_then_modified_time() {
        assert_eq!(
            year_guess(Path::new("/a/IMG_20110812_101500.jpg"), 0),
            Some(2011)
        );
        // 1. juli 2015 (UTC).
        assert_eq!(
            year_guess(Path::new("/a/DSC0001.JPG"), 1_435_708_800),
            Some(2015)
        );
    }

    #[test]
    fn fetch_reads_local_files_and_reports_missing() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("IMG_0001.JPG");
        std::fs::write(&f, vec![7u8; 3 << 20]).unwrap();
        let cancel = AtomicBool::new(false);
        fetch(&f, &cancel).unwrap();
        assert!(matches!(
            fetch(&dir.path().join("borte.jpg"), &cancel),
            Err(FetchError::Missing)
        ));
        cancel.store(true, Ordering::Relaxed);
        assert!(matches!(fetch(&f, &cancel), Err(FetchError::Cancelled)));
    }
}
