//! Livsløpet til den krypterte lagringen.

use p2a_core::{CommentKind, ContentHash, Role};
use p2a_store::{
    KeyStore, MemoryKeyStore, NewComment, RecoveryKey, Store, StoreError, VaultStatus,
};

const SECRET_NAME: &str = "Oldemor Astrid Solbakken";

fn setup() -> (tempfile::TempDir, MemoryKeyStore) {
    (tempfile::tempdir().unwrap(), MemoryKeyStore::default())
}

#[test]
fn first_start_creates_store_and_recovery_key() {
    let (dir, keys) = setup();
    assert_eq!(
        Store::status(dir.path(), &keys).unwrap(),
        VaultStatus::Empty
    );

    let (store, recovery) = Store::create(dir.path(), &keys).unwrap();
    assert_eq!(store.schema_version().unwrap(), 3);
    assert_eq!(recovery.display_code().len(), 34);
    assert_eq!(
        Store::status(dir.path(), &keys).unwrap(),
        VaultStatus::Ready
    );

    assert!(matches!(
        Store::create(dir.path(), &keys),
        Err(StoreError::AlreadyExists(_))
    ));
}

#[test]
fn data_survives_reopen() {
    let (dir, keys) = setup();
    {
        let (store, _) = Store::create(dir.path(), &keys).unwrap();
        store
            .add_person(SECRET_NAME, Role::Besteforeldre, true, None)
            .unwrap();
        store
            .add_comment(&NewComment {
                album_year: 2011,
                target_kind: "album".into(),
                target_id: None,
                kind: CommentKind::MerAv,
                text: Some("Flere bilder fra hytta".into()),
            })
            .unwrap();
    }
    let store = Store::open(dir.path(), &keys).unwrap();
    let persons = store.persons().unwrap();
    assert_eq!(persons.len(), 1);
    assert_eq!(persons[0].name, SECRET_NAME);
    assert_eq!(persons[0].role, Role::Besteforeldre);
    assert!(persons[0].important);
    let comments = store.comments(2011).unwrap();
    assert_eq!(comments[0].comment.kind, CommentKind::MerAv);
    assert!(store.comments(2012).unwrap().is_empty());
}

#[test]
fn files_on_disk_are_encrypted() {
    let (dir, keys) = setup();
    let hash = ContentHash([7; 32]);
    {
        let (store, _) = Store::create(dir.path(), &keys).unwrap();
        store
            .add_person(SECRET_NAME, Role::Besteforeldre, false, None)
            .unwrap();
        store
            .put_thumbnail(&hash, b"JPEG-MINIATYR av barna")
            .unwrap();
        // Tving innholdet fra WAL-filen inn i databasefilen før vi leser den.
        store
            .connection()
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .unwrap();
    }
    let mut all = Vec::new();
    for entry in walk(dir.path()) {
        all.extend(std::fs::read(entry).unwrap());
    }
    assert!(!all.is_empty());
    for needle in [
        SECRET_NAME.as_bytes(),
        b"SQLite format 3",
        b"JPEG-MINIATYR",
        b"persons",
    ] {
        assert!(
            !all.windows(needle.len()).any(|w| w == needle),
            "fant ukryptert «{}» på disken",
            String::from_utf8_lossy(needle)
        );
    }
}

#[test]
fn new_machine_needs_recovery_key() {
    let (dir, keys) = setup();
    let code = {
        let (store, recovery) = Store::create(dir.path(), &keys).unwrap();
        store
            .add_person(SECRET_NAME, Role::Barn, false, Some("2011-03-02"))
            .unwrap();
        recovery.display_code().to_string()
    };

    // Ny maskin: tom nøkkelring, men dataene er kopiert over.
    let new_keys = MemoryKeyStore::default();
    assert_eq!(
        Store::status(dir.path(), &new_keys).unwrap(),
        VaultStatus::NeedsRecovery
    );
    assert!(matches!(
        Store::open(dir.path(), &new_keys),
        Err(StoreError::NeedsRecovery)
    ));

    let wrong = RecoveryKey::generate().unwrap();
    assert!(matches!(
        Store::recover(dir.path(), &new_keys, &wrong),
        Err(StoreError::WrongKey)
    ));

    let recovery = RecoveryKey::parse(&code).unwrap();
    let store = Store::recover(dir.path(), &new_keys, &recovery).unwrap();
    assert_eq!(
        store.persons().unwrap()[0].birth_date.as_deref(),
        Some("2011-03-02")
    );
    drop(store);

    // Nøkkelen er lagt tilbake i nøkkelringen, så vanlig oppstart virker igjen.
    assert!(new_keys.load().unwrap().is_some());
    Store::open(dir.path(), &new_keys).unwrap();
}

#[test]
fn wrong_master_key_is_rejected() {
    let (dir, keys) = setup();
    drop(Store::create(dir.path(), &keys).unwrap());
    keys.save(&[9u8; 32]).unwrap();
    assert!(matches!(
        Store::open(dir.path(), &keys),
        Err(StoreError::WrongKey)
    ));
}

#[test]
fn thumbnails_round_trip_and_are_bound_to_hash() {
    let (dir, keys) = setup();
    let (store, _) = Store::create(dir.path(), &keys).unwrap();
    let a = ContentHash([1; 32]);
    let b = ContentHash([2; 32]);
    assert_eq!(store.get_thumbnail(&a).unwrap(), None);
    store.put_thumbnail(&a, b"miniatyr a").unwrap();
    assert_eq!(
        store.get_thumbnail(&a).unwrap().as_deref(),
        Some(&b"miniatyr a"[..])
    );

    // Flytter vi a sin fil til b sitt navn, skal dekrypteringen feile (hashen er AAD).
    let path = |h: &ContentHash| {
        let hex = h.to_hex();
        dir.path().join("miniatyrer").join(&hex[..2]).join(hex)
    };
    std::fs::create_dir_all(path(&b).parent().unwrap()).unwrap();
    std::fs::copy(path(&a), path(&b)).unwrap();
    assert!(matches!(store.get_thumbnail(&b), Err(StoreError::Decrypt)));
}

#[test]
fn delete_all_removes_everything_but_source_photos() {
    let (dir, keys) = setup();
    let photo = dir.path().join("Bilder").join("IMG_0001.jpg");
    std::fs::create_dir_all(photo.parent().unwrap()).unwrap();
    std::fs::write(&photo, b"originalbilde").unwrap();
    {
        let (store, _) = Store::create(dir.path(), &keys).unwrap();
        store.put_thumbnail(&ContentHash([3; 32]), b"x").unwrap();
        store.set_setting("valgt_aar", "2011").unwrap();
        assert_eq!(store.setting("valgt_aar").unwrap().as_deref(), Some("2011"));
    }
    Store::delete_all(dir.path(), &keys).unwrap();
    assert_eq!(
        Store::status(dir.path(), &keys).unwrap(),
        VaultStatus::Empty
    );
    assert!(keys.load().unwrap().is_none());
    assert!(!dir.path().join("katalog.db").exists());
    assert!(!dir.path().join("miniatyrer").exists());
    assert!(photo.exists(), "bildene i kildemappene skal aldri røres");
    // Kan kjøres igjen uten feil.
    Store::delete_all(dir.path(), &keys).unwrap();
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}

#[test]
fn decisions_and_feedback_are_kept() {
    use p2a_core::learn::{Action, FeedbackReason};
    use p2a_core::select::draft::Decision;

    let (dir, keys) = setup();
    let a = ContentHash([1; 32]);
    let b = ContentHash([2; 32]);
    {
        let (store, _) = Store::create(dir.path(), &keys).unwrap();
        store.set_decision(2025, &a, Some(Decision::Med)).unwrap();
        store
            .set_decision(2025, &a, Some(Decision::IkkeMed))
            .unwrap();
        store.set_decision(2025, &b, Some(Decision::Med)).unwrap();
        store.set_decision(2025, &b, None).unwrap();
        let id = store
            .add_feedback(2025, Action::Bytt, &b, Some(&a))
            .unwrap();
        store
            .set_feedback_reason(id, Some(FeedbackReason::ViktigOyeblikk))
            .unwrap();
        store.add_feedback(2025, Action::TaBort, &a, None).unwrap();
    }
    let store = Store::open(dir.path(), &keys).unwrap();
    let d = store.decisions(2025).unwrap();
    assert_eq!(d.len(), 1);
    assert_eq!(d[&a], Decision::IkkeMed);
    assert!(store.decisions(2024).unwrap().is_empty());
    let log = store.feedback_log().unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].action, Action::Bytt);
    assert_eq!(log[0].reason, Some(FeedbackReason::ViktigOyeblikk));
    assert_eq!(log[1].reason, None);
}

#[test]
fn godparents_are_remembered_and_confirmation_sticks() {
    use p2a_core::{RelationKind, RelationStatus};
    use p2a_store::Relation;

    let (dir, keys) = setup();
    {
        let (store, _) = Store::create(dir.path(), &keys).unwrap();
        let barn = store.add_person("Ella", Role::Barn, false, None).unwrap();
        let fadder = store.add_person("Kari", Role::Venn, false, None).unwrap();
        let mut r = Relation {
            person_id: fadder,
            kind: RelationKind::FadderFor,
            other_id: barn,
            status: RelationStatus::Bekreftet,
            learned_year: Some(2008),
        };
        store.set_relation(&r).unwrap();
        r.status = RelationStatus::Foreslatt;
        store.set_relation(&r).unwrap();
    }
    let store = Store::open(dir.path(), &keys).unwrap();
    let rel = store.relations().unwrap();
    assert_eq!(rel.len(), 1);
    assert_eq!(rel[0].status, RelationStatus::Bekreftet);
    assert_eq!(rel[0].learned_year, Some(2008));
}
