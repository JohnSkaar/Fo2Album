//! Ende-til-ende: to kilder med overlappende bilder inn i den krypterte katalogen.

use std::path::Path;
use std::sync::atomic::AtomicBool;

use p2a_core::{DateSource, SourceKind};
use p2a_ingest::pipeline::{ingest_all, Phase, Progress};
use p2a_ingest::synth::{jpeg, ExifSpec};
use p2a_store::{MemoryKeyStore, Store};

fn write(root: &Path, rel: &str, bytes: &[u8]) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, bytes).unwrap();
}

fn exif_at(taken: &str) -> ExifSpec<'_> {
    ExifSpec {
        taken: Some(taken),
        make: Some("Apple"),
        model: Some("iPhone 4"),
        ..Default::default()
    }
}

#[test]
fn two_sources_with_exact_duplicates() {
    let lib = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let dropbox = lib.path().join("Dropbox/Camera Uploads");
    let icloud = lib.path().join("Pictures/iCloud Photos/Photos");

    let summer = jpeg(64, 48, 1, Some(&exif_at("2011:07:14 12:34:56")));
    let christmas = jpeg(64, 48, 2, Some(&exif_at("2011:12:24 17:00:00")));
    let from_name = jpeg(64, 48, 3, None);
    let no_date = jpeg(64, 48, 4, None);

    // Samme sommerbilde i begge kilder (eksakt dublett), med ulike navn.
    write(&dropbox, "2011-07-14 12.34.56.jpg", &summer);
    write(&icloud, "IMG_0001.JPG", &summer);
    write(&icloud, "IMG_0002.JPG", &christmas);
    write(&dropbox, "IMG_20100601_101010.jpg", &from_name);
    write(&dropbox, "ukjent.jpg", &no_date);
    // Bare i skyen: skal telles, men ikke leses.
    write(&icloud, ".IMG_0003.HEIC.icloud", b"plist");

    let keys = MemoryKeyStore::default();
    let (mut store, _) = Store::create(data.path(), &keys).unwrap();
    store
        .add_source(SourceKind::Dropbox, &dropbox, "Dropbox")
        .unwrap();
    store
        .add_source(SourceKind::Icloud, &icloud, "iCloud Bilder")
        .unwrap();

    let mut phases = Vec::new();
    let report = ingest_all(&mut store, &AtomicBool::new(false), &mut |p: Progress| {
        phases.push(p.phase)
    })
    .unwrap();

    assert!(phases.contains(&Phase::Skanner) && phases.contains(&Phase::Leser));
    assert_eq!(report.added, 6);
    assert_eq!(report.read, 5, "plassholderen skal ikke leses");
    assert_eq!(report.new_photos, 4);
    let s = &report.summary;
    assert_eq!((s.sources, s.files, s.cloud_only), (2, 6, 1));
    assert_eq!(s.photos, 4);
    assert_eq!(s.exact_duplicates, 1);
    assert_eq!(
        s.uncertain_dates, 1,
        "bare ukjent.jpg har dato fra endringstid"
    );

    // 2011 har sommer- og julebildet; sommerbildet finnes i begge kilder.
    let y2011 = store.photos_in_year(2011).unwrap();
    assert_eq!(y2011.len(), 2);
    assert_eq!(y2011[0].taken_at.unwrap().to_iso(), "2011-07-14T12:34:56");
    assert_eq!(
        y2011[0].sources,
        vec![SourceKind::Dropbox, SourceKind::Icloud]
    );
    assert_eq!(y2011[1].sources, vec![SourceKind::Icloud]);
    assert_eq!(y2011[0].date_source, Some(DateSource::Exif));
    let y2010 = store.photos_in_year(2010).unwrap();
    assert_eq!(y2010[0].date_source, Some(DateSource::Filename));
    assert!(store.years().unwrap().contains(&(2011, 2)));

    // Ny kjøring uten endringer: ingenting leses.
    let again = ingest_all(&mut store, &AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!((again.added, again.changed, again.read), (0, 0, 0));
    assert_eq!(again.summary, report.summary);

    // Julebildet slettes i iCloud: filen og bildet forsvinner fra katalogen.
    std::fs::remove_file(icloud.join("IMG_0002.JPG")).unwrap();
    let after = ingest_all(&mut store, &AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(after.removed, 1);
    assert_eq!(after.summary.photos, 3);
    assert_eq!(store.photos_in_year(2011).unwrap().len(), 1);

    // Fjerner vi Dropbox som kilde, står sommerbildet igjen via iCloud.
    let dropbox_id = store.sources().unwrap()[0].id;
    store.remove_source(dropbox_id).unwrap();
    assert_eq!(store.summary().unwrap().photos, 1);
    assert!(
        lib.path()
            .join("Dropbox/Camera Uploads/ukjent.jpg")
            .exists(),
        "filene på disken røres ikke"
    );
}

#[test]
fn missing_source_is_reported_not_deleted() {
    let data = tempfile::tempdir().unwrap();
    let keys = MemoryKeyStore::default();
    let (mut store, _) = Store::create(data.path(), &keys).unwrap();
    store
        .add_source(
            SourceKind::Pc,
            Path::new("/finnes/ikke/Bilder"),
            "Ekstern disk",
        )
        .unwrap();
    let report = ingest_all(&mut store, &AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(report.missing_sources, vec!["Ekstern disk".to_string()]);
    assert_eq!(store.sources().unwrap().len(), 1);
}

#[test]
fn cancel_stops_before_reading() {
    let lib = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    for i in 0..10 {
        write(
            lib.path(),
            &format!("IMG_{i:04}.jpg"),
            &jpeg(16, 16, i, None),
        );
    }
    let keys = MemoryKeyStore::default();
    let (mut store, _) = Store::create(data.path(), &keys).unwrap();
    store
        .add_source(SourceKind::Pc, lib.path(), "Bilder")
        .unwrap();
    let report = ingest_all(&mut store, &AtomicBool::new(true), &mut |_| {}).unwrap();
    assert!(report.cancelled);
    assert_eq!(report.read, 0);
    // Neste kjøring fortsetter der den slapp.
    let report = ingest_all(&mut store, &AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!((report.read, report.summary.photos), (10, 10));
}
