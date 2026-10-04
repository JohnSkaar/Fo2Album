//! HEIC-dekoding med operativsystemets egne bildebiblioteker (ARCHITECTURE.md, «Bildedekoding»):
//! ImageIO på Mac og WIC på Windows. Unngår HEVC-patent- og LGPL-spørsmålene ved å
//! bruke dekoderne som allerede følger med (eller er installert på) maskinen.
//!
//! Bildet dekodes nedskalert (lengste side omtrent `max_side`) og uten rotering; den som
//! kaller bruker EXIF-orienteringen, slik som for JPEG.

#[derive(Debug, thiserror::Error)]
pub enum HeicError {
    /// Ingen HEIC-dekoder på denne maskinen (f.eks. Windows uten HEIF-utvidelsen).
    #[error("HEIC kan ikke dekodes på denne maskinen")]
    Unsupported,
    #[error("kunne ikke dekode HEIC-bildet: {0}")]
    Corrupt(String),
}

/// Dekodet bilde: RGB, 8 bit, rad for rad.
#[derive(Debug, Clone)]
pub struct Rgb {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    /// Originalens mål (før nedskalering, før rotering).
    pub full_width: u32,
    pub full_height: u32,
}

#[cfg(target_os = "macos")]
mod mac;
#[cfg(windows)]
mod win;

/// Dekoder HEIC/HEIF fra minnet.
pub fn decode(bytes: &[u8], max_side: u32) -> Result<Rgb, HeicError> {
    #[cfg(target_os = "macos")]
    return mac::decode(bytes, max_side);
    #[cfg(windows)]
    return win::decode(bytes, max_side);
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = (bytes, max_side);
        Err(HeicError::Unsupported)
    }
}

/// Koder RGB til HEIC. Bare for tester (Mac); `None` der plattformen ikke kan kode HEIC.
#[doc(hidden)]
pub fn encode_for_tests(width: u32, height: u32, rgb: &[u8]) -> Option<Vec<u8>> {
    #[cfg(target_os = "macos")]
    return mac::encode(width, height, rgb);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (width, height, rgb);
        None
    }
}

/// Mål som passer innenfor `max_side` med samme forhold. Brukes av plattformkoden.
#[cfg_attr(not(any(target_os = "macos", windows)), allow(dead_code))]
fn fit(w: u32, h: u32, max_side: u32) -> (u32, u32) {
    let long = w.max(h).max(1);
    if long <= max_side {
        return (w.max(1), h.max(1));
    }
    (
        ((w as u64 * max_side as u64) / long as u64).max(1) as u32,
        ((h as u64 * max_side as u64) / long as u64).max(1) as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_keeps_aspect() {
        assert_eq!(fit(4032, 3024, 1000), (1000, 750));
        assert_eq!(fit(3024, 4032, 1000), (750, 1000));
        assert_eq!(fit(800, 600, 1000), (800, 600));
    }

    /// Koder et syntetisk bilde til HEIC og dekoder det igjen. Kjører der plattformen kan
    /// kode HEIC (Mac); ellers sjekkes bare at dekoding feiler ryddig.
    #[test]
    fn round_trip_where_supported() {
        let (w, h) = (1200u32, 900u32);
        let rgb: Vec<u8> = (0..w * h)
            .flat_map(|i| {
                let (x, y) = (i % w, i / w);
                [(x * 255 / w) as u8, (y * 255 / h) as u8, 128]
            })
            .collect();
        match encode_for_tests(w, h, &rgb) {
            Some(heic) => {
                assert_eq!(&heic[4..8], b"ftyp", "ser ut som en HEIF-fil");
                let d = decode(&heic, 600).expect("dekode HEIC på Mac");
                assert_eq!((d.full_width, d.full_height), (w, h));
                assert!(d.width.max(d.height) <= 600 + 1 && d.width > d.height);
                assert_eq!(d.pixels.len(), (d.width * d.height * 3) as usize);
                // Øverste venstre hjørne er mørkt rødt/grønt, nederste høyre lyst.
                let px = |x: u32, y: u32| d.pixels[((y * d.width + x) * 3) as usize];
                assert!(
                    px(d.width - 2, 1) > px(1, 1) + 100,
                    "rødkanalen øker mot høyre"
                );
            }
            None => {
                assert!(decode(b"ikke heic", 600).is_err());
            }
        }
    }
}
