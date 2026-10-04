//! Det appen lærer av brukerens bytter (SCORING.md §6.3).
//!
//! Hver gang brukeren tar med, tar bort eller bytter et bilde, lagres handlingen, og appen
//! spør forsiktig om hvorfor. Svarene flytter noen få, forklarbare vekter litt om gangen.
//! Alt er deterministisk: samme logg gir alltid samme preferanser, og loggen ligger kryptert
//! hos familien, så neste års album starter der dette slapp.

use std::str::FromStr;

use crate::model::UnknownValue;
use crate::BasicQuality;

/// Hva brukeren gjorde med et bilde i utkastet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    TaMed,
    TaBort,
    /// Byttet ut ett bilde med et annet.
    Bytt,
    /// Ga bildet mer plass (egen side).
    Fremhev,
    /// Med, men mindre plass.
    Demp,
    /// Tok bort en hel dag (hendelse).
    FjernDag,
}

impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Action::TaMed => "ta_med",
            Action::TaBort => "ta_bort",
            Action::Bytt => "bytt",
            Action::Fremhev => "fremhev",
            Action::Demp => "demp",
            Action::FjernDag => "fjern_dag",
        }
    }
}

impl FromStr for Action {
    type Err = UnknownValue;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "ta_med" => Action::TaMed,
            "ta_bort" => Action::TaBort,
            "bytt" => Action::Bytt,
            "fremhev" => Action::Fremhev,
            "demp" => Action::Demp,
            "fjern_dag" => Action::FjernDag,
            _ => return Err(UnknownValue(s.to_string())),
        })
    }
}

/// Svarene brukeren kan gi på «Hvorfor?». Fast liste, så de kan brukes direkte som signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FeedbackReason {
    // Når et bilde tas bort eller byttes ut.
    Uskarpt,
    DaarligLys,
    ForLikt,
    ForMangeHerfra,
    LikerIkke,
    Privat,
    /// «Bare en ting, ingen personer.»
    Gjenstand,
    /// En hel dag som ikke er viktig.
    UviktigDag,
    // Når et bilde tas med eller byttes inn.
    ViktigOyeblikk,
    ViktigPerson,
    FintBilde,
    ManglerHerfra,
    Skarpere,
    /// «Fin stemning» (et bilde uten personer).
    Stemning,
}

impl FeedbackReason {
    pub const ALL: [FeedbackReason; 14] = [
        FeedbackReason::Uskarpt,
        FeedbackReason::DaarligLys,
        FeedbackReason::ForLikt,
        FeedbackReason::ForMangeHerfra,
        FeedbackReason::LikerIkke,
        FeedbackReason::Privat,
        FeedbackReason::ViktigOyeblikk,
        FeedbackReason::ViktigPerson,
        FeedbackReason::FintBilde,
        FeedbackReason::ManglerHerfra,
        FeedbackReason::Skarpere,
        FeedbackReason::Gjenstand,
        FeedbackReason::UviktigDag,
        FeedbackReason::Stemning,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            FeedbackReason::Uskarpt => "uskarpt",
            FeedbackReason::DaarligLys => "daarlig_lys",
            FeedbackReason::ForLikt => "for_likt",
            FeedbackReason::ForMangeHerfra => "for_mange_herfra",
            FeedbackReason::LikerIkke => "liker_ikke",
            FeedbackReason::Privat => "privat",
            FeedbackReason::ViktigOyeblikk => "viktig_oyeblikk",
            FeedbackReason::ViktigPerson => "viktig_person",
            FeedbackReason::FintBilde => "fint_bilde",
            FeedbackReason::ManglerHerfra => "mangler_herfra",
            FeedbackReason::Skarpere => "skarpere",
            FeedbackReason::Gjenstand => "gjenstand",
            FeedbackReason::UviktigDag => "uviktig_dag",
            FeedbackReason::Stemning => "stemning",
        }
    }
}

impl FromStr for FeedbackReason {
    type Err = UnknownValue;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        FeedbackReason::ALL
            .into_iter()
            .find(|r| r.as_str() == s)
            .ok_or_else(|| UnknownValue(s.to_string()))
    }
}

/// Én handling i loggen.
#[derive(Debug, Clone, PartialEq)]
pub struct Feedback {
    pub action: Action,
    pub reason: Option<FeedbackReason>,
    /// Kvaliteten på bildet som ble tatt med (ved `TaMed` og `Bytt`), hvis kjent.
    pub added: Option<BasicQuality>,
    /// Kvaliteten på bildet som ble tatt bort (ved `TaBort` og `Bytt`), hvis kjent.
    pub removed: Option<BasicQuality>,
}

/// Lærte vekter. Startverdiene gir samme utkast som før brukeren har gjort noe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Preferences {
    /// Vekter i kvalitetsmålet (normaliseres til sum 1).
    pub w_sharp: f32,
    pub w_exposure: f32,
    pub w_color: f32,
    /// Hvor mye kvalitet teller mot å vise flere deler av en hendelse (0–1).
    pub quality_importance: f32,
    /// Ganges med antall bilder per hendelse.
    pub event_density: f32,
    /// Maks pHash-avstand for «nesten likt». Høyere = flere bilder regnes som like.
    pub similar_hamming: f32,
    /// Bilder uten personer må være blant de beste (1 − dette) for å komme med.
    pub mood_percentile: f32,
}

impl Default for Preferences {
    fn default() -> Self {
        Preferences {
            w_sharp: 0.5,
            w_exposure: 0.3,
            w_color: 0.2,
            quality_importance: 0.75,
            event_density: 1.0,
            similar_hamming: 10.0,
            mood_percentile: 0.9,
        }
    }
}

/// Det appen har lært, i en form grensesnittet kan si med ord.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lesson {
    SkarphetTellerMer,
    LysTellerMer,
    FargerTellerMer,
    OyeblikkFremforKvalitet,
    KvalitetFremforOyeblikk,
    FaerreLikeBilder,
    FlereFraHverHendelse,
    FaerreFraHverHendelse,
    TingBareNaarFlotte,
    LikerStemningsbilder,
}

/// Hvor stor endring som må til før den nevnes som noe appen har lært.
const LESSON_MIN: f32 = 0.05;

impl Preferences {
    /// Kvalitet 0–1 med de lærte vektene.
    pub fn quality(&self, q: &BasicQuality) -> f32 {
        let sum = self.w_sharp + self.w_exposure + self.w_color;
        (self.w_sharp * q.sharp + self.w_exposure * q.exposure + self.w_color * q.color) / sum
    }

    /// Endringene fra startverdiene som er store nok til å nevnes.
    pub fn lessons(&self) -> Vec<Lesson> {
        let d = Preferences::default();
        let mut out = Vec::new();
        if self.w_sharp - d.w_sharp >= LESSON_MIN {
            out.push(Lesson::SkarphetTellerMer);
        }
        if self.w_exposure - d.w_exposure >= LESSON_MIN {
            out.push(Lesson::LysTellerMer);
        }
        if self.w_color - d.w_color >= LESSON_MIN {
            out.push(Lesson::FargerTellerMer);
        }
        if d.quality_importance - self.quality_importance >= LESSON_MIN {
            out.push(Lesson::OyeblikkFremforKvalitet);
        }
        if self.quality_importance - d.quality_importance >= LESSON_MIN {
            out.push(Lesson::KvalitetFremforOyeblikk);
        }
        if self.similar_hamming - d.similar_hamming >= 2.0 {
            out.push(Lesson::FaerreLikeBilder);
        }
        if self.event_density - d.event_density >= LESSON_MIN {
            out.push(Lesson::FlereFraHverHendelse);
        }
        if d.event_density - self.event_density >= LESSON_MIN {
            out.push(Lesson::FaerreFraHverHendelse);
        }
        if self.mood_percentile - d.mood_percentile >= 0.04 {
            out.push(Lesson::TingBareNaarFlotte);
        }
        if d.mood_percentile - self.mood_percentile >= LESSON_MIN {
            out.push(Lesson::LikerStemningsbilder);
        }
        out
    }
}

/// Hvor mye skarphet må skille før et bytte uten svar tolkes som et signal.
const IMPLICIT_SHARP_GAP: f32 = 0.15;

/// Regner ut preferansene fra loggen, eldste først. Små steg og faste grenser, så ett
/// enkelt svar aldri snur utvalget på hodet.
pub fn learn(log: &[Feedback]) -> Preferences {
    let mut p = Preferences::default();
    for f in log {
        match f.reason {
            Some(FeedbackReason::Uskarpt) => {
                p.w_sharp += 0.04;
                p.quality_importance += 0.02;
            }
            Some(FeedbackReason::DaarligLys) => p.w_exposure += 0.04,
            Some(FeedbackReason::ForLikt) => p.similar_hamming += 1.0,
            Some(FeedbackReason::ForMangeHerfra) => p.event_density -= 0.04,
            Some(FeedbackReason::ManglerHerfra) => p.event_density += 0.04,
            Some(FeedbackReason::ViktigOyeblikk) => p.quality_importance -= 0.03,
            Some(FeedbackReason::ViktigPerson) => p.quality_importance -= 0.02,
            Some(FeedbackReason::FintBilde) => p.w_color += 0.03,
            Some(FeedbackReason::Skarpere) => p.w_sharp += 0.03,
            Some(FeedbackReason::Gjenstand) => p.mood_percentile += 0.02,
            Some(FeedbackReason::Stemning) => p.mood_percentile -= 0.03,
            // Huskes og vises igjen, men endrer ingen vekter.
            Some(
                FeedbackReason::LikerIkke | FeedbackReason::Privat | FeedbackReason::UviktigDag,
            ) => {}
            None => match f.action {
                // Uten svar: bare svake signaler fra selve handlingen.
                Action::TaMed => p.event_density += 0.01,
                Action::TaBort => p.event_density -= 0.01,
                Action::Fremhev | Action::Demp | Action::FjernDag => {}
                Action::Bytt => {
                    if let (Some(a), Some(r)) = (f.added, f.removed) {
                        if a.sharp + IMPLICIT_SHARP_GAP < r.sharp {
                            p.quality_importance -= 0.01;
                        } else if a.sharp > r.sharp + IMPLICIT_SHARP_GAP {
                            p.w_sharp += 0.01;
                        }
                    }
                }
            },
        }
        p.w_sharp = p.w_sharp.clamp(0.3, 0.8);
        p.w_exposure = p.w_exposure.clamp(0.15, 0.6);
        p.w_color = p.w_color.clamp(0.1, 0.5);
        p.quality_importance = p.quality_importance.clamp(0.4, 0.95);
        p.event_density = p.event_density.clamp(0.6, 1.6);
        p.similar_hamming = p.similar_hamming.clamp(6.0, 16.0);
        p.mood_percentile = p.mood_percentile.clamp(0.7, 0.99);
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fb(action: Action, reason: Option<FeedbackReason>) -> Feedback {
        Feedback {
            action,
            reason,
            added: None,
            removed: None,
        }
    }

    #[test]
    fn empty_log_gives_defaults_and_no_lessons() {
        assert_eq!(learn(&[]), Preferences::default());
        assert!(learn(&[]).lessons().is_empty());
    }

    #[test]
    fn reasons_move_the_right_weights() {
        let log = vec![fb(Action::TaBort, Some(FeedbackReason::Uskarpt)); 3];
        let p = learn(&log);
        assert!(p.w_sharp > Preferences::default().w_sharp);
        assert_eq!(
            p.lessons(),
            vec![Lesson::SkarphetTellerMer, Lesson::KvalitetFremforOyeblikk]
        );

        let log = vec![fb(Action::TaMed, Some(FeedbackReason::ViktigOyeblikk)); 2];
        assert_eq!(learn(&log).lessons(), vec![Lesson::OyeblikkFremforKvalitet]);
    }

    #[test]
    fn weights_are_bounded() {
        let log = vec![fb(Action::TaBort, Some(FeedbackReason::ForMangeHerfra)); 500];
        assert_eq!(learn(&log).event_density, 0.6);
    }

    #[test]
    fn dislike_changes_nothing() {
        let log = vec![fb(Action::TaBort, Some(FeedbackReason::LikerIkke)); 10];
        assert_eq!(learn(&log), Preferences::default());
    }

    #[test]
    fn implicit_swap_to_blurrier_photo_means_moments_matter() {
        let q = |sharp| BasicQuality {
            sharp,
            exposure: 0.8,
            color: 0.5,
            skin: 0.1,
        };
        let swap = Feedback {
            action: Action::Bytt,
            reason: None,
            added: Some(q(0.3)),
            removed: Some(q(0.8)),
        };
        let p = learn(&vec![swap; 5]);
        assert!(p.quality_importance < Preferences::default().quality_importance);
    }

    #[test]
    fn reason_codes_round_trip() {
        for r in FeedbackReason::ALL {
            assert_eq!(r.as_str().parse::<FeedbackReason>().unwrap(), r);
        }
        assert!("tull".parse::<FeedbackReason>().is_err());
    }
}
