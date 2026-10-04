//! EXIF-metadata: opptakstid med tidssone, kamera, orientering, GPS og størrelse.
//! Virker for JPEG, HEIC/HEIF, PNG og WebP (kamadak-exif).

use std::io::{BufRead, Seek};
use std::path::Path;

use exif::{In, Tag, Value};
use p2a_core::TakenAt;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExifData {
    pub taken_at: Option<TakenAt>,
    pub make: Option<String>,
    pub model: Option<String>,
    /// EXIF-orientering 1–8.
    pub orientation: Option<u16>,
    /// (breddegrad, lengdegrad) i desimalgrader.
    pub gps: Option<(f64, f64)>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl ExifData {
    /// Har kameraet skrevet EXIF (ikke bare et program som har lagret filen)?
    pub fn has_camera(&self) -> bool {
        self.make.is_some() || self.model.is_some()
    }
}

/// `None` hvis filen ikke har lesbar EXIF.
pub fn read_file(path: &Path) -> Option<ExifData> {
    let file = std::fs::File::open(path).ok()?;
    read(&mut std::io::BufReader::new(file))
}

pub fn read<R: BufRead + Seek>(reader: &mut R) -> Option<ExifData> {
    let exif = exif::Reader::new().read_from_container(reader).ok()?;
    let field = |tag| exif.get_field(tag, In::PRIMARY);
    let ascii = |tag| {
        match &field(tag)?.value {
            Value::Ascii(v) => v.first().map(|b| {
                String::from_utf8_lossy(b)
                    .trim_matches(|c: char| c == '\0' || c.is_whitespace())
                    .to_string()
            }),
            _ => None,
        }
        .filter(|s| !s.is_empty())
    };
    let uint = |tag| field(tag).and_then(|f| f.value.get_uint(0));

    let taken_at = [Tag::DateTimeOriginal, Tag::DateTimeDigitized]
        .into_iter()
        .find_map(|tag| {
            let raw = ascii(tag)?;
            let dt = exif::DateTime::from_ascii(raw.as_bytes()).ok()?;
            let mut t = TakenAt::new(
                dt.year as i32,
                dt.month,
                dt.day,
                dt.hour,
                dt.minute,
                dt.second,
            )?;
            let offset_tag = if tag == Tag::DateTimeOriginal {
                Tag::OffsetTimeOriginal
            } else {
                Tag::OffsetTimeDigitized
            };
            t.offset_minutes = ascii(offset_tag).and_then(|s| parse_offset(&s));
            Some(t)
        });

    let gps = (|| {
        let coord = |tag, ref_tag, neg: &str| {
            let Value::Rational(r) = &field(tag)?.value else {
                return None;
            };
            if r.len() < 3 {
                return None;
            }
            let v = r[0].to_f64() + r[1].to_f64() / 60.0 + r[2].to_f64() / 3600.0;
            let sign = if ascii(ref_tag).as_deref() == Some(neg) {
                -1.0
            } else {
                1.0
            };
            v.is_finite().then_some(sign * v)
        };
        let lat = coord(Tag::GPSLatitude, Tag::GPSLatitudeRef, "S")?;
        let lon = coord(Tag::GPSLongitude, Tag::GPSLongitudeRef, "W")?;
        // 0,0 betyr nesten alltid «mangler».
        ((lat, lon) != (0.0, 0.0) && lat.abs() <= 90.0 && lon.abs() <= 180.0).then_some((lat, lon))
    })();

    Some(ExifData {
        taken_at,
        make: ascii(Tag::Make),
        model: ascii(Tag::Model),
        orientation: uint(Tag::Orientation)
            .and_then(|o| u16::try_from(o).ok())
            .filter(|o| (1..=8).contains(o)),
        gps,
        width: uint(Tag::PixelXDimension).filter(|&w| w > 0),
        height: uint(Tag::PixelYDimension).filter(|&h| h > 0),
    })
}

/// `+02:00` → 120, `-05:30` → -330.
fn parse_offset(s: &str) -> Option<i16> {
    let (sign, rest) = match s.as_bytes().first()? {
        b'+' => (1, &s[1..]),
        b'-' => (-1, &s[1..]),
        _ => return None,
    };
    let (h, m) = rest.split_once(':')?;
    let (h, m): (i16, i16) = (h.parse().ok()?, m.parse().ok()?);
    (h <= 14 && m < 60).then_some(sign * (h * 60 + m))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{jpeg, ExifSpec};

    #[test]
    fn reads_full_exif_from_jpeg() {
        let bytes = jpeg(
            64,
            48,
            1,
            Some(&ExifSpec {
                taken: Some("2011:07:14 12:34:56"),
                offset: Some("+02:00"),
                make: Some("Apple"),
                model: Some("iPhone 4"),
                orientation: Some(6),
                gps: Some((58.1467, 7.9956)),
            }),
        );
        let e = read(&mut std::io::Cursor::new(bytes)).unwrap();
        let t = e.taken_at.unwrap();
        assert_eq!(t.to_iso(), "2011-07-14T12:34:56");
        assert_eq!(t.offset_minutes, Some(120));
        assert_eq!(e.make.as_deref(), Some("Apple"));
        assert_eq!(e.model.as_deref(), Some("iPhone 4"));
        assert_eq!(e.orientation, Some(6));
        let (lat, lon) = e.gps.unwrap();
        assert!((lat - 58.1467).abs() < 1e-4 && (lon - 7.9956).abs() < 1e-4);
        assert!(e.has_camera());
    }

    #[test]
    fn missing_or_zero_dates_give_none() {
        let no_exif = jpeg(16, 16, 2, None);
        assert_eq!(read(&mut std::io::Cursor::new(no_exif)), None);

        let zero = jpeg(
            16,
            16,
            3,
            Some(&ExifSpec {
                taken: Some("0000:00:00 00:00:00"),
                ..Default::default()
            }),
        );
        let e = read(&mut std::io::Cursor::new(zero)).unwrap();
        assert_eq!(e.taken_at, None);
        assert!(!e.has_camera());
    }

    #[test]
    fn offsets() {
        assert_eq!(parse_offset("+02:00"), Some(120));
        assert_eq!(parse_offset("-05:30"), Some(-330));
        assert_eq!(parse_offset("02:00"), None);
        assert_eq!(parse_offset("+25:00"), None);
    }
}
