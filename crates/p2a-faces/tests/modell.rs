//! Modellene lastes og kjøres. Ekte ansikter testes ikke her (vi legger ikke bilder av
//! personer i repoet); se `examples/sammenlign.rs` og `docs/ARCHITECTURE.md` for kontrollen
//! mot OpenCV, og testen under for en egen mappe med bilder.

use image::{Rgb, RgbImage};
use p2a_faces::{Embedding, FaceEngine, EMBEDDING_LEN};

#[test]
fn ingen_ansikter_i_flate_og_monstre() {
    let engine = FaceEngine::new().expect("modellene lastes");
    let flat = RgbImage::from_pixel(800, 600, Rgb([120, 140, 160]));
    assert!(engine.faces(&flat).unwrap().is_empty());
    let stripes = RgbImage::from_fn(640, 480, |x, y| {
        let v = if (x / 16 + y / 16) % 2 == 0 { 30 } else { 220 };
        Rgb([v, v, v])
    });
    assert!(engine.faces(&stripes).unwrap().is_empty());
}

#[test]
fn kjennetegn_er_normalisert_og_stabilt() {
    let engine = FaceEngine::new().unwrap();
    let face = RgbImage::from_fn(112, 112, |x, y| Rgb([(x * 2) as u8, (y * 2) as u8, 90]));
    let a = engine.embed_aligned(&face).unwrap();
    let b = engine.embed_aligned(&face).unwrap();
    assert_eq!(a.0.len(), EMBEDDING_LEN);
    assert!((a.similarity(&a) - 1.0).abs() < 1e-4);
    assert_eq!(a, b);
    assert_eq!(Embedding::from_bytes(&a.to_bytes()), Some(a));
}

/// Kjør med `P2A_ANSIKTSMAPPE=/sti/til/bilder cargo test -p p2a-faces -- --ignored`: alle
/// bildene i mappen skal ha minst ett ansikt.
#[test]
#[ignore]
fn ansikter_i_egen_mappe() {
    let dir = std::env::var("P2A_ANSIKTSMAPPE").expect("P2A_ANSIKTSMAPPE");
    let engine = FaceEngine::new().unwrap();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let Ok(img) = image::open(&path) else {
            continue;
        };
        let n = engine.faces(&img.to_rgb8()).unwrap().len();
        println!("{}: {n}", path.display());
        assert!(n > 0, "fant ingen ansikter i {}", path.display());
    }
}
