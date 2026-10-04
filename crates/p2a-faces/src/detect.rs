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
}
