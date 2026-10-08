//! `p2a`: verktøy for utviklere.
//!
//! ```text
//! p2a bench [--antall 1000] [--bredde 2048] [--mappe STI]
//! p2a demo --data STI [--antall 200] [--ansikter MAPPE]
//! p2a les-inn --mappe STI [--kilde pc|dropbox|icloud|google_disk] [--data STI]
//! p2a eval lag-gullsett --pdf ALBUM.pdf --aar 2010 --navn familie-2010 [--data STI]
//! p2a eval liste [--data STI]
//! p2a eval kjor [--data STI] [--resultater eval/RESULTS.md]
//! p2a trykk --aar 2011 --ut album.pdf [--sider N] [--data STI]
//! ```
//!
//! `trykk` lager trykkfilen (PDF) for årets album slik utkastet i appen ser ut, med tekstene
//! brukeren har lagret (eller forslaget). Filen skrives bare til `--ut`.
//!
//! `les-inn` og `eval` bruker den samme krypterte katalogen som appen (standard datamappe,
//! nøkkel fra nøkkelringen), så bildene du har lest inn i appen, kan brukes direkte.
//! Gullsett og album blir på maskinen; bare tallene skrives til `eval/RESULTS.md`.
//!
//! `demo` lager syntetiske bilder fordelt på «Dropbox» og «iCloud» (med noen dubletter og
//! et bilde uten dato) og en kryptert lagring med nøkkel i fil, slik appen bruker på Linux
//! under utvikling. Start så appen med `P2A_DATA_DIR=STI`.
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
use p2a_store::{FileKeyStore, MemoryKeyStore, Store};
use rayon::prelude::*;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("bench") => bench(&args[1..]),
        Some("demo") => demo(&args[1..]),
        Some("les-inn") => read_in(&args[1..]),
        Some("trykk") => print_album(&args[1..]),
        Some("eval") => match args.get(1).map(String::as_str) {
            Some("lag-gullsett") => eval_make_gold(&args[2..]),
            Some("liste") => eval_list(&args[2..]),
            Some("kjor") => eval_run(&args[2..]),
            _ => Err(USAGE.to_string()),
        },
        _ => Err(USAGE.to_string()),
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(2);
    }
}

const USAGE: &str = "Bruk:
  p2a bench [--antall N] [--bredde PX] [--mappe STI]
  p2a demo --data STI [--antall N] [--ansikter MAPPE]
  p2a les-inn --mappe STI [--kilde pc|dropbox|icloud|google_disk] [--data STI]
  p2a eval lag-gullsett --pdf ALBUM.pdf --aar ÅR --navn NAVN [--data STI]
  p2a eval liste [--data STI]
  p2a eval kjor [--data STI] [--resultater eval/RESULTS.md]
  p2a trykk --aar ÅR --ut ALBUM.pdf [--sider N] [--data STI]";

/// Samme datamappe som appen (Tauri `app_data_dir` for `no.fo2album.app`), eller `--data`.
fn data_dir(args: &[String]) -> Result<PathBuf, String> {
    if let Some(d) = flag(args, "--data").or_else(|| std::env::var("P2A_DATA_DIR").ok()) {
        return Ok(PathBuf::from(d));
    }
    const ID: &str = "no.fo2album.app";
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
    let dir = if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library/Application Support").join(ID))
    } else if cfg!(windows) {
        env("APPDATA").map(|a| a.join(ID))
    } else {
        env("XDG_DATA_HOME")
            .or_else(|| env("HOME").map(|h| h.join(".local/share")))
            .map(|d| d.join(ID))
    };
    dir.ok_or_else(|| "fant ikke datamappen; bruk --data STI".to_string())
}

/// Åpner appens krypterte katalog med nøkkelen fra nøkkelringen.
fn open_store(args: &[String]) -> Result<(Store, PathBuf), String> {
    let dir = data_dir(args)?;
    let keys = p2a_keychain::PlatformKeyStore::new(&dir);
    let store = Store::open(&dir, &keys).map_err(|e| match e {
        p2a_store::StoreError::NotFound(_) => format!(
            "fant ingen data i {}. Start appen én gang først (eller bruk --data).",
            dir.display()
        ),
        e => e.to_string(),
    })?;
    Ok((store, dir))
}

fn print_album(args: &[String]) -> Result<(), String> {
    use p2a_print::album;
    let year: i32 = flag(args, "--aar")
        .ok_or("mangler --aar ÅR")?
        .parse()
        .map_err(|_| "ugyldig --aar")?;
    let out = PathBuf::from(flag(args, "--ut").ok_or("mangler --ut FIL.pdf")?);
    let pages = flag(args, "--sider")
        .map(|v| v.parse::<usize>())
        .transpose()
        .map_err(|_| "ugyldig --sider")?;
    let (store, _) = open_store(args)?;
    let e = |e: p2a_store::StoreError| e.to_string();
    let (draft, metas) = album::draft_for_year(&store, year, pages).map_err(e)?;
    let (front, back) = album::default_covers(&metas);
    let text = album::load_text(&store, year).map_err(e)?;
    let a = album::from_draft(&store, year, &draft, text, front, back).map_err(e)?;
    let t = Instant::now();
    let (pdf, report) = p2a_print::render(&a, &mut |done, total| {
        eprint!("\rBilder: {done} av {total}");
    })
    .map_err(|e| e.to_string())?;
    eprintln!();
    std::fs::write(&out, &pdf).map_err(|e| format!("kunne ikke skrive {}: {e}", out.display()))?;
    println!(
        "{}: {} sider, {} bilder, {:.1} MB, {:.1} s",
        out.display(),
        report.pages,
        report.photos,
        pdf.len() as f64 / 1e6,
        t.elapsed().as_secs_f64()
    );
    if !report.missing.is_empty() {
        println!("Mangler (grå felt): {}", report.missing.len());
    }
    if !report.low_resolution.is_empty() {
        println!(
            "Lav oppløsning for størrelsen: {}",
            report.low_resolution.len()
        );
    }
    Ok(())
}

fn read_in(args: &[String]) -> Result<(), String> {
    let folder = PathBuf::from(flag(args, "--mappe").ok_or("mangler --mappe STI")?);
    if !folder.is_dir() {
        return Err(format!("fant ikke mappen {}", folder.display()));
    }
    let kind = match flag(args, "--kilde") {
        Some(k) => k.parse::<SourceKind>().map_err(|e| e.to_string())?,
        None => p2a_ingest::sources::detect_kind(&folder),
    };
    let (mut store, _) = open_store(args)?;
    let label = folder
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    store
        .add_source(kind, &folder, &label)
        .map_err(|e| e.to_string())?;
    let start = Instant::now();
    let mut last = Instant::now();
    let report = ingest_all(
        &mut store,
        &DedupConfig::default(),
        &AtomicBool::new(false),
        &mut |p| {
            if last.elapsed().as_secs() >= 2 {
                eprintln!("  {:?}: {} av {}", p.phase, p.done, p.total);
                last = Instant::now();
            }
        },
    )
    .map_err(|e| e.to_string())?;
    let s = &report.summary;
    println!(
        "Ferdig på {:.0} s: {} nye bilder. Katalogen har {} bilder, {} eksakte og {} transkodede dubletter, {} bare i skyen, {} uten miniatyr.",
        start.elapsed().as_secs_f64(),
        report.new_photos,
        s.photos,
        s.exact_duplicates,
        s.near_duplicates,
        s.cloud_only,
        s.without_preview
    );
    Ok(())
}

fn eval_make_gold(args: &[String]) -> Result<(), String> {
    let pdf = PathBuf::from(flag(args, "--pdf").ok_or("mangler --pdf ALBUM.pdf")?);
    let year: i32 = flag(args, "--aar")
        .ok_or("mangler --aar ÅR")?
        .parse()
        .map_err(|_| "ugyldig --aar")?;
    let name = flag(args, "--navn").ok_or("mangler --navn NAVN")?;
    let (store, dir) = open_store(args)?;
    eprintln!(
        "Henter bildene ut av {} og leter i biblioteket …",
        pdf.display()
    );
    let start = Instant::now();
    let report =
        p2a_eval::run::create_gold_set(&store, &pdf, year, &name).map_err(|e| e.to_string())?;
    let g = &report.gold;
    println!(
        "Gullsett «{name}»: {} sider, {} bilder i albumet ({} pynt hoppet over, {} i formater som ikke støttes).",
        report.stats.pages, g.bilder_i_album, report.stats.decorations, report.stats.unsupported
    );
    println!(
        "  {} sikre treff ({:.0} %), {} usikre, {} uten treff, søkt i {} bilder fra {}–{}. Tid: {:.0} s.",
        g.valgt.len(),
        g.match_rate() * 100.0,
        g.usikre.len(),
        g.uten_treff.len(),
        report.library_size,
        year - 1,
        year + 1,
        start.elapsed().as_secs_f64()
    );
    let review_dir = dir.join("eval-kontroll");
    std::fs::create_dir_all(&review_dir).map_err(|e| e.to_string())?;
    let review = review_dir.join(format!("{name}.html"));
    let html = p2a_eval::run::review_html(&store, &report).map_err(|e| e.to_string())?;
    std::fs::write(&review, html).map_err(|e| e.to_string())?;
    println!(
        "  Kontrollside (bare på denne maskinen): {}",
        review.display()
    );
    Ok(())
}

fn eval_list(args: &[String]) -> Result<(), String> {
    let (store, _) = open_store(args)?;
    let sets = p2a_eval::gold::GoldSet::load_all(&store).map_err(|e| e.to_string())?;
    if sets.is_empty() {
        println!("Ingen gullsett ennå. Lag ett med `p2a eval lag-gullsett`.");
    }
    for g in sets {
        println!(
            "{} ({}): {} bilder valgt av familien, {:.0} % sikre treff, fra «{}»",
            g.navn,
            g.aar,
            g.valgt.len(),
            g.match_rate() * 100.0,
            g.kilde
        );
    }
    Ok(())
}

fn eval_run(args: &[String]) -> Result<(), String> {
    let (store, _) = open_store(args)?;
    let results = p2a_eval::run::evaluate_all(&store).map_err(|e| e.to_string())?;
    if results.is_empty() {
        return Err("ingen gullsett; lag ett med `p2a eval lag-gullsett` først".into());
    }
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "ukjent".into());
    let mut section = format!(
        "\n## {} – utgangspunkt (prototypens utvalg) og hendelsesstyrt utkast, commit {commit}\n\n{}\n",
        p2a_core::TakenAt::from_unix_utc(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        )
        .map(|t| t.to_iso()[..10].to_string())
        .unwrap_or_default(),
        p2a_eval::metrics::MARKDOWN_HEADER
    );
    for r in &results {
        section.push_str(&p2a_eval::metrics::markdown_row(&r.gold.navn, &r.metrics));
        section.push('\n');
        section.push_str(&p2a_eval::metrics::markdown_row(
            &format!("{} (utkast)", r.gold.navn),
            &r.draft_metrics,
        ));
        section.push('\n');
    }
    print!("{section}");
    if let Some(path) = flag(args, "--resultater") {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        f.write_all(section.as_bytes()).map_err(|e| e.to_string())?;
        println!("\nLagt til i {path}.");
    }
    Ok(())
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

fn label_or(dir: &std::path::Path) -> String {
    dir.file_name()
        .map_or_else(|| "Bilder".into(), |n| n.to_string_lossy().into())
}

fn demo(args: &[String]) -> Result<(), String> {
    let data = PathBuf::from(flag(args, "--data").ok_or("mangler --data STI")?);
    let count: u32 = flag(args, "--antall")
        .map_or(Ok(200), |v| v.parse())
        .map_err(|_| "ugyldig --antall")?;
    let lib = data.join("demobilder");
    let (dropbox, icloud) = (
        lib.join("Dropbox/Camera Uploads"),
        lib.join("iCloud Photos"),
    );
    std::fs::create_dir_all(&dropbox).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&icloud).map_err(|e| e.to_string())?;
    (0..count).into_par_iter().for_each(|i| {
        let year = if i % 5 == 0 { 2010 } else { 2011 };
        let (month, day) = (1 + (i * 7) % 12, 1 + (i * 3) % 28);
        // Hjemme i Oslo, unntatt på turen.
        let mut gps = (59.91 + (i % 7) as f64 * 0.002, 10.75);
        let taken = if year == 2011 && i % 8 == 2 {
            // En tur over tre dager langt hjemmefra (som en gutte- eller jentetur).
            gps = (61.1 + (i % 5) as f64 * 0.01, 8.5);
            let k = i / 8;
            format!(
                "2011:08:{:02} {:02}:{:02}:00",
                12 + k % 3,
                10 + (k / 3) % 8,
                (k * 7) % 60
            )
        } else if year == 2011 && i % 8 == 6 {
            // Påskehelg hjemme: tre korte dager på rad, som appen foreslår å slå sammen.
            let k = i / 8;
            format!(
                "2011:04:{:02} {:02}:{:02}:00",
                22 + k % 3,
                11 + (k / 3) % 3,
                (k * 11) % 60
            )
        } else if year == 2011 && i % 4 == 1 {
            // Én stor hendelse (som en dåp): mange bilder samme dag, så historien får
            // flere sider i utkastet.
            let min = i / 4;
            format!(
                "2011:06:05 {:02}:{:02}:{:02}",
                11 + min / 60,
                min % 60,
                i % 60
            )
        } else {
            format!(
                "{year}:{month:02}:{day:02} {:02}:{:02}:00",
                8 + i % 12,
                i % 60
            )
        };
        let spec = ExifSpec {
            taken: Some(&taken),
            make: Some("Apple"),
            gps: Some(gps),
            ..Default::default()
        };
        let (w, h) = if i % 4 == 0 { (900, 1200) } else { (1200, 900) };
        let mut img = pattern(w, h, i);
        // Detaljer i de fleste bildene, så skarphetsmålet har noe å skille på.
        let detail = [0, 10, 25, 45][(i % 4) as usize] as f32;
        for (x, y, p) in img.enumerate_pixels_mut() {
            let v = ((x / 6 + y / 6 + i) % 2) as f32 * detail - detail / 2.0;
            for c in p.0.iter_mut() {
                *c = (*c as f32 + v).clamp(0.0, 255.0) as u8;
            }
        }
        // Hudtoner i halvparten av bildene: appens stedfortreder for «personer i bildet».
        if i % 2 == 0 {
            let (fw, fh) = (w / 4, h / 3);
            for y in h / 3..h / 3 + fh {
                for x in w / 3..w / 3 + fw {
                    img.get_pixel_mut(x, y).0 = [224, 172, 140];
                }
            }
        }
        let jpeg = insert_exif(&encode_jpeg(&img, 85), &tiff(&spec));
        let dir = if i % 2 == 0 { &dropbox } else { &icloud };
        std::fs::write(dir.join(format!("IMG_{i:04}.JPG")), &jpeg).expect("skrive");
        if i % 9 == 0 {
            // Samme bilde i begge kilder.
            std::fs::write(icloud.join(format!("kopi_{i:04}.JPG")), &jpeg).expect("skrive");
        }
    });
    std::fs::write(
        dropbox.join("ukjent.jpg"),
        encode_jpeg(&pattern(800, 600, 9999), 85),
    )
    .map_err(|e| e.to_string())?;

    let keys = FileKeyStore::in_dir(&data);
    let (store, recovery) = Store::create(&data, &keys).map_err(|e| e.to_string())?;
    store
        .add_source(SourceKind::Dropbox, &dropbox, "Camera Uploads")
        .map_err(|e| e.to_string())?;
    store
        .add_source(SourceKind::Icloud, &icloud, "iCloud Photos")
        .map_err(|e| e.to_string())?;
    // Ekte bilder med ansikter (for «Hvem er med?»), f.eks. egne bilder. Kopieres ikke.
    if let Some(dir) = flag(args, "--ansikter") {
        let dir = PathBuf::from(dir);
        store
            .add_source(SourceKind::Pc, &dir, &label_or(&dir))
            .map_err(|e| e.to_string())?;
    }
    println!("Demodata i {}", data.display());
    println!(
        "Gjenopprettingsnøkkel: {}",
        recovery.display_code().as_str()
    );
    println!("Start appen med: P2A_DATA_DIR={} pnpm dev", data.display());
    Ok(())
}
