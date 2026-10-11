//! Filer som bare ligger i skyen.

use p2a_core::{FileStatus, SourceKind};
use p2a_store::{FileEntry, MemoryKeyStore, Store};

#[test]
fn skyfilene_listes_med_full_sti_og_blir_lokale_etter_henting() {
    let dir = tempfile::tempdir().unwrap();
    let keys = MemoryKeyStore::default();
    let (mut store, _) = Store::create(dir.path(), &keys).unwrap();
    let src = store
        .add_source(SourceKind::Icloud, dir.path(), "iCloud Photos")
        .unwrap();
    let entry = |name: &str, status| FileEntry {
        rel_path: name.into(),
        size: 100,
        modified: 1_300_000_000,
        status,
    };
    store
        .sync_files(
            src,
            &[
                entry("2011/IMG_0001.JPG", FileStatus::CloudOnly),
                entry("IMG_0002.JPG", FileStatus::Local),
            ],
        )
        .unwrap();
    let sky = store.cloud_files().unwrap();
    assert_eq!(sky.len(), 1);
    assert_eq!(sky[0].path, dir.path().join("2011").join("IMG_0001.JPG"));
    assert_eq!(sky[0].modified, 1_300_000_000);

    // Når filen er lastet ned, ser neste skanning den som lokal, og den skal leses inn.
    let (_, to_read) = store
        .sync_files(
            src,
            &[
                entry("2011/IMG_0001.JPG", FileStatus::Local),
                entry("IMG_0002.JPG", FileStatus::Local),
            ],
        )
        .unwrap();
    assert!(store.cloud_files().unwrap().is_empty());
    assert!(to_read.iter().any(|f| f.rel_path == "2011/IMG_0001.JPG"));
}
