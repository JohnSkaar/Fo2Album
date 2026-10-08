//! Brukerens valg i utkastet og loggen appen lærer av.

use p2a_core::learn::Action;
use p2a_core::select::draft::Decision;
use p2a_core::ContentHash;
use p2a_store::{MemoryKeyStore, Store};

#[test]
fn angret_handling_fjernes_fra_loggen() {
    let dir = tempfile::tempdir().unwrap();
    let keys = MemoryKeyStore::default();
    let (store, _) = Store::create(dir.path(), &keys).unwrap();
    let h = ContentHash([7; 32]);
    store.set_decision(2011, &h, Some(Decision::Med)).unwrap();
    let a = store.add_feedback(2011, Action::TaMed, &h, None).unwrap();
    let b = store.add_feedback(2011, Action::TaBort, &h, None).unwrap();
    assert_eq!(store.feedback_log().unwrap().len(), 2);
    store.delete_feedback(b).unwrap();
    assert_eq!(store.feedback_log().unwrap().len(), 1);
    store.delete_feedback(a).unwrap();
    assert!(store.feedback_log().unwrap().is_empty());
    store.set_decision(2011, &h, None).unwrap();
    assert!(store.decisions(2011).unwrap().is_empty());
}
