//! `p2a`: verktøy for utviklere.
//!
//! ```text
//! p2a bench [--antall 1000] [--bredde 2048] [--mappe STI]
//! ```
//!
//! `bench` lager syntetiske bilder (ikke medregnet i tiden), leser dem inn i en kryptert
//! katalog og skriver tid per fase. Mål: 10 000 bilder analysert på under 20 minutter på
//! en vanlig bærbar maskin (ARCHITECTURE.md, «Ytelse»).

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use p2a_core::config::DedupConfig;
use p2a_core::SourceKind;
use p2a_ingest::pipeline::{ingest_all, Phase};
use p2a_ingest::synth::{encode_jpeg, insert_exif, pattern, tiff, ExifSpec};
use p2a_store::{MemoryKeyStore, Store};
use rayon::prelude::*;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("bench") => bench(&args[1..]),
        _ => Err("Bruk: p2a bench [--antall N] [--bredde PX] [--mappe STI]".to_string()),
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(2);
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn bench(args: &[String]) -> Result<(), String> {
    let count: u32 = flag(args, "--antall")
        .map_or(Ok(1000), |v| v.parse())
        .map_err(|_| "ugyldig --antall")?;
    let width: u32 = flag(args, "--bredde")
        .map_or(Ok(2048), |v| v.parse())
        .map_err(|_| "ugyldig --bredde")?;
    let height = width * 3 / 4;
    let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let lib = flag(args, "--mappe")
        .map(PathBuf::from)
        .unwrap_or_else(|| tmp.path().join("bilder"));

    eprintln!(
        "Lager {count} syntetiske bilder à {width}×{height} i {} …",
        lib.display()
    );
    let t = Instant::now();
    let bytes = generate(&lib, count, width, height)?;
    eprintln!(
        "  ferdig på {:.1} s ({:.0} MB, snitt {:.0} kB per bilde; ikke medregnet)",
        t.elapsed().as_secs_f64(),
        bytes as f64 / 1e6,
        bytes as f64 / count as f64 / 1e3
    );

    let data = tmp.path().join("data");
    let keys = MemoryKeyStore::default();
    let (mut store, _) = Store::create(&data, &keys).map_err(|e| e.to_string())?;
    store
        .add_source(SourceKind::Pc, &lib, "Bilder")
        .map_err(|e| e.to_string())?;

    let start = Instant::now();
    let mut marks: Vec<(Phase, f64)> = Vec::new();
    let report = ingest_all(
        &mut store,
        &DedupConfig::default(),
        &AtomicBool::new(false),
        &mut |p| {
            if marks.last().map(|m| m.0) != Some(p.phase) {
                marks.push((p.phase, start.elapsed().as_secs_f64()));
            }
        },
    )
    .map_err(|e| e.to_string())?;
    let total = start.elapsed().as_secs_f64();

    println!(
        "Innlesing av {count} bilder ({width}×{height}), {} tråder:",
        rayon::current_num_threads()
    );
    for (i, (phase, at)) in marks.iter().enumerate() {
        let end = marks.get(i + 1).map_or(total, |m| m.1);
        println!("  {:<10} {:>8.2} s", format!("{phase:?}"), end - at);
    }
    println!(
        "  {:<10} {:>8.2} s  ({:.1} ms per bilde)",
        "Totalt",
        total,
        total * 1000.0 / count as f64
    );
    println!(
        "  Anslag for 10 000 bilder: {:.1} min (mål for hele analysen: under 20 min)",
        total / count as f64 * 10_000.0 / 60.0
    );
    println!(
        "  Bilder: {}, dubletter: {} eksakte, {} transkodede",
        report.summary.photos, report.summary.exact_duplicates, report.summary.near_duplicates
    );
    Ok(())
}

/// Bilder med EXIF-tid spredt over ett år; hvert tiende er en eksakt kopi i en undermappe.
fn generate(dir: &Path, count: u32, width: u32, height: u32) -> Result<u64, String> {
    std::fs::create_dir_all(dir.join("kopier")).map_err(|e| e.to_string())?;
    let sizes: Vec<u64> = (0..count)
        .into_par_iter()
        .map(|i| {
            let day = i * 365 / count.max(1);
            let (month, dom) = (1 + day / 31 % 12, 1 + day % 28);
            let taken = format!(
                "2011:{month:02}:{dom:02} {:02}:{:02}:{:02}",
                8 + i % 12,
                i % 60,
                (i * 7) % 60
            );
            let spec = ExifSpec {
                taken: Some(&taken),
                make: Some("Apple"),
                model: Some("iPhone 4"),
                ..Default::default()
            };
            let jpeg = insert_exif(&encode_jpeg(&pattern(width, height, i), 88), &tiff(&spec));
            let name = format!("IMG_{i:05}.JPG");
            std::fs::write(dir.join(&name), &jpeg).expect("skrive testbilde");
            if i % 10 == 0 {
                std::fs::write(dir.join("kopier").join(&name), &jpeg).expect("skrive kopi");
            }
            jpeg.len() as u64
        })
        .collect();
    Ok(sizes.iter().sum())
}
