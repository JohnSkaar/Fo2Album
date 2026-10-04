//! Syntetiske testbilder med valgfri EXIF. Brukes av tester og ytelsesmålingen,
//! så vi aldri trenger ekte familiebilder i repoet.

use std::io::Cursor;

use exif::experimental::Writer;
use exif::{Field, In, Rational, Tag, Value};
use image::{ImageBuffer, Rgb, RgbImage};

#[derive(Debug, Clone, Default)]
pub struct ExifSpec<'a> {
    /// `ÅÅÅÅ:MM:DD TT:MM:SS`
    pub taken: Option<&'a str>,
    /// `+02:00`
    pub offset: Option<&'a str>,
    pub make: Option<&'a str>,
    pub model: Option<&'a str>,
    pub orientation: Option<u16>,
    pub gps: Option<(f64, f64)>,
}

/// Et deterministisk, fotoaktig bilde styrt av `seed`: en himmel-/bakkegradient med
/// noen myke fargeflekker («motiver») og litt skarp tekstur. Ulike frø gir tydelig ulike
/// bilder; samme frø i ulik størrelse gir samme bilde.
pub fn pattern(width: u32, height: u32, seed: u32) -> RgbImage {
    let mut state = seed.wrapping_mul(2_654_435_761).wrapping_add(0x9E37_79B9) | 1;
    let mut rnd = move || {
        // xorshift32
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state as f32) / (u32::MAX as f32)
    };
    let sky = [rnd() * 255.0, rnd() * 255.0, rnd() * 255.0];
    let ground = [rnd() * 255.0, rnd() * 255.0, rnd() * 255.0];
    let horizon = 0.3 + rnd() * 0.4;
    let blobs: Vec<([f32; 2], f32, [f32; 3])> = (0..6)
        .map(|_| {
            (
                [rnd(), rnd()],
                0.05 + rnd() * 0.2,
                [rnd() * 255.0, rnd() * 255.0, rnd() * 255.0],
            )
        })
        .collect();
    ImageBuffer::from_fn(width, height, |x, y| {
        let fx = x as f32 / width.max(1) as f32;
        let fy = y as f32 / height.max(1) as f32;
        let t = ((fy - horizon) * 8.0).clamp(-1.0, 1.0) * 0.5 + 0.5;
        let mut c = [0.0f32; 3];
        for i in 0..3 {
            c[i] = sky[i] * (1.0 - t) + ground[i] * t;
        }
        for (pos, r, col) in &blobs {
            let d2 = (fx - pos[0]).powi(2) + (fy - pos[1]).powi(2);
            let w = (-d2 / (r * r)).exp();
            for i in 0..3 {
                c[i] = c[i] * (1.0 - w) + col[i] * w;
            }
        }
        // Fin tekstur (høy frekvens), som skarphetsmål senere kan se.
        let texture = if (x / 3 + y / 3) % 2 == 0 { 6.0 } else { -6.0 };
        Rgb([
            (c[0] + texture).clamp(0.0, 255.0) as u8,
            (c[1] + texture).clamp(0.0, 255.0) as u8,
            (c[2] + texture).clamp(0.0, 255.0) as u8,
        ])
    })
}

pub fn encode_jpeg(img: &RgbImage, quality: u8) -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality)
        .encode_image(img)
        .expect("JPEG-koding");
    out
}

pub fn encode_png(img: &RgbImage) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png)
        .expect("PNG-koding");
    out.into_inner()
}

/// JPEG med mønster fra `seed` og ev. EXIF.
pub fn jpeg(width: u32, height: u32, seed: u32, exif: Option<&ExifSpec>) -> Vec<u8> {
    let bytes = encode_jpeg(&pattern(width, height, seed), 90);
    match exif {
        Some(spec) => insert_exif(&bytes, &tiff(spec)),
        None => bytes,
    }
}

/// Setter inn et APP1/Exif-segment rett etter SOI-markøren.
pub fn insert_exif(jpeg: &[u8], tiff: &[u8]) -> Vec<u8> {
    assert_eq!(&jpeg[..2], &[0xFF, 0xD8], "ikke en JPEG");
    let len = (2 + 6 + tiff.len()) as u16;
    let mut out = Vec::with_capacity(jpeg.len() + len as usize + 2);
    out.extend_from_slice(&jpeg[..2]);
    out.extend_from_slice(&[0xFF, 0xE1]);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(b"Exif\0\0");
    out.extend_from_slice(tiff);
    out.extend_from_slice(&jpeg[2..]);
    out
}

fn ascii(tag: Tag, s: &str) -> Field {
    Field {
        tag,
        ifd_num: In::PRIMARY,
        value: Value::Ascii(vec![s.as_bytes().to_vec()]),
    }
}

fn dms(v: f64) -> Value {
    let v = v.abs();
    let d = v.floor();
    let m = ((v - d) * 60.0).floor();
    let s = ((v - d) * 60.0 - m) * 60.0;
    Value::Rational(vec![
        Rational {
            num: d as u32,
            denom: 1,
        },
        Rational {
            num: m as u32,
            denom: 1,
        },
        Rational {
            num: (s * 10_000.0).round() as u32,
            denom: 10_000,
        },
    ])
}

/// TIFF-struktur med EXIF-feltene i `spec`.
pub fn tiff(spec: &ExifSpec) -> Vec<u8> {
    let mut fields = Vec::new();
    if let Some(t) = spec.taken {
        fields.push(ascii(Tag::DateTimeOriginal, t));
    }
    if let Some(o) = spec.offset {
        fields.push(ascii(Tag::OffsetTimeOriginal, o));
    }
    if let Some(m) = spec.make {
        fields.push(ascii(Tag::Make, m));
    }
    if let Some(m) = spec.model {
        fields.push(ascii(Tag::Model, m));
    }
    if let Some(o) = spec.orientation {
        fields.push(Field {
            tag: Tag::Orientation,
            ifd_num: In::PRIMARY,
            value: Value::Short(vec![o]),
        });
    }
    if let Some((lat, lon)) = spec.gps {
        fields.push(ascii(
            Tag::GPSLatitudeRef,
            if lat < 0.0 { "S" } else { "N" },
        ));
        fields.push(Field {
            tag: Tag::GPSLatitude,
            ifd_num: In::PRIMARY,
            value: dms(lat),
        });
        fields.push(ascii(
            Tag::GPSLongitudeRef,
            if lon < 0.0 { "W" } else { "E" },
        ));
        fields.push(Field {
            tag: Tag::GPSLongitude,
            ifd_num: In::PRIMARY,
            value: dms(lon),
        });
    }
    let mut w = Writer::new();
    for f in &fields {
        w.push_field(f);
    }
    let mut out = Cursor::new(Vec::new());
    w.write(&mut out, false).expect("EXIF-skriving");
    out.into_inner()
}
