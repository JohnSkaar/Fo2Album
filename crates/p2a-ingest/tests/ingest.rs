//! Ende-til-ende: to kilder med overlappende bilder inn i den krypterte katalogen.

use std::path::Path;
use std::sync::atomic::AtomicBool;

use p2a_core::config::DedupConfig;
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
    let report = ingest_all(
        &mut store,
        &DedupConfig::default(),
        &AtomicBool::new(false),
        &mut |p: Progress| phases.push(p.phase),
    )
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
    let again = ingest_all(
        &mut store,
        &DedupConfig::default(),
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!((again.added, again.changed, again.read), (0, 0, 0));
    assert_eq!(again.summary, report.summary);

    // Julebildet slettes i iCloud: filen og bildet forsvinner fra katalogen.
    std::fs::remove_file(icloud.join("IMG_0002.JPG")).unwrap();
    let after = ingest_all(
        &mut store,
        &DedupConfig::default(),
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
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
    let report = ingest_all(
        &mut store,
        &DedupConfig::default(),
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
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
    let report = ingest_all(
        &mut store,
        &DedupConfig::default(),
        &AtomicBool::new(true),
        &mut |_| {},
    )
    .unwrap();
    assert!(report.cancelled);
    assert_eq!(report.read, 0);
    // Neste kjøring fortsetter der den slapp.
    let report = ingest_all(
        &mut store,
        &DedupConfig::default(),
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!((report.read, report.summary.photos), (10, 10));
}

#[test]
fn transcoded_copies_corrupt_files_and_thumbnails() {
    use image::imageops::FilterType;
    use p2a_ingest::synth::{encode_jpeg, insert_exif, pattern, tiff};

    let lib = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let icloud = lib.path().join("iCloud Photos");
    let dropbox = lib.path().join("Dropbox");

    // Original i høy oppløsning med EXIF.
    let scene = pattern(1600, 1200, 42);
    let exif = |t| {
        tiff(&ExifSpec {
            taken: Some(t),
            make: Some("Apple"),
            ..Default::default()
        })
    };
    write(
        &icloud,
        "IMG_0100.JPG",
        &insert_exif(&encode_jpeg(&scene, 92), &exif("2011:03:02 08:15:00")),
    );
    // Samme bilde sendt på WhatsApp: nedskalert, komprimert, uten EXIF.
    let small = image::imageops::resize(&scene, 800, 600, FilterType::Triangle);
    write(
        &dropbox,
        "WhatsApp Image 2011-03-02 at 09.00.00.jpeg",
        &encode_jpeg(&small, 70),
    );
    // Neste bilde i serien, 5 s senere: ligner, men er et annet øyeblikk.
    let next = pattern(1600, 1200, 43);
    write(
        &icloud,
        "IMG_0101.JPG",
        &insert_exif(&encode_jpeg(&next, 92), &exif("2011:03:02 08:15:05")),
    );
    // Stående bilde (orientering 6).
    let portrait = tiff(&ExifSpec {
        taken: Some("2011:03:03 10:00:00"),
        orientation: Some(6),
        ..Default::default()
    });
    write(
        &icloud,
        "IMG_0102.JPG",
        &insert_exif(&encode_jpeg(&pattern(800, 600, 44), 90), &portrait),
    );
    // Skadet fil.
    write(&dropbox, "IMG_0999.JPG", b"ikke et bilde");

    let keys = MemoryKeyStore::default();
    let (mut store, _) = Store::create(data.path(), &keys).unwrap();
    store
        .add_source(SourceKind::Icloud, &icloud, "iCloud")
        .unwrap();
    store
        .add_source(SourceKind::Dropbox, &dropbox, "Dropbox")
        .unwrap();
    let report = ingest_all(
        &mut store,
        &DedupConfig::default(),
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();

    assert_eq!(report.unreadable, 1);
    assert_eq!(report.summary.unreadable, 1);
    assert_eq!(
        report.summary.near_duplicates, 1,
        "WhatsApp-kopien skal kjennes igjen"
    );
    assert_eq!(
        report.summary.photos, 3,
        "original, seriebilde og stående bilde"
    );

    let photos = store.photos_in_year(2011).unwrap();
    let original = &photos[0];
    assert_eq!(
        original.width,
        Some(1600),
        "originalen i høyest oppløsning beholdes"
    );
    assert_eq!(
        original.sources,
        vec![SourceKind::Dropbox, SourceKind::Icloud],
        "kildene til kopien regnes med"
    );
    assert_eq!(
        photos[1].width,
        Some(1600),
        "seriebildet er ikke en dublett"
    );
    let p = &photos[2];
    assert_eq!(
        (p.width, p.height),
        (Some(600), Some(800)),
        "mål etter rotering"
    );

    // Miniatyrene finnes, er krypterte på disken og dekrypteres til gyldig JPEG.
    for photo in &photos {
        assert!(photo.has_thumbnail);
        let bytes = store.get_thumbnail(&photo.hash).unwrap().unwrap();
        let img = image::load_from_memory(&bytes).unwrap();
        assert!(img.width() <= 400 && img.height() <= 400);
    }
    let thumb = image::load_from_memory(&store.get_thumbnail(&p.hash).unwrap().unwrap()).unwrap();
    assert!(
        thumb.height() > thumb.width(),
        "miniatyren av det stående bildet er stående"
    );

    // Fjernes originalen, blir WhatsApp-kopien det beste bildet igjen.
    std::fs::remove_file(icloud.join("IMG_0100.JPG")).unwrap();
    let after = ingest_all(
        &mut store,
        &DedupConfig::default(),
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(after.summary.near_duplicates, 0);
    assert_eq!(after.summary.photos, 3);
}

#[test]
fn heic_is_decoded_where_the_platform_can() {
    use p2a_ingest::synth::pattern;

    let lib = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let img = pattern(1200, 900, 77);
    // Ekte HEIC der plattformen kan kode det (Mac); ellers en ugyldig HEIC-fil.
    let heic = p2a_heic::encode_for_tests(img.width(), img.height(), img.as_raw());
    let can_decode = heic.is_some();
    if cfg!(target_os = "macos") {
        assert!(
            can_decode,
            "ImageIO skal kunne kode HEIC på Mac, ellers er ikke dekodingen testet"
        );
    }
    write(
        lib.path(),
        "IMG_20110302_081500.HEIC",
        &heic.unwrap_or_else(|| b"....ftypheic".to_vec()),
    );

    let keys = MemoryKeyStore::default();
    let (mut store, _) = Store::create(data.path(), &keys).unwrap();
    store
        .add_source(SourceKind::Icloud, lib.path(), "iCloud")
        .unwrap();
    let report = ingest_all(
        &mut store,
        &DedupConfig::default(),
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();

    if can_decode {
        assert_eq!(report.summary.photos, 1);
        assert_eq!(report.summary.without_preview, 0);
        let p = &store.photos_in_year(2011).unwrap()[0];
        assert!(p.has_thumbnail);
        assert_eq!((p.width, p.height), (Some(1200), Some(900)));
        let thumb =
            image::load_from_memory(&store.get_thumbnail(&p.hash).unwrap().unwrap()).unwrap();
        assert_eq!(thumb.width(), 400);
    } else {
        // Uten dekoder: bildet er med (dato fra filnavnet), men uten miniatyr.
        assert_eq!(report.summary.photos + report.summary.unreadable, 1);
    }
}

/// Ansikter med ekte bilder. Kjør med
/// `P2A_ANSIKTSMAPPE=/sti cargo test -p p2a-ingest -- --ignored --nocapture`.
/// Mappen bør ha minst to bilder av samme person; de skal havne i samme gruppe.
#[test]
#[ignore]
fn ansikter_grupperes_i_personer() {
    let dir = std::env::var("P2A_ANSIKTSMAPPE").expect("P2A_ANSIKTSMAPPE");
    let data = tempfile::tempdir().unwrap();
    let keys = MemoryKeyStore::default();
    let (mut store, _) = Store::create(data.path(), &keys).unwrap();
    store
        .add_source(SourceKind::Pc, Path::new(&dir), "Ansikter")
        .unwrap();
    let mut phases = Vec::new();
    let report = ingest_all(
        &mut store,
        &DedupConfig::default(),
        &AtomicBool::new(false),
        &mut |p: Progress| phases.push(p.phase),
    )
    .unwrap();
    assert!(phases.contains(&Phase::Personer));
    assert!(report.faces > 0);
    let groups = store.face_groups(2, 3).unwrap();
    for g in &groups {
        println!(
            "gruppe {}: {} ansikter i {} bilder",
            g.group, g.faces, g.photos
        );
    }
    assert!(
        groups.iter().any(|g| g.photos >= 2),
        "samme person i flere bilder"
    );
    // Andre gang er alt gjort: ingen ansikter mangler.
    assert!(store
        .photos_missing_faces(p2a_ingest::pipeline::face_model())
        .unwrap()
        .is_empty());
}
