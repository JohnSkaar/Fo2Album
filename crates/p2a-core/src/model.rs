//! Domenetyper som deles mellom innlesing, lagring og poengsetting.

use std::fmt;
use std::str::FromStr;

/// Hvor en bildemappe kommer fra. Brukes til merking i grensesnittet og til
/// å velge beste kopi når samme bilde finnes i flere kilder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Pc,
    Dropbox,
    Icloud,
    GoogleDisk,
    /// Apple Bilder-biblioteket via PhotoKit (M1b).
    ApplePhotos,
}

impl SourceKind {
    pub const ALL: [SourceKind; 5] = [
        SourceKind::Pc,
        SourceKind::Dropbox,
        SourceKind::Icloud,
        SourceKind::GoogleDisk,
        SourceKind::ApplePhotos,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            SourceKind::Pc => "pc",
            SourceKind::Dropbox => "dropbox",
            SourceKind::Icloud => "icloud",
            SourceKind::GoogleDisk => "google_disk",
            SourceKind::ApplePhotos => "apple_photos",
        }
    }
}

/// Hvor opptakstiden er hentet fra, i synkende pålitelighet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DateSource {
    Exif,
    Filename,
    /// Filens endringstid. Usikker: synkronisering setter den ofte til nedlastingstidspunktet.
    FileModified,
}

impl DateSource {
    pub fn as_str(self) -> &'static str {
        match self {
            DateSource::Exif => "exif",
            DateSource::Filename => "filnavn",
            DateSource::FileModified => "endringstid",
        }
    }

    pub fn is_uncertain(self) -> bool {
        self == DateSource::FileModified
    }
}

/// Personens rolle i familien (SCORING.md §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Barn,
    Kjernefamilie,
    Besteforeldre,
    NaerFamilie,
    Venn,
    Annen,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Barn => "barn",
            Role::Kjernefamilie => "kjernefamilie",
            Role::Besteforeldre => "besteforeldre",
            Role::NaerFamilie => "naer_familie",
            Role::Venn => "venn",
            Role::Annen => "annen",
        }
    }
}

/// Type kommentar på et utkast (SCORING.md §6.3). Fritekst lagres, men tolkes ikke i v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommentKind {
    MerAv,
    MindreAv,
    Viktig,
    IkkeTaMed,
    Fritekst,
}

impl CommentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CommentKind::MerAv => "mer_av",
            CommentKind::MindreAv => "mindre_av",
            CommentKind::Viktig => "viktig",
            CommentKind::IkkeTaMed => "ikke_ta_med",
            CommentKind::Fritekst => "fritekst",
        }
    }
}

/// Ukjent verdi ved tolking av en lagret enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownValue(pub String);

impl fmt::Display for UnknownValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ukjent verdi «{}»", self.0)
    }
}

impl std::error::Error for UnknownValue {}

macro_rules! impl_from_str {
    ($t:ty, $all:expr) => {
        impl FromStr for $t {
            type Err = UnknownValue;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                $all.into_iter()
                    .find(|v| v.as_str() == s)
                    .ok_or_else(|| UnknownValue(s.to_string()))
            }
        }
    };
}

impl_from_str!(SourceKind, SourceKind::ALL);
impl_from_str!(
    DateSource,
    [
        DateSource::Exif,
        DateSource::Filename,
        DateSource::FileModified
    ]
);
impl_from_str!(
    Role,
    [
        Role::Barn,
        Role::Kjernefamilie,
        Role::Besteforeldre,
        Role::NaerFamilie,
        Role::Venn,
        Role::Annen,
    ]
);
impl_from_str!(
    CommentKind,
    [
        CommentKind::MerAv,
        CommentKind::MindreAv,
        CommentKind::Viktig,
        CommentKind::IkkeTaMed,
        CommentKind::Fritekst,
    ]
);

/// BLAKE3-hash av filinnholdet. Nøkkel for all cache (SCORING.md §1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl fmt::Debug for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentHash({}…)", &self.to_hex()[..12])
    }
}

/// Lokal opptakstid uten tidssone, slik kameraet skrev den, pluss ev. forskyvning fra UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TakenAt {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    /// Minutter fra UTC (EXIF `OffsetTimeOriginal`), hvis kjent.
    pub offset_minutes: Option<i16>,
}

impl TakenAt {
    /// Lager en tid hvis verdiene er gyldige (sjekker ikke skuddår for 29. februar).
    pub fn new(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Option<Self> {
        let valid = (1900..=2200).contains(&year)
            && (1..=12).contains(&month)
            && (1..=days_in_month(year, month)).contains(&day)
            && hour < 24
            && minute < 60
            && second < 61;
        valid.then_some(TakenAt {
            year,
            month,
            day,
            hour,
            minute,
            second,
            offset_minutes: None,
        })
    }

    /// ISO 8601 lokal tid, f.eks. `2011-07-14T12:34:56`. Sorterer riktig som tekst.
    pub fn to_iso(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// Leser formatet fra [`TakenAt::to_iso`].
    pub fn from_iso(s: &str) -> Option<Self> {
        let n = |r: std::ops::Range<usize>| s.get(r)?.parse::<u32>().ok();
        if s.len() != 19 {
            return None;
        }
        TakenAt::new(
            n(0..4)? as i32,
            n(5..7)? as u8,
            n(8..10)? as u8,
            n(11..13)? as u8,
            n(14..16)? as u8,
            n(17..19)? as u8,
        )
    }

    /// Fra sekunder siden 1970 (UTC). Brukes for filens endringstid når EXIF og filnavn mangler.
    pub fn from_unix_utc(secs: i64) -> Option<Self> {
        let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
        let (y, m, d) = civil_from_days(days);
        let mut t = TakenAt::new(
            y,
            m,
            d,
            (rem / 3600) as u8,
            (rem % 3600 / 60) as u8,
            (rem % 60) as u8,
        )?;
        t.offset_minutes = Some(0);
        Some(t)
    }

    /// Sekunder siden 1970-01-01 i lokal tid (uten tidssone). Brukes til tidsavstander.
    pub fn local_seconds(&self) -> i64 {
        let days = days_from_civil(self.year, self.month, self.day);
        days * 86_400 + self.hour as i64 * 3600 + self.minute as i64 * 60 + self.second as i64
    }
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i32, m: u8) -> u8 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(y) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Dager siden 1970-01-01 (Howard Hinnants algoritme).
fn days_from_civil(y: i32, m: u8, d: u8) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Omvendt av [`days_from_civil`].
fn civil_from_days(days: i64) -> (i32, u8, u8) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    ((yoe + era * 400 + i64::from(m <= 2)) as i32, m, d)
}

/// Om en fil kan leses lokalt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileStatus {
    Local,
    /// Plassholder: innholdet ligger bare i skyen. Leses ikke (ville utløst nedlasting).
    CloudOnly,
    /// Kunne ikke leses (skadet fil, manglende tilgang, ukjent format).
    Unreadable,
}

impl FileStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            FileStatus::Local => "lokal",
            FileStatus::CloudOnly => "bare_i_skyen",
            FileStatus::Unreadable => "uleselig",
        }
    }
}

impl_from_str!(
    FileStatus,
    [
        FileStatus::Local,
        FileStatus::CloudOnly,
        FileStatus::Unreadable
    ]
);

/// Det vi vet om ett unikt bilde etter innlesing.
#[derive(Debug, Clone, PartialEq)]
pub struct PhotoMeta {
    pub hash: ContentHash,
    /// jpeg | heic | png | webp
    pub format: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub orientation: Option<u16>,
    pub taken_at: Option<TakenAt>,
    pub date_source: Option<DateSource>,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub gps: Option<(f64, f64)>,
    /// Perseptuell hash (64 bit) for nesten like bilder.
    pub phash: Option<u64>,
    /// Enkle kvalitetsmål (prototypens). Erstattes av Q_tech i M3.
    pub quality: Option<BasicQuality>,
}

/// Prototypens enkle kvalitetsmål, alle 0–1. Grunnlaget for utgangspunktet
/// (`select::baseline`) som M3 skal slå på gullsettet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BasicQuality {
    /// Laplace-varians på hele bildet (log-skalert).
    pub sharp: f32,
    /// Nær middels lysstyrke og lite klipping.
    pub exposure: f32,
    /// Fargerikhet (Hasler og Süsstrunk).
    pub color: f32,
}

impl BasicQuality {
    /// Prototypens poeng, 0–100: skarphet 50 %, eksponering 30 %, farge 20 %.
    pub fn score(&self) -> f32 {
        100.0 * (0.5 * self.sharp + 0.3 * self.exposure + 0.2 * self.color)
    }
}

impl PhotoMeta {
    pub fn new(hash: ContentHash) -> Self {
        PhotoMeta {
            hash,
            format: None,
            width: None,
            height: None,
            orientation: None,
            taken_at: None,
            date_source: None,
            camera_make: None,
            camera_model: None,
            gps: None,
            phash: None,
            quality: None,
        }
    }

    /// Antall piksler, brukt for å velge beste kopi blant dubletter.
    pub fn pixels(&self) -> u64 {
        self.width.unwrap_or(0) as u64 * self.height.unwrap_or(0) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_unix_utc_matches_known_dates() {
        assert_eq!(
            TakenAt::from_unix_utc(0).unwrap().to_iso(),
            "1970-01-01T00:00:00"
        );
        assert_eq!(
            TakenAt::from_unix_utc(1_310_646_896).unwrap().to_iso(),
            "2011-07-14T12:34:56"
        );
        for secs in [951_782_400i64, 1_709_164_800, 4_102_444_799] {
            assert_eq!(TakenAt::from_unix_utc(secs).unwrap().local_seconds(), secs);
        }
    }

    #[test]
    fn enums_round_trip() {
        for k in SourceKind::ALL {
            assert_eq!(k.as_str().parse::<SourceKind>(), Ok(k));
        }
        assert_eq!("barn".parse::<Role>(), Ok(Role::Barn));
        assert!("ukjent".parse::<Role>().is_err());
    }

    #[test]
    fn taken_at_validates_and_round_trips() {
        let t = TakenAt::new(2011, 7, 14, 12, 34, 56).unwrap();
        assert_eq!(t.to_iso(), "2011-07-14T12:34:56");
        assert_eq!(TakenAt::from_iso(&t.to_iso()), Some(t));
        assert!(TakenAt::new(2011, 2, 29, 0, 0, 0).is_none());
        assert!(TakenAt::new(2012, 2, 29, 0, 0, 0).is_some());
        assert!(TakenAt::new(2011, 13, 1, 0, 0, 0).is_none());
        assert!(TakenAt::new(0, 1, 1, 0, 0, 0).is_none());
    }

    #[test]
    fn local_seconds_is_monotonic() {
        assert_eq!(
            TakenAt::new(1970, 1, 1, 0, 0, 0).unwrap().local_seconds(),
            0
        );
        assert_eq!(
            TakenAt::new(2000, 3, 1, 0, 0, 0).unwrap().local_seconds()
                - TakenAt::new(2000, 2, 28, 0, 0, 0).unwrap().local_seconds(),
            2 * 86_400
        );
    }

    #[test]
    fn content_hash_hex() {
        let h = ContentHash([0xab; 32]);
        assert_eq!(h.to_hex().len(), 64);
        assert!(h.to_hex().starts_with("abab"));
    }
}
