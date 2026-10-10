//! YuNet: forbehandling og avkoding av modellens utdata (som i OpenCVs `FaceDetectorYN`).

use image::{imageops::FilterType, RgbImage};
use tract_onnx::prelude::*;

/// Modellen tar et kvadratisk bilde på 640 × 640 punkter.
pub(crate) const SIZE: u32 = 640;
const STRIDES: [u32; 3] = [8, 16, 32];
const NMS_IOU: f32 = 0.3;

/// Et ansikt i bildet, i punkter i originalbildet.
#[derive(Clone, Debug, PartialEq)]
pub struct Detection {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Høyre øye, venstre øye, nesetipp, høyre og venstre munnvik (sett fra personen).
    pub landmarks: [[f32; 2]; 5],
    pub score: f32,
}

impl Detection {
    fn iou(&self, o: &Detection) -> f32 {
        let x1 = self.x.max(o.x);
        let y1 = self.y.max(o.y);
        let x2 = (self.x + self.w).min(o.x + o.w);
        let y2 = (self.y + self.h).min(o.y + o.h);
        let inter = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
        let union = self.w * self.h + o.w * o.h - inter;
        if union > 0.0 {
            inter / union
        } else {
            0.0
        }
    }
}

/// Skalerer bildet ned (eller opp) så den lengste siden er 640, legger det øverst til venstre
/// i et svart kvadrat, og lager en BGR-tensor (0–255) slik modellen er trent. Returnerer også
/// skalaen tilbake til originalbildet.
pub(crate) fn prepare(img: &RgbImage) -> (Tensor, f32) {
    let (w, h) = img.dimensions();
    let scale = SIZE as f32 / w.max(h).max(1) as f32;
    let (nw, nh) = (
        ((w as f32 * scale).round() as u32).clamp(1, SIZE),
        ((h as f32 * scale).round() as u32).clamp(1, SIZE),
    );
    let small = image::imageops::resize(img, nw, nh, FilterType::Triangle);
    let s = SIZE as usize;
    let arr = tract_ndarray::Array4::from_shape_fn((1, 3, s, s), |(_, c, y, x)| {
        if (x as u32) < nw && (y as u32) < nh {
            // BGR: kanal 0 er blå.
            small.get_pixel(x as u32, y as u32)[2 - c] as f32
        } else {
            0.0
        }
    });
    (arr.into(), 1.0 / scale)
}

/// Gjør modellens tolv utdata (cls, obj, bbox og kps for hvert av de tre nivåene) om til
/// ansikter, fjerner overlappende funn og skalerer tilbake til originalbildet.
pub(crate) fn decode(
    out: &[Vec<f32>],
    min_score: f32,
    back: f32,
    w: u32,
    h: u32,
) -> Vec<Detection> {
    let mut found = Vec::new();
    for (k, &stride) in STRIDES.iter().enumerate() {
        let (cls, obj, bbox, kps) = (&out[k], &out[3 + k], &out[6 + k], &out[9 + k]);
        let cols = SIZE / stride;
        for i in 0..cls.len() {
            let score = (cls[i].clamp(0.0, 1.0) * obj[i].clamp(0.0, 1.0)).sqrt();
            if score < min_score {
                continue;
            }
            let (r, c) = ((i as u32 / cols) as f32, (i as u32 % cols) as f32);
            let s = stride as f32;
            let cx = (c + bbox[i * 4]) * s;
            let cy = (r + bbox[i * 4 + 1]) * s;
            let bw = bbox[i * 4 + 2].exp() * s;
            let bh = bbox[i * 4 + 3].exp() * s;
            let mut landmarks = [[0.0; 2]; 5];
            for (n, lm) in landmarks.iter_mut().enumerate() {
                *lm = [
                    (kps[i * 10 + 2 * n] + c) * s * back,
                    (kps[i * 10 + 2 * n + 1] + r) * s * back,
                ];
            }
            let x = ((cx - bw / 2.0) * back).max(0.0);
            let y = ((cy - bh / 2.0) * back).max(0.0);
            found.push(Detection {
                x,
                y,
                w: (bw * back).min(w as f32 - x),
                h: (bh * back).min(h as f32 - y),
                landmarks,
                score,
            });
        }
    }
    nms(found)
}

fn nms(mut v: Vec<Detection>) -> Vec<Detection> {
    v.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut keep: Vec<Detection> = Vec::new();
    for d in v {
        if keep.iter().all(|k| k.iou(&d) <= NMS_IOU) {
            keep.push(d);
        }
    }
    keep
}

/// Utsnittene overlapper med en fjerdedel, så et ansikt alltid ligger helt inne i minst ett.
const TILE_OVERLAP: f32 = 0.25;
/// Høyst så mange utsnitt per bilde (ellers blir utsnittene større).
const MAX_TILES: usize = 9;

/// Et ansikt lavere enn denne andelen av bildet regnes som lite (personer lenger unna).
pub(crate) const SMALL_FACE: f32 = 0.06;

/// Om bildet bør gjennomsøkes i utsnitt: større enn det modellen ser i ett pass, og enten en
/// stor gruppe eller et lite ansikt (folk lenger unna). Vanlige familiebilder med noen få
/// personer på nært hold får ett pass, så innlesingen ikke blir tregere.
pub(crate) fn wants_tiles(found: &[Detection], w: u32, h: u32) -> bool {
    if w.max(h) as f32 <= SIZE as f32 * 1.2 {
        return false;
    }
    found.len() >= 8 || has_small(found, h)
}

pub(crate) fn has_small(found: &[Detection], h: u32) -> bool {
    found.iter().any(|d| d.h < h as f32 * SMALL_FACE)
}

/// Kvadratiske utsnitt (x, y, bredde, høyde) som dekker bildet med overlapp. Så små som mulig
/// (modellens størrelse, altså full oppløsning), men høyst [`MAX_TILES`].
pub(crate) fn tiles(w: u32, h: u32) -> Vec<(u32, u32, u32, u32)> {
    let axis = |len: u32, t: u32| -> Vec<u32> {
        if len <= t {
            return vec![0];
        }
        let step = (t as f32 * (1.0 - TILE_OVERLAP)) as u32;
        let n = (len - t).div_ceil(step) + 1;
        // Jevnt fordelt, siste slutter i kanten.
        (0..n)
            .map(|k| ((len - t) as u64 * k as u64 / (n - 1) as u64) as u32)
            .collect()
    };
    let mut t = SIZE;
    loop {
        let (xs, ys) = (axis(w, t), axis(h, t));
        if xs.len() * ys.len() <= MAX_TILES || t >= w.max(h) {
            return ys
                .iter()
                .flat_map(|&y| xs.iter().map(move |&x| (x, y, t.min(w), t.min(h))))
                .collect();
        }
        t = (t as f32 * 1.15) as u32;
    }
}

/// Et funn i et utsnitt, flyttet til hele bildet. Funn som berører kanten av utsnittet (der
/// den ikke er bildets kant), er trolig et halvt ansikt og forkastes; nabo-utsnittet har det.
pub(crate) fn from_tile(
    mut d: Detection,
    (x, y, tw, th): (u32, u32, u32, u32),
    w: u32,
    h: u32,
) -> Option<Detection> {
    const EDGE: f32 = 2.0;
    let cut_left = x > 0 && d.x <= EDGE;
    let cut_top = y > 0 && d.y <= EDGE;
    let cut_right = x + tw < w && d.x + d.w >= tw as f32 - EDGE;
    let cut_bottom = y + th < h && d.y + d.h >= th as f32 - EDGE;
    if cut_left || cut_top || cut_right || cut_bottom {
        return None;
    }
    let (fx, fy) = (x as f32, y as f32);
    d.x += fx;
    d.y += fy;
    for lm in &mut d.landmarks {
        lm[0] += fx;
        lm[1] += fy;
    }
    Some(d)
}

/// Funn fra hele bildet og utsnittene, uten dobbelttelling.
pub(crate) fn merge(all: Vec<Detection>) -> Vec<Detection> {
    nms(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn det(x: f32, score: f32) -> Detection {
        Detection {
            x,
            y: 0.0,
            w: 10.0,
            h: 10.0,
            landmarks: [[0.0; 2]; 5],
            score,
        }
    }

    #[test]
    fn overlapp_beholder_det_sikreste() {
        let out = nms(vec![det(0.0, 0.85), det(1.0, 0.95), det(30.0, 0.9)]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].score, 0.95);
        assert_eq!(out[1].x, 30.0);
    }

    #[test]
    fn forbehandling_bevarer_forholdet() {
        let img = RgbImage::from_pixel(1280, 640, image::Rgb([10, 20, 30]));
        let (t, back) = prepare(&img);
        assert_eq!(back, 2.0);
        let a = t.to_plain_array_view::<f32>().unwrap();
        // BGR øverst til venstre, svart under bildet.
        assert_eq!(
            [a[[0, 0, 0, 0]], a[[0, 1, 0, 0]], a[[0, 2, 0, 0]]],
            [30.0, 20.0, 10.0]
        );
        assert_eq!(a[[0, 0, 400, 0]], 0.0);
    }

    #[test]
    fn utsnitt_dekker_bildet_med_overlapp() {
        let t = tiles(1008, 756);
        assert_eq!(t.len(), 4);
        assert_eq!(t[0], (0, 0, 640, 640));
        assert_eq!(t[3], (368, 116, 640, 640));
        // Store bilder: høyst ni utsnitt, som dekker hele bildet.
        let t = tiles(4032, 3024);
        assert!(t.len() <= MAX_TILES, "{}", t.len());
        let (x, y, tw, th) = *t.last().unwrap();
        assert_eq!((x + tw, y + th), (4032, 3024));
        // Små bilder: ett utsnitt.
        assert_eq!(tiles(600, 400), vec![(0, 0, 600, 400)]);
    }

    #[test]
    fn halve_ansikter_i_kanten_av_et_utsnitt_forkastes() {
        let d = |x: f32| Detection {
            x,
            y: 100.0,
            w: 30.0,
            h: 30.0,
            landmarks: [[x + 10.0, 110.0]; 5],
            score: 0.9,
        };
        // Utsnitt nr. 2 fra venstre: venstre kant er inne i bildet.
        assert!(from_tile(d(0.0), (480, 0, 640, 640), 2000, 640).is_none());
        let moved = from_tile(d(100.0), (480, 0, 640, 640), 2000, 640).unwrap();
        assert_eq!((moved.x, moved.landmarks[0][0]), (580.0, 590.0));
        // Bildets egen kant er ikke en kuttkant.
        assert!(from_tile(d(0.0), (0, 0, 640, 640), 2000, 640).is_some());
        assert!(from_tile(d(611.0), (480, 0, 640, 640), 2000, 640).is_none());
    }

    #[test]
    fn gruppebilder_og_smaa_ansikter_gir_utsnitt() {
        let face = |h: f32| Detection {
            x: 0.0,
            y: 0.0,
            w: h,
            h,
            landmarks: [[0.0; 2]; 5],
            score: 0.9,
        };
        assert!(!wants_tiles(&[face(300.0)], 1008, 756));
        assert!(wants_tiles(&[face(30.0)], 1008, 756));
        assert!(
            !wants_tiles(&vec![face(200.0); 4], 1008, 756),
            "familiebilde"
        );
        assert!(wants_tiles(&vec![face(200.0); 8], 1008, 756), "stor gruppe");
        assert!(
            !wants_tiles(&[face(30.0)], 700, 500),
            "liten nok til ett pass"
        );
    }
}
