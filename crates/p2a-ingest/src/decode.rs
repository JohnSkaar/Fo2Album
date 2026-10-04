//! Dekoding av bilder og miniatyrer.
//!
//! JPEG dekodes direkte i redusert størrelse (`jpeg-decoder`), PNG og WebP med `image`.
//! HEIC dekodes med plattformens egne API-er (M1.4); til da får HEIC-bilder metadata og
//! hash, men ingen miniatyr.

use std::io::Cursor;
use std::path::Path;

use image::metadata::Orientation;
use image::{DynamicImage, ImageFormat, ImageReader};

/// Lengste side på miniatyren. Holder til rutenettet på skjermer med høy oppløsning.
pub const THUMB_MAX: u32 = 400;
const THUMB_QUALITY: u8 = 78;
/// JPEG dekodes direkte i redusert størrelse (1/2, 1/4 eller 1/8), men aldri mindre enn
/// dette på lengste side. Nok til miniatyr, pHash og kvalitetsmål (M3).
pub const ANALYSIS_MIN: u32 = 1000;

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    /// Formatet kan ikke dekodes på denne plattformen ennå (f.eks. HEIC før M1.4).
    #[error("formatet støttes ikke ennå")]
    Unsupported,
    #[error("kunne ikke dekode bildet: {0}")]
    Corrupt(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Et dekodet bilde, rotert etter EXIF-orienteringen. `image` kan være nedskalert;
/// `width` og `height` er alltid målene til originalen (etter rotering).
pub struct Decoded {
    pub image: DynamicImage,
    pub width: u32,
    pub height: u32,
}

pub fn decode_file(
    path: &Path,
    format: Option<&str>,
    orientation: Option<u16>,
) -> Result<Decoded, DecodeError> {
    let image_format = match format {
        Some("jpeg") => ImageFormat::Jpeg,
        Some("png") => ImageFormat::Png,
        Some("webp") => ImageFormat::WebP,
        _ => return Err(DecodeError::Unsupported),
    };
    let bytes = std::fs::read(path)?;
    let (mut image, (mut width, mut height)) = if image_format == ImageFormat::Jpeg {
        decode_jpeg_scaled(&bytes)?
    } else {
        let mut reader = ImageReader::new(Cursor::new(bytes));
        reader.set_format(image_format);
        let img = reader
            .decode()
            .map_err(|e| DecodeError::Corrupt(e.to_string()))?;
        let dims = (img.width(), img.height());
        (img, dims)
    };
    if let Some(o) = orientation.and_then(|o| Orientation::from_exif(o as u8)) {
        image.apply_orientation(o);
        if matches!(orientation, Some(5..=8)) {
            std::mem::swap(&mut width, &mut height);
        }
    }
    Ok(Decoded {
        image,
        width,
        height,
    })
}

/// Dekoder JPEG i redusert størrelse. Returnerer bildet og originalens mål.
fn decode_jpeg_scaled(bytes: &[u8]) -> Result<(DynamicImage, (u32, u32)), DecodeError> {
    let corrupt = |e: jpeg_decoder::Error| DecodeError::Corrupt(e.to_string());
    let no_header = || DecodeError::Corrupt("mangler JPEG-hode".into());
    let mut dec = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    dec.read_info().map_err(corrupt)?;
    let info = dec.info().ok_or_else(no_header)?;
    let (fw, fh) = (info.width as u32, info.height as u32);
    let long = fw.max(fh).max(1);
    let target = ANALYSIS_MIN.min(long);
    let request = |v: u32| (v * target).div_ceil(long).max(1) as u16;
    let (w, h) = dec.scale(request(fw), request(fh)).map_err(corrupt)?;
    let pixels = dec.decode().map_err(corrupt)?;
    let format = dec.info().ok_or_else(no_header)?.pixel_format;
    let (w, h) = (w as u32, h as u32);
    let bad = || DecodeError::Corrupt("uventet antall piksler".into());
    let image = match format {
        jpeg_decoder::PixelFormat::RGB24 => {
            DynamicImage::ImageRgb8(image::RgbImage::from_raw(w, h, pixels).ok_or_else(bad)?)
        }
        jpeg_decoder::PixelFormat::L8 => {
            DynamicImage::ImageLuma8(image::GrayImage::from_raw(w, h, pixels).ok_or_else(bad)?)
        }
        jpeg_decoder::PixelFormat::L16 => {
            let px: Vec<u8> = pixels.chunks_exact(2).map(|c| c[0]).collect();
            DynamicImage::ImageLuma8(image::GrayImage::from_raw(w, h, px).ok_or_else(bad)?)
        }
        jpeg_decoder::PixelFormat::CMYK32 => {
            // Adobe-CMYK i JPEG er lagret invertert.
            let px: Vec<u8> = pixels
                .chunks_exact(4)
                .flat_map(|c| {
                    let k = c[3] as u16;
                    [c[0], c[1], c[2]].map(|v| (v as u16 * k / 255) as u8)
                })
                .collect();
            DynamicImage::ImageRgb8(image::RgbImage::from_raw(w, h, px).ok_or_else(bad)?)
        }
    };
    Ok((image, (fw, fh)))
}

/// JPEG-miniatyr som passer innenfor [`THUMB_MAX`] × [`THUMB_MAX`].
pub fn thumbnail_jpeg(decoded: &Decoded) -> Vec<u8> {
    let thumb = decoded.image.thumbnail(THUMB_MAX, THUMB_MAX).to_rgb8();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, THUMB_QUALITY)
        .encode_image(&thumb)
        .expect("JPEG-koding i minnet feiler ikke");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth;

    #[test]
    fn decodes_and_applies_orientation() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.jpg");
        std::fs::write(&p, synth::jpeg(80, 40, 1, None)).unwrap();
        let d = decode_file(&p, Some("jpeg"), None).unwrap();
        assert_eq!((d.width, d.height), (80, 40));
        // Orientering 6 = rotert 90°: bredde og høyde bytter plass.
        let d = decode_file(&p, Some("jpeg"), Some(6)).unwrap();
        assert_eq!((d.width, d.height), (40, 80));
        assert_eq!((d.image.width(), d.image.height()), (40, 80));
    }

    #[test]
    fn large_jpegs_are_decoded_at_reduced_size() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("stor.jpg");
        std::fs::write(&p, synth::jpeg(4032, 3024, 5, None)).unwrap();
        let d = decode_file(&p, Some("jpeg"), None).unwrap();
        assert_eq!((d.width, d.height), (4032, 3024), "originalens mål");
        let long = d.image.width().max(d.image.height());
        assert!(
            (ANALYSIS_MIN..4032).contains(&long),
            "dekodet med lengste side {long}"
        );
    }

    #[test]
    fn thumbnail_fits_and_keeps_aspect() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.png");
        std::fs::write(&p, synth::encode_png(&synth::pattern(1200, 800, 3))).unwrap();
        let d = decode_file(&p, Some("png"), None).unwrap();
        let t = image::load_from_memory(&thumbnail_jpeg(&d)).unwrap();
        assert_eq!((t.width(), t.height()), (400, 267));
    }

    #[test]
    fn corrupt_and_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.jpg");
        std::fs::write(&p, b"ikke et bilde").unwrap();
        assert!(matches!(
            decode_file(&p, Some("jpeg"), None),
            Err(DecodeError::Corrupt(_))
        ));
        assert!(matches!(
            decode_file(&p, Some("heic"), None),
            Err(DecodeError::Unsupported)
        ));
    }
}
