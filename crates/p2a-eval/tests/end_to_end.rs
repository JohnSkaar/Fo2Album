//! Hele M2-flyten på syntetiske data: bibliotek → album-PDF → gullsett → evaluering.

use std::path::Path;
use std::sync::atomic::AtomicBool;

use lopdf::{dictionary, Document, Object, Stream};
use p2a_core::config::DedupConfig;
use p2a_core::SourceKind;
use p2a_eval::run::{create_gold_set, evaluate_all, review_html};
use p2a_ingest::pipeline::ingest_all;
use p2a_ingest::synth::{encode_jpeg, insert_exif, pattern, tiff, ExifSpec};
use p2a_store::{MemoryKeyStore, Store};

/// Lager en PDF der hver side har en felles bakgrunn (pynt) og de gitte JPEG-ene.
fn write_album(path: &Path, pages: &[Vec<(Vec<u8>, u32, u32)>], background: &(Vec<u8>, u32, u32)) {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let image = |doc: &mut Document, (bytes, w, h): &(Vec<u8>, u32, u32)| {
        let mut s = Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Image", "Width" => *w as i64, "Height" => *h as i64,
                "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8, "Filter" => "DCTDecode",
            },
            bytes.clone(),
        );
        s.allows_compression = false;
        doc.add_object(s)
    };
    let bg = image(&mut doc, background);
    let mut kids = Vec::new();
    for imgs in pages {
        let mut xobjects = lopdf::Dictionary::new();
        xobjects.set("Bg", bg);
        let mut ops = String::from("q 595 0 0 842 0 0 cm /Bg Do Q\n");
        for (i, img) in imgs.iter().enumerate() {
            let id = image(&mut doc, img);
            xobjects.set(format!("Im{i}"), id);
            ops.push_str(&format!(
                "q 250 0 0 200 40 {} cm /Im{i} Do Q\n",
                600 - 250 * i
            ));
        }
        let content = doc.add_object(Stream::new(dictionary! {}, ops.into_bytes()));
        let page = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "Contents" => content,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Resources" => dictionary! { "XObject" => xobjects },
        });
        kids.push(Object::Reference(page));
    }
    let count = kids.len() as i64;
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    doc.save(path).unwrap();
}

#[test]
fn gold_set_from_album_pdf_and_baseline_evaluation() {
    let lib = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();

    // Seks hendelser i 2010 (måned 2, 4, …, 12), åtte bilder hver med en time imellom.
    let seed = |e: u32, i: u32| e * 100 + i;
    for e in 0..6u32 {
        for i in 0..8u32 {
            let taken = format!("2010:{:02}:10 {:02}:00:00", 2 + e * 2, 8 + i);
            let spec = ExifSpec {
                taken: Some(&taken),
                make: Some("Canon"),
                ..Default::default()
            };
            let jpeg = insert_exif(
                &encode_jpeg(&pattern(1600, 1200, seed(e, i)), 90),
                &tiff(&spec),
            );
            std::fs::write(lib.path().join(format!("IMG_{e}{i}.JPG")), jpeg).unwrap();
        }
    }
    let keys = MemoryKeyStore::default();
    let (mut store, _) = Store::create(data.path(), &keys).unwrap();
    store
        .add_source(SourceKind::Pc, lib.path(), "Bilder")
        .unwrap();
    ingest_all(
        &mut store,
        &DedupConfig::default(),
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();

    // Familiens album: to bilder fra hver av de fem første hendelsene, beskåret kvadratisk
    // eller nedskalert og komprimert på nytt, pluss ett bilde som ikke finnes i biblioteket.
    let album_img = |s: u32, square: bool| {
        let img = image::DynamicImage::ImageRgb8(pattern(1600, 1200, s));
        let img = if square {
            img.crop_imm(200, 0, 1200, 1200)
        } else {
            img.thumbnail(1000, 1000)
        };
        let rgb = img.to_rgb8();
        (encode_jpeg(&rgb, 80), rgb.width(), rgb.height())
    };
    let mut pages: Vec<Vec<(Vec<u8>, u32, u32)>> = (0..5u32)
        .map(|e| vec![album_img(seed(e, 1), true), album_img(seed(e, 5), false)])
        .collect();
    pages.push(vec![album_img(9999, false)]);
    let background = {
        let rgb = image::RgbImage::from_pixel(600, 800, image::Rgb([250, 246, 240]));
        (encode_jpeg(&rgb, 90), 600, 800)
    };
    let pdf = lib.path().join("Øyeblikk fra 2010.pdf");
    write_album(&pdf, &pages, &background);

    let report = create_gold_set(&store, &pdf, 2010, "test-2010").unwrap();
    assert_eq!(report.stats.pages, 6);
    assert_eq!(
        report.stats.decorations, 1,
        "bakgrunnen på alle sider er pynt"
    );
    let g = &report.gold;
    assert_eq!(g.bilder_i_album, 11);
    assert_eq!(
        g.valgt.len(),
        10,
        "usikre: {:?}, uten treff: {:?}",
        g.usikre,
        g.uten_treff
    );
    assert_eq!(g.uten_treff.len() + g.usikre.len(), 1);
    assert_eq!(g.kilde, "Øyeblikk fra 2010.pdf");

    let html = review_html(&store, &report).unwrap();
    assert!(html.contains("Gullsett test-2010") && html.contains("data:image/jpeg;base64,"));

    // Evaluering: familien har 5 av 6 hendelser; utgangspunktet velger like mange bilder.
    let results = evaluate_all(&store).unwrap();
    assert_eq!(results.len(), 1);
    let m = &results[0].metrics;
    assert_eq!(
        (m.bilder_i_biblioteket, m.bilder_i_fasit, m.bilder_valgt),
        (48, 10, 10)
    );
    assert_eq!(m.hendelser, 6);
    assert!((m.hendelsesdekning_fasit - 5.0 / 6.0).abs() < 1e-9);
    for v in [
        m.presisjon,
        m.gjenfinning,
        m.hendelsesdekning_utvalg,
        m.redundans,
    ] {
        assert!((0.0..=1.0).contains(&v));
    }

    // Gullsettet ligger i den krypterte databasen og overlever ny åpning.
    drop(store);
    let store = Store::open(data.path(), &keys).unwrap();
    assert_eq!(evaluate_all(&store).unwrap()[0].gold.valgt.len(), 10);
}
