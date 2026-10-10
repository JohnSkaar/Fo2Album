//! Måler små ansikter (personer langt unna): hvor mange som finnes i ett pass over hele
//! bildet, og som i innlesingen (først lengste side 1008, så høyere oppløsning ved behov).
//! `cargo run --release -p p2a-faces --example gruppebilde -- bilde.jpg …`
use image::imageops::FilterType;
use p2a_faces::{FaceAnalyzer, FaceEngine, MORE_PIXELS};

fn main() {
    let engine = FaceEngine::new().expect("modellene");
    for path in std::env::args().skip(1) {
        let full = image::open(&path).expect("bildet");
        let small = full.resize(1008, 1008, FilterType::Triangle).to_rgb8();
        let t = std::time::Instant::now();
        let once = engine.detect_once(&small).expect("ett pass").len();
        let t1 = t.elapsed().as_millis();
        let t = std::time::Instant::now();
        let mut scan = engine.scan(&small).expect("analysen");
        let mut more = false;
        if scan.more_pixels && full.width().max(full.height()) > 1008 {
            let long = MORE_PIXELS.min(full.width().max(full.height()));
            let big = full.resize(long, long, FilterType::Triangle).to_rgb8();
            scan = engine.scan(&big).expect("høyere oppløsning");
            more = true;
        }
        println!(
            "{path}: ett pass {once} ({t1} ms); som innlesingen {} ({} ms{})",
            scan.faces.len(),
            t.elapsed().as_millis(),
            if more {
                ", lest på nytt i høyere oppløsning"
            } else {
                ""
            }
        );
    }
}
