//! Skriver ansiktene i bildene som JSON-linjer, for sammenligning med OpenCV:
//! `cargo run --release -p p2a-faces --example sammenlign -- bilde1.jpg bilde2.png …`
fn main() {
    let engine = p2a_faces::FaceEngine::new().expect("modellene");
    for path in std::env::args().skip(1) {
        let img = image::open(&path).expect("bildet").to_rgb8();
        let t = std::time::Instant::now();
        let faces = engine.faces(&img).expect("ansiktene");
        let list: Vec<String> = faces
            .iter()
            .map(|f| {
                let d = &f.detection;
                format!(
                    "{{\"box\":[{:.1},{:.1},{:.1},{:.1}],\"score\":{:.3},\"sharp\":{:.3},\"emb\":[{}]}}",
                    d.x, d.y, d.w, d.h, d.score, f.sharpness,
                    f.embedding.0.iter().map(|x| format!("{x:.6}")).collect::<Vec<_>>().join(",")
                )
            })
            .collect();
        println!(
            "{{\"file\":\"{path}\",\"ms\":{},\"faces\":[{}]}}",
            t.elapsed().as_millis(),
            list.join(",")
        );
    }
}
