//! Trykkfilen fra ende til ende med syntetiske bilder.

use std::path::Path;

use p2a_core::layout::PageKind;
use p2a_print::layout::Look;
use p2a_print::{render, Album, AlbumPage, AlbumText, PhotoSource};

fn photo(dir: &Path, name: &str, w: u32, h: u32, seed: u8) -> PhotoSource {
    let img = image::RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([
            (x * 255 / w) as u8,
            (y * 255 / h) as u8,
            seed.wrapping_mul(40),
        ])
    });
    let path = dir.join(name);
    img.save(&path).unwrap();
    PhotoSource {
        path,
        format: Some("jpeg".into()),
        orientation: None,
        width: w,
        height: h,
        rotation: 0,
        look: Look::default(),
    }
}

fn album(dir: &Path) -> Album {
    let l = |n: &str, s| photo(dir, n, 1200, 800, s);
    let p = |n: &str, s| photo(dir, n, 800, 1200, s);
    Album {
        year: 2011,
        text: AlbumText {
            title: "Øyeblikk fra 2011".into(),
            subtitle: "Kari, Ola, Emma og Jonas".into(),
            intro: "2011 var året da Emma begynte på skolen.\n\nVi var på hytta hele sommeren."
                .into(),
            back: "Takk for et fint år!".into(),
        },
        cover: Some(l("forside.jpg", 1)),
        back: Some(p("bakside.jpg", 2)),
        pages: vec![
            AlbumPage {
                kind: PageKind::Helside,
                mal: None,
                photos: vec![p("a.jpg", 3)],
            },
            AlbumPage {
                kind: PageKind::Luft,
                mal: None,
                photos: vec![l("b.jpg", 4)],
            },
            // Faste rammer med utsnitt, og et bilde gjort større.
            AlbumPage {
                kind: PageKind::Rutenett { kolonner: 2 },
                mal: Some("2-side".into()),
                photos: vec![l("g.jpg", 9), {
                    let mut x = l("h.jpg", 10);
                    x.look.focus = (0.0, 0.5);
                    x
                }],
            },
            AlbumPage {
                kind: PageKind::Rutenett { kolonner: 3 },
                mal: None,
                photos: vec![
                    l("c.jpg", 5),
                    p("d.jpg", 6),
                    {
                        let mut x = l("e.jpg", 7);
                        x.look.size = 2;
                        x
                    },
                    {
                        let mut gone = l("f.jpg", 8);
                        gone.path = dir.join("finnes-ikke.jpg");
                        gone
                    },
                ],
            },
        ],
    }
}

#[test]
fn trykkfil_med_forside_tekst_historier_og_bakside() {
    let dir = tempfile::tempdir().unwrap();
    let a = album(dir.path());
    let mut calls = Vec::new();
    let (pdf, report) = render(&a, &mut |done, total| calls.push((done, total))).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    // Forside, tekstside, fire historiesider, tom side og bakside: åtte sider, partall.
    assert_eq!(report.pages, 8);
    let text = String::from_utf8_lossy(&pdf);
    assert_eq!(text.matches("/TrimBox").count(), report.pages);
    assert_eq!(text.matches("/BleedBox").count(), report.pages);
    assert_eq!(report.photos, 10);
    assert_eq!(report.missing.len(), 1);
    assert!(report.missing[0].ends_with("finnes-ikke.jpg"));
    assert!(
        !report.low_resolution.is_empty(),
        "1200 punkter på en helside er for lite"
    );
    assert_eq!(calls.last(), Some(&(10, 10)));
}

#[test]
fn tom_side_gir_partall() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = album(dir.path());
    a.pages.pop();
    let (_, report) = render(&a, &mut |_, _| {}).unwrap();
    // Forside, tekstside, tre historiesider og bakside.
    assert_eq!(report.pages, 6);
}

#[test]
fn uten_sider_er_det_ingen_trykkfil() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = album(dir.path());
    a.pages.clear();
    assert!(render(&a, &mut |_, _| {}).is_err());
}

/// Skriver en trykkfil til `P2A_TRYKK_UT` for å se på den (pdftoppm e.l.).
#[test]
#[ignore]
fn skriv_eksempel() {
    let dir = tempfile::tempdir().unwrap();
    let (pdf, report) = render(&album(dir.path()), &mut |_, _| {}).unwrap();
    std::fs::write(std::env::var("P2A_TRYKK_UT").unwrap(), pdf).unwrap();
    println!("{report:?}");
}
