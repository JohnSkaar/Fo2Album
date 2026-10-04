//! Ansikter: lagring, gruppering og brukerens navngiving.

use p2a_core::{ContentHash, FileStatus, PhotoMeta, Role, SourceKind, TakenAt};
use p2a_store::{
    FileEntry, MemoryKeyStore, NewFace, ReadOutcome, Store, GROUP_IGNORED, GROUP_UNNAMED,
};

fn face(x: f32, h: f32, emb: u8) -> NewFace {
    NewFace {
        x,
        y: 0.2,
        w: h * 0.8,
        h,
        landmarks: [0.0; 10],
        score: 0.95,
        sharpness: 0.7,
        embedding: vec![emb; 512],
    }
}

/// En lagring med tre bilder fra 2011 og ett fra 2012.
fn store_with_photos() -> (tempfile::TempDir, Store, Vec<ContentHash>) {
    let dir = tempfile::tempdir().unwrap();
    let keys = MemoryKeyStore::default();
    let (mut store, _) = Store::create(dir.path(), &keys).unwrap();
    let src = store
        .add_source(SourceKind::Pc, dir.path(), "Bilder")
        .unwrap();
    let entries: Vec<FileEntry> = (0..4)
        .map(|i| FileEntry {
            rel_path: format!("{i}.jpg"),
            size: 100,
            modified: 0,
            status: FileStatus::Local,
        })
        .collect();
    let (_, files) = store.sync_files(src, &entries).unwrap();
    let hashes: Vec<ContentHash> = (0..4).map(|i| ContentHash([i as u8 + 1; 32])).collect();
    let reads: Vec<(i64, ReadOutcome)> = files
        .iter()
        .zip(&hashes)
        .enumerate()
        .map(|(i, (f, h))| {
            let mut m = PhotoMeta::new(*h);
            m.taken_at = TakenAt::new(if i < 3 { 2011 } else { 2012 }, 5, 1 + i as u8, 12, 0, 0);
            (f.file_id, ReadOutcome::New(Box::new(m)))
        })
        .collect();
    store.record_reads(&reads).unwrap();
    for h in &hashes {
        store.set_has_thumbnail(h).unwrap();
    }
    (dir, store, hashes)
}

#[test]
fn ansikter_lagres_og_bildet_er_ferdig() {
    let (_dir, mut store, hashes) = store_with_photos();
    assert_eq!(store.photos_missing_faces().unwrap().len(), 4);
    store
        .put_faces(&[
            (hashes[0], vec![face(0.1, 0.3, 1), face(0.6, 0.02, 2)]),
            (hashes[1], vec![]),
        ])
        .unwrap();
    assert_eq!(store.photos_missing_faces().unwrap().len(), 2);
    let year = store.faces_in_year(2011).unwrap();
    assert_eq!(year.len(), 2, "bare bilder som er gått gjennom");
    assert_eq!(year[&hashes[0]].len(), 2);
    assert!(year[&hashes[0]][0].counts());
    assert!(!year[&hashes[0]][1].counts(), "for lite til å telle");
    assert!(year[&hashes[1]].is_empty());
    // Gjentatt analyse erstatter ansiktene.
    store
        .put_faces(&[(hashes[0], vec![face(0.1, 0.3, 1)])])
        .unwrap();
    assert_eq!(store.faces_in_year(2011).unwrap()[&hashes[0]].len(), 1);
}

#[test]
fn navngiving_overlever_ny_gruppering() {
    let (_dir, mut store, hashes) = store_with_photos();
    store
        .put_faces(&[
            (hashes[0], vec![face(0.1, 0.3, 1), face(0.5, 0.3, 2)]),
            (hashes[1], vec![face(0.1, 0.3, 1)]),
            (hashes[2], vec![face(0.1, 0.3, 3)]),
        ])
        .unwrap();
    let faces = store.stored_faces().unwrap();
    assert_eq!(faces.len(), 4);
    assert!(faces.iter().all(|f| f.fixed.is_none()));
    // Grupper: ansikt 1 og 3 er samme person; 2 og 4 er hver sin.
    let g = |k| GROUP_UNNAMED + k;
    store
        .set_face_groups(&[
            (faces[0].id, g(0)),
            (faces[1].id, g(1)),
            (faces[2].id, g(0)),
            (faces[3].id, g(2)),
        ])
        .unwrap();
    let groups = store.face_groups(1, 3).unwrap();
    assert_eq!(groups.len(), 3);
    assert_eq!(
        (groups[0].group, groups[0].faces, groups[0].photos),
        (g(0), 2, 2)
    );
    assert_eq!(groups[0].person_id, None);

    let ella = store.add_person("Ella", Role::Barn, false, None).unwrap();
    store.name_face_group(g(0), ella).unwrap();
    store.ignore_face_group(g(2)).unwrap();
    let faces = store.stored_faces().unwrap();
    assert_eq!(faces[0].fixed, Some(ella));
    assert_eq!(faces[3].fixed, Some(GROUP_IGNORED));
    assert_eq!(faces[1].fixed, None);

    // Ny gruppering legger ansikt 2 hos Ella som forslag.
    store.set_face_groups(&[(faces[1].id, ella)]).unwrap();
    let year = store.faces_in_year(2011).unwrap();
    assert!(year[&hashes[0]].iter().all(|f| f.person == Some(ella)));
    assert!(year[&hashes[2]][0].ignored);
    let groups = store.face_groups(1, 3).unwrap();
    assert_eq!(groups.len(), 1, "ignorerte grupper vises ikke");
    assert_eq!(groups[0].person_id, Some(ella));
    assert_eq!(groups[0].faces, 3);

    // Flytt ett ansikt ut igjen.
    store.move_face(faces[1].id, None).unwrap();
    assert_eq!(store.face_groups(1, 3).unwrap()[0].faces, 2);
}
