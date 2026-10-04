//! Kjennetegn og landemerker for ett bilde, og kjennetegn for et ferdig opprettet ansikt
//! (112 × 112): `cargo run -p p2a-faces --example justert -- bilde.jpg opprettet.png`
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let engine = p2a_faces::FaceEngine::new().unwrap();
    let img = image::open(&a[1]).unwrap().to_rgb8();
    let d = &engine.detect(&img).unwrap()[0];
    println!("landemerker {:?}", d.landmarks);
    let ours = p2a_faces::align_face(&img, &d.landmarks);
    ours.save("/tmp/claude-0/-home-user-Fo2Album/86a5361a-7714-57b2-949f-1671615a48fd/scratchpad/faces/lena_aligned_ours.png").unwrap();
    let cv = image::open(&a[2]).unwrap().to_rgb8();
    let e1 = engine.embed_aligned(&cv).unwrap();
    println!(
        "{}",
        e1.0.iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
}
