//! Opptakstid fra filnavn, når EXIF mangler (SCORING.md §2).
//!
//! Vanlige mønstre fra telefoner, WhatsApp og Dropbox-kameraopplasting.

use std::sync::LazyLock;

use p2a_core::TakenAt;
use regex::Regex;

static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        // IMG_20110714_123456, PXL_20110714_123456789, VID_…, Screenshot_20110714-123456,
        // 20110714_123456 (Samsung), signal-2011-07-14-123456
        r"(?:^|[^0-9])(?P<y>(?:19|20)\d{2})(?P<mo>\d{2})(?P<d>\d{2})[_\-T ]?(?P<h>\d{2})(?P<mi>\d{2})(?P<s>\d{2})",
        // 2011-07-14 12.34.56 (Dropbox), WhatsApp Image 2011-07-14 at 12.34.56,
        // Screenshot 2011-07-14 at 12.34.56, signal-2011-07-14-12-34-56
        r"(?:^|[^0-9])(?P<y>(?:19|20)\d{2})-(?P<mo>\d{2})-(?P<d>\d{2})(?:[ _\-T]|\sat\s|\skl\.?\s)(?P<h>\d{2})[.:\-](?P<mi>\d{2})[.:\-](?P<s>\d{2})",
        // Bare dato: IMG-20110714-WA0001 (WhatsApp), 2011-07-14
        r"(?:^|[^0-9])(?P<y>(?:19|20)\d{2})-?(?P<mo>\d{2})-?(?P<d>\d{2})(?:[^0-9]|$)",
    ]
    .iter()
    .map(|p| Regex::new(p).expect("gyldig regex"))
    .collect()
});

/// Prøver mønstrene i rekkefølge (mest presise først). Ugyldige datoer forkastes.
pub fn from_filename(name: &str) -> Option<TakenAt> {
    for re in PATTERNS.iter() {
        for caps in re.captures_iter(name) {
            let n = |k: &str| {
                caps.name(k)
                    .map(|m| m.as_str().parse::<u32>().unwrap_or(99))
            };
            let (h, mi, s) = (
                n("h").unwrap_or(0),
                n("mi").unwrap_or(0),
                n("s").unwrap_or(0),
            );
            if let Some(t) = TakenAt::new(
                n("y")? as i32,
                n("mo")? as u8,
                n("d")? as u8,
                h as u8,
                mi as u8,
                s as u8,
            ) {
                return Some(t);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iso(name: &str) -> Option<String> {
        from_filename(name).map(|t| t.to_iso())
    }

    #[test]
    fn common_phone_and_app_names() {
        let cases = [
            ("IMG_20110714_123456.jpg", "2011-07-14T12:34:56"),
            ("PXL_20230714_123456789.jpg", "2023-07-14T12:34:56"),
            ("20110714_123456.jpg", "2011-07-14T12:34:56"),
            ("Screenshot_20110714-123456.png", "2011-07-14T12:34:56"),
            ("2011-07-14 12.34.56.jpg", "2011-07-14T12:34:56"),
            (
                "WhatsApp Image 2011-07-14 at 12.34.56.jpeg",
                "2011-07-14T12:34:56",
            ),
            ("signal-2011-07-14-12-34-56-123.jpg", "2011-07-14T12:34:56"),
            ("IMG-20110714-WA0001.jpg", "2011-07-14T00:00:00"),
        ];
        for (name, want) in cases {
            assert_eq!(iso(name).as_deref(), Some(want), "{name}");
        }
    }

    #[test]
    fn rejects_names_without_valid_dates() {
        for name in [
            "IMG_0001.JPG",
            "DSC01234.JPG",
            "P1010123.JPG",
            "bursdag.jpg",
            "IMG_20111345_123456.jpg", // måned 13
            "IMG_20110230_000000.jpg", // 30. februar
        ] {
            assert_eq!(iso(name), None, "{name}");
        }
    }
}
