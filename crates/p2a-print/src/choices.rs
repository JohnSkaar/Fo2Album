//! Brukerens valg for årets album utover enkeltbilder: sider hun er fornøyd med, turer uten
//! barn, dager slått sammen, sidetall per historie, forside og bakside, og rotering.
//! Lagres kryptert i familieprofilen (én JSON per år) og brukes både av utkastet og
//! trykkfilen, så det brukeren finpusser, er det som trykkes.
//!
//! Bilder og hendelser er nøklet med bildets hash (heks), så valgene overlever at
//! utkastet lages på nytt. En hendelse er nøklet med et hvilket som helst bilde i den.

use std::collections::{BTreeMap, HashSet};

use p2a_core::layout::PageKind;
use p2a_core::select::draft::{DraftHints, LockedPage};
use p2a_core::ContentHash;
use p2a_store::{Store, StoreError};
use serde::{Deserialize, Serialize};

/// En side brukeren er fornøyd med.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedPage {
    /// helside | luft | rutenett
    pub kind: String,
    #[serde(default)]
    pub kolonner: Option<u8>,
    pub photos: Vec<String>,
}

impl SavedPage {
    pub fn page_kind(&self) -> PageKind {
        match self.kind.as_str() {
            "helside" => PageKind::Helside,
            "luft" => PageKind::Luft,
            _ => PageKind::Rutenett {
                kolonner: self.kolonner.unwrap_or(3),
            },
        }
    }

    pub fn from_kind(kind: PageKind, photos: Vec<String>) -> Self {
        let (kind, kolonner) = match kind {
            PageKind::Helside => ("helside", None),
            PageKind::Luft => ("luft", None),
            PageKind::Rutenett { kolonner } => ("rutenett", Some(kolonner)),
        };
        SavedPage {
            kind: kind.into(),
            kolonner,
            photos,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AlbumChoices {
    pub locked_pages: Vec<SavedPage>,
    pub adult_trips: Vec<String>,
    pub family_trips: Vec<String>,
    pub merged: Vec<Vec<String>>,
    /// «Presenter denne historien på x sider»: (et bilde i historien, sider).
    pub page_targets: Vec<(String, usize)>,
    pub cover: Option<String>,
    pub back: Option<String>,
    /// Rotering med klokka i grader (90, 180, 270), per bilde.
    pub rotation: BTreeMap<String, u16>,
}

/// Én endring fra grensesnittet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Change {
    /// «Fornøyd med siden»: beholdes som den er i neste utkast.
    Fornoyd {
        page: SavedPage,
        on: bool,
    },
    /// Tur uten barn (`true`) eller vanlig familietur (`false`). Nøkkel: et bilde i turen.
    TurUtenBarn {
        photo: String,
        on: bool,
    },
    /// Slå sammen dagene som disse bildene hører til, til én historie.
    SlaaSammen {
        photos: Vec<String>,
    },
    /// Del opp igjen en sammenslått historie (nøkkel: et bilde i den).
    DelOpp {
        photo: String,
    },
    /// Presenter historien på så mange sider (`None` = appens forslag).
    Sider {
        photo: String,
        pages: Option<usize>,
    },
    Forside {
        photo: Option<String>,
    },
    Bakside {
        photo: Option<String>,
    },
    /// Roter bildet en kvart omdreining med klokka.
    Roter {
        photo: String,
    },
}

fn key(year: i32) -> String {
    format!("albumvalg:{year}")
}

pub fn load(store: &Store, year: i32) -> Result<AlbumChoices, StoreError> {
    Ok(store
        .setting(&key(year))?
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default())
}

pub fn save(store: &Store, year: i32, c: &AlbumChoices) -> Result<(), StoreError> {
    let json = serde_json::to_string(c).expect("valgene kan alltid gjøres om til JSON");
    store.set_setting(&key(year), &json)
}

impl AlbumChoices {
    /// Bruker en endring. En side som endres, er ikke lenger «fornøyd» (brukeren får lage
    /// den på nytt), og et bilde finnes i høyst én fornøyd side.
    pub fn apply(&mut self, change: Change) {
        match change {
            Change::Fornoyd { page, on } => {
                let ids: HashSet<&String> = page.photos.iter().collect();
                self.locked_pages
                    .retain(|p| !p.photos.iter().any(|id| ids.contains(id)));
                if on {
                    self.locked_pages.push(page);
                }
            }
            Change::TurUtenBarn { photo, on } => {
                self.adult_trips.retain(|p| p != &photo);
                self.family_trips.retain(|p| p != &photo);
                if on {
                    self.adult_trips.push(photo);
                } else {
                    self.family_trips.push(photo);
                }
            }
            Change::SlaaSammen { photos } => {
                let ids: HashSet<&String> = photos.iter().collect();
                self.merged.retain(|m| !m.iter().any(|id| ids.contains(id)));
                self.merged.push(photos);
            }
            Change::DelOpp { photo } => self.merged.retain(|m| !m.contains(&photo)),
            Change::Sider { photo, pages } => {
                self.page_targets.retain(|(p, _)| p != &photo);
                if let Some(n) = pages {
                    self.page_targets.push((photo, n.max(1)));
                }
            }
            Change::Forside { photo } => {
                if photo.is_some() && photo == self.back {
                    self.back = None;
                }
                self.cover = photo;
            }
            Change::Bakside { photo } => {
                if photo.is_some() && photo == self.cover {
                    self.cover = None;
                }
                self.back = photo;
            }
            Change::Roter { photo } => {
                let r = (self.rotation.get(&photo).copied().unwrap_or(0) + 90) % 360;
                if r == 0 {
                    self.rotation.remove(&photo);
                } else {
                    self.rotation.insert(photo, r);
                }
            }
        }
    }

    pub fn rotation_of(&self, h: &ContentHash) -> u16 {
        self.rotation.get(&h.to_hex()).copied().unwrap_or(0)
    }

    /// Valgene som hint til utkastet.
    pub fn hints(&self, base: DraftHints) -> DraftHints {
        let h = |s: &String| ContentHash::from_hex(s);
        DraftHints {
            locked_pages: self
                .locked_pages
                .iter()
                .map(|p| LockedPage {
                    kind: p.page_kind(),
                    photos: p.photos.iter().filter_map(h).collect(),
                })
                .filter(|p| !p.photos.is_empty())
                .collect(),
            adult_trips: self.adult_trips.iter().filter_map(h).collect(),
            family_trips: self.family_trips.iter().filter_map(h).collect(),
            merged_events: self
                .merged
                .iter()
                .map(|m| m.iter().filter_map(h).collect())
                .collect(),
            page_targets: self
                .page_targets
                .iter()
                .filter_map(|(p, n)| Some((h(p)?, *n)))
                .collect(),
            ..base
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: u8) -> String {
        ContentHash([b; 32]).to_hex()
    }

    #[test]
    fn fornoyd_erstatter_sider_med_samme_bilder() {
        let mut c = AlbumChoices::default();
        let page = |ids: &[u8]| {
            SavedPage::from_kind(
                PageKind::Rutenett { kolonner: 2 },
                ids.iter().map(|&b| hex(b)).collect(),
            )
        };
        c.apply(Change::Fornoyd {
            page: page(&[1, 2]),
            on: true,
        });
        c.apply(Change::Fornoyd {
            page: page(&[2, 3]),
            on: true,
        });
        assert_eq!(c.locked_pages, vec![page(&[2, 3])]);
        c.apply(Change::Fornoyd {
            page: page(&[2, 3]),
            on: false,
        });
        assert!(c.locked_pages.is_empty());
    }

    #[test]
    fn turtype_og_forside_bytter_og_rotering_gaar_rundt() {
        let mut c = AlbumChoices::default();
        c.apply(Change::TurUtenBarn {
            photo: hex(1),
            on: true,
        });
        c.apply(Change::TurUtenBarn {
            photo: hex(1),
            on: false,
        });
        assert!(c.adult_trips.is_empty());
        assert_eq!(c.family_trips, vec![hex(1)]);

        c.apply(Change::Forside {
            photo: Some(hex(5)),
        });
        c.apply(Change::Bakside {
            photo: Some(hex(5)),
        });
        assert_eq!((c.cover.clone(), c.back.clone()), (None, Some(hex(5))));

        for want in [90, 180, 270, 0] {
            c.apply(Change::Roter { photo: hex(7) });
            assert_eq!(c.rotation_of(&ContentHash([7; 32])), want);
        }
    }

    #[test]
    fn hint_til_utkastet() {
        let mut c = AlbumChoices::default();
        c.apply(Change::SlaaSammen {
            photos: vec![hex(1), hex(9)],
        });
        c.apply(Change::Sider {
            photo: hex(1),
            pages: Some(3),
        });
        c.apply(Change::TurUtenBarn {
            photo: hex(4),
            on: true,
        });
        let h = c.hints(DraftHints::default());
        assert_eq!(h.merged_events.len(), 1);
        assert_eq!(h.page_targets.get(&ContentHash([1; 32])), Some(&3));
        assert!(h.adult_trips.contains(&ContentHash([4; 32])));
        // Ny sammenslåing med et av de samme bildene erstatter den gamle.
        c.apply(Change::SlaaSammen {
            photos: vec![hex(9), hex(12)],
        });
        assert_eq!(c.merged, vec![vec![hex(9), hex(12)]]);
        c.apply(Change::DelOpp { photo: hex(12) });
        assert!(c.merged.is_empty());
    }

    #[test]
    fn tolererer_gamle_eller_ufullstendige_data() {
        let c: AlbumChoices = serde_json::from_str(r#"{"cover":"abc"}"#).unwrap();
        assert_eq!(c.cover.as_deref(), Some("abc"));
        assert!(c.hints(DraftHints::default()).locked_pages.is_empty());
    }
}
