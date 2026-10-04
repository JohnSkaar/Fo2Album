//! Retter opp et ansikt til 112 × 112 punkter med øyne, nese og munn på faste steder, slik
//! SFace er trent (samme mal og metode som OpenCVs `FaceRecognizerSF::alignCrop`).

use image::{Rgb, RgbImage};

/// Hvor de fem landemerkene skal havne i det oppruttede ansiktet.
const TEMPLATE: [[f32; 2]; 5] = [
    [38.2946, 51.6963],
    [73.5318, 51.5014],
    [56.0252, 71.7366],
    [41.5493, 92.3655],
    [70.7299, 92.2041],
];

/// Likhetstransform (skala, rotasjon, flytting) som passer `src` best til `dst` (Umeyama).
/// Gir `[a, -b, tx; b, a, ty]` som `(a, b, tx, ty)`.
fn similarity(src: &[[f32; 2]; 5], dst: &[[f32; 2]; 5]) -> (f32, f32, f32, f32) {
    let n = 5.0;
    let mean = |p: &[[f32; 2]; 5]| {
        let s = p.iter().fold([0.0, 0.0], |a, q| [a[0] + q[0], a[1] + q[1]]);
        [s[0] / n, s[1] / n]
    };
    let (ms, md) = (mean(src), mean(dst));
    let (mut sxx, mut sxy, mut var) = (0.0, 0.0, 0.0);
    for (s, d) in src.iter().zip(dst) {
        let (sx, sy) = (s[0] - ms[0], s[1] - ms[1]);
        let (dx, dy) = (d[0] - md[0], d[1] - md[1]);
        // For en likhetstransform i 2D reduseres Umeyama til disse summene.
        sxx += sx * dx + sy * dy;
        sxy += sx * dy - sy * dx;
        var += sx * sx + sy * sy;
    }
    if var <= f32::EPSILON {
        return (1.0, 0.0, md[0] - ms[0], md[1] - ms[1]);
    }
    let a = sxx / var;
    let b = sxy / var;
    let tx = md[0] - (a * ms[0] - b * ms[1]);
    let ty = md[1] - (b * ms[0] + a * ms[1]);
    (a, b, tx, ty)
}

/// Ansiktet, rettet opp og beskåret til 112 × 112 (bilineær sampling, svart utenfor bildet).
pub fn align_face(img: &RgbImage, landmarks: &[[f32; 2]; 5]) -> RgbImage {
    let (a, b, tx, ty) = similarity(landmarks, &TEMPLATE);
    // Inversen av [a -b; b a] er [a b; -b a] / (a² + b²).
    let det = a * a + b * b;
    let (w, h) = (img.width() as f32, img.height() as f32);
    RgbImage::from_fn(112, 112, |u, v| {
        let (du, dv) = (u as f32 - tx, v as f32 - ty);
        let x = (a * du + b * dv) / det;
        let y = (-b * du + a * dv) / det;
        if x < 0.0 || y < 0.0 || x > w - 1.0 || y > h - 1.0 {
            return Rgb([0, 0, 0]);
        }
        let (x0, y0) = (x.floor() as u32, y.floor() as u32);
        let (x1, y1) = (
            (x0 + 1).min(img.width() - 1),
            (y0 + 1).min(img.height() - 1),
        );
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let p = |xx, yy, c: usize| img.get_pixel(xx, yy)[c] as f32;
        Rgb(std::array::from_fn(|c| {
            let top = p(x0, y0, c) * (1.0 - fx) + p(x1, y0, c) * fx;
            let bot = p(x0, y1, c) * (1.0 - fx) + p(x1, y1, c) * fx;
            (top * (1.0 - fy) + bot * fy).round().clamp(0.0, 255.0) as u8
        }))
    })
}

/// Skarphet i et opprettet ansikt: Laplace-varians på gråtoner, log-skalert til 0–1
/// (samme skala som `subject_sharpness` i p2a-ingest).
pub(crate) fn sharpness(face: &RgbImage) -> f32 {
    let g: Vec<f32> = face
        .pixels()
        .map(|p| 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32)
        .collect();
    let (w, h) = (face.width() as usize, face.height() as usize);
    let mut vals = Vec::with_capacity(w * h);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = y * w + x;
            vals.push(g[i - 1] + g[i + 1] + g[i - w] + g[i + w] - 4.0 * g[i]);
        }
    }
    let n = vals.len() as f32;
    let mean = vals.iter().sum::<f32>() / n;
    let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n;
    (var.ln_1p() / 3000f32.ln_1p()).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malen_gir_identitet() {
        let (a, b, tx, ty) = similarity(&TEMPLATE, &TEMPLATE);
        assert!((a - 1.0).abs() < 1e-5 && b.abs() < 1e-5 && tx.abs() < 1e-3 && ty.abs() < 1e-3);
    }

    #[test]
    fn dobbel_storrelse_og_flytting() {
        let src = TEMPLATE.map(|[x, y]| [x * 2.0 + 100.0, y * 2.0 + 50.0]);
        let (a, b, tx, ty) = similarity(&src, &TEMPLATE);
        assert!((a - 0.5).abs() < 1e-4 && b.abs() < 1e-4);
        assert!((tx + 50.0).abs() < 1e-2 && (ty + 25.0).abs() < 1e-2);
    }

    #[test]
    fn oppretting_henter_riktige_punkter() {
        // Et bilde der hvert punkt har x i rødt og y i grønt; malen plassert 1:1 gir samme bilde.
        let img = RgbImage::from_fn(200, 200, |x, y| Rgb([x as u8, y as u8, 0]));
        let out = align_face(&img, &TEMPLATE);
        assert_eq!(out.get_pixel(10, 20).0, [10, 20, 0]);
        assert_eq!(out.get_pixel(100, 90).0, [100, 90, 0]);
    }
}
