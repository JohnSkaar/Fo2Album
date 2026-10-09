//! Ansikter: finner ansikter i et bilde og lager et kjennetegn (embedding) for hvert ansikt,
//! så de samme personene kan kjennes igjen på tvers av bildene.
//!
//! Alt skjer lokalt (CLAUDE.md, prinsipp 1). Modellene ligger inne i programmet og kjøres med
//! `tract`, en ONNX-motor skrevet i Rust, uten nettverk og uten eksterne biblioteker:
//!
//! - **YuNet** (OpenCV Zoo, MIT-lisens) finner ansikter og fem landemerker (øyne, nese,
//!   munnviker).
//! - **SFace** (OpenCV Zoo, Apache-2.0) gir 128 tall per ansikt. To ansikter med cosinus-
//!   likhet over [`SAME_PERSON`] er trolig samme person.
//!
//! Se `docs/ARCHITECTURE.md`, «Modeller», for kilde, versjon og sjekksum.

mod align;
pub mod cluster;
mod detect;

use image::RgbImage;
use tract_onnx::prelude::*;

pub use align::align_face;
pub use detect::Detection;

/// Cosinus-likhet der to ansikter regnes som samme person (OpenCVs anbefaling for SFace).
pub const SAME_PERSON: f32 = 0.363;

/// Antall tall i et ansiktskjennetegn.
pub const EMBEDDING_LEN: usize = 128;

const YUNET: &[u8] = include_bytes!("../models/face_detection_yunet_2023mar.onnx");
const SFACE: &[u8] = include_bytes!("../models/face_recognition_sface_2021dec.onnx");

#[derive(Debug, thiserror::Error)]
pub enum FaceError {
    #[error("ansiktsmodellen kunne ikke lastes eller kjøres: {0}")]
    Model(String),
}

impl From<TractError> for FaceError {
    fn from(e: TractError) -> Self {
        FaceError::Model(format!("{e:#}"))
    }
}

type Plan = Arc<TypedRunnableModel>;

/// Kjennetegnet for ett ansikt: 128 tall med lengde 1, så likhet er et prikkprodukt.
#[derive(Clone, Debug, PartialEq)]
pub struct Embedding(pub Vec<f32>);

impl Embedding {
    /// Normaliserer til lengde 1. Et nullvektor-kjennetegn blir stående som det er.
    pub fn normalized(mut v: Vec<f32>) -> Self {
        let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if n > 0.0 {
            v.iter_mut().for_each(|x| *x /= n);
        }
        Embedding(v)
    }

    /// Cosinus-likhet, fra −1 til 1.
    pub fn similarity(&self, other: &Embedding) -> f32 {
        self.0.iter().zip(&other.0).map(|(a, b)| a * b).sum()
    }

    /// Lagringsformat: små-endian f32.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        if b.len() != EMBEDDING_LEN * 4 {
            return None;
        }
        Some(Embedding(
            b.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect(),
        ))
    }
}

/// Et ansikt i et bilde, med kjennetegn og et mål på hvor godt ansiktet er.
#[derive(Clone, Debug)]
pub struct Face {
    pub detection: Detection,
    pub embedding: Embedding,
    /// Skarphet i ansiktsutsnittet, 0–1 (Laplace-varians, log-skalert).
    pub sharpness: f32,
}

impl Face {
    /// Ansiktets høyde som andel av bildets høyde.
    pub fn height_share(&self, image_height: u32) -> f32 {
        self.detection.h / image_height.max(1) as f32
    }
}

/// Modellene, lastet og optimalisert. Lag én og bruk den til alle bildene; den kan deles
/// mellom tråder.
/// Navn og versjon på modellene i [`FaceEngine`]. Lagres med ansiktene, så appen vet hvilke
/// bilder som må analyseres på nytt når modellen byttes.
pub const DEFAULT_MODEL: &str = "yunet-2023mar+sface-2021dec";

/// Det appen trenger fra en ansiktsmodell. Byttbart (beslutning 6. oktober 2026): en annen
/// motor (ONNX Runtime) eller leverandør (f.eks. Luxand) kan settes inn uten at resten av appen
/// endres. Kjennetegn fra ulike modeller kan ikke sammenlignes, så `model()` må endres når
/// modellen endres; da analyseres bildene på nytt, og navnene brukeren har gitt, beholdes.
pub trait FaceAnalyzer: Send + Sync {
    /// Navn og versjon, f.eks. [`DEFAULT_MODEL`].
    fn model(&self) -> &str;
    /// Ansiktene i bildet med kjennetegn og skarphet, største først.
    fn faces(&self, img: &RgbImage) -> Result<Vec<Face>, FaceError>;
    /// Likhet (cosinus) over denne er trolig samme person.
    fn same_person(&self) -> f32;
}

/// Modellen appen bruker nå.
pub fn default_analyzer() -> Result<Box<dyn FaceAnalyzer>, FaceError> {
    Ok(Box::new(FaceEngine::new()?))
}

pub struct FaceEngine {
    detector: Plan,
    recognizer: Plan,
    /// Minste poeng for at noe regnes som et ansikt.
    pub min_score: f32,
}

impl FaceEngine {
    /// Laster modellene som ligger inne i programmet.
    pub fn new() -> Result<Self, FaceError> {
        let detector = tract_onnx::onnx()
            .model_for_read(&mut std::io::Cursor::new(YUNET))?
            .with_input_fact(
                0,
                f32::fact([1, 3, detect::SIZE as usize, detect::SIZE as usize]).into(),
            )?
            .into_optimized()?
            .into_runnable()?;
        let mut rec = tract_onnx::onnx().model_for_read(&mut std::io::Cursor::new(SFACE))?;
        // SFace (2021) har vektene som ekstra innganger med startverdier; bare `data` er en
        // ekte inngang.
        rec.set_input_names(["data"])?;
        let recognizer = rec
            .with_input_fact(0, f32::fact([1, 3, 112, 112]).into())?
            .into_optimized()?
            .into_runnable()?;
        Ok(FaceEngine {
            detector,
            recognizer,
            min_score: 0.8,
        })
    }

    /// Finner ansiktene i bildet, sortert etter størrelse (største først).
    pub fn detect(&self, img: &RgbImage) -> Result<Vec<Detection>, FaceError> {
        let (input, scale) = detect::prepare(img);
        let out = self.detector.run(tvec!(input.into()))?;
        let views: Vec<_> = out
            .iter()
            .map(|t| {
                t.to_plain_array_view::<f32>()
                    .map(|v| v.iter().copied().collect::<Vec<_>>())
            })
            .collect::<Result<_, _>>()?;
        let mut found = detect::decode(&views, self.min_score, scale, img.width(), img.height());
        found.sort_by(|a, b| (b.w * b.h).total_cmp(&(a.w * a.h)));
        Ok(found)
    }

    /// Kjennetegnet for ett ansikt.
    pub fn embed(&self, img: &RgbImage, d: &Detection) -> Result<Embedding, FaceError> {
        self.embed_aligned(&align_face(img, &d.landmarks))
    }

    /// Kjennetegnet for et ansikt som allerede er rettet opp til 112 × 112 (se [`align_face`]).
    pub fn embed_aligned(&self, face: &RgbImage) -> Result<Embedding, FaceError> {
        let input = Tensor::from(tract_ndarray::Array4::from_shape_fn(
            (1, 3, 112, 112),
            |(_, c, y, x)| face.get_pixel(x as u32, y as u32)[c] as f32,
        ));
        let out = self.recognizer.run(tvec!(input.into()))?;
        let v = out[0]
            .to_plain_array_view::<f32>()?
            .iter()
            .copied()
            .collect();
        Ok(Embedding::normalized(v))
    }

    /// Alle ansiktene i bildet, med kjennetegn og skarphet.
    pub fn faces(&self, img: &RgbImage) -> Result<Vec<Face>, FaceError> {
        self.detect(img)?
            .into_iter()
            .map(|d| {
                let aligned = align_face(img, &d.landmarks);
                Ok(Face {
                    embedding: self.embed_aligned(&aligned)?,
                    sharpness: align::sharpness(&aligned),
                    detection: d,
                })
            })
            .collect()
    }
}

impl FaceAnalyzer for FaceEngine {
    fn model(&self) -> &str {
        DEFAULT_MODEL
    }

    fn faces(&self, img: &RgbImage) -> Result<Vec<Face>, FaceError> {
        FaceEngine::faces(self, img)
    }

    fn same_person(&self) -> f32 {
        SAME_PERSON
    }
}
