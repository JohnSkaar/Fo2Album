//! Brukerens valg for årets album utover enkeltbilder: sider brukeren er fornøyd med (med
//! rammer), turer uten barn, dager slått sammen, egne historier, sidetall per historie,
//! forside og bakside, og per bilde rotering, størrelse og utsnitt.
//! Lagres kryptert i familieprofilen (én JSON per år) og brukes både av utkastet og
//! trykkfilen, så det brukeren finpusser, er det som trykkes.
//!
//! Bilder og hendelser er nøklet med bildets hash (heks), så valgene overlever at
//! utkastet lages på nytt. En hendelse er nøklet med et hvilket som helst bilde i den.

use std::collections::{BTreeMap, BTreeSet, HashSet};

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
    /// Rammene brukeren har valgt for siden (`layout::TEMPLATES`); `None` = automatisk.
    #[serde(default)]
    pub mal: Option<String>,
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
            mal: None,
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
    /// Størrelse per bilde (se `DraftHints::sizes`): -1 til 4.
    pub size: BTreeMap<String, i8>,
    /// Utsnitt i en ramme som fylles: punktet (prosent av bredde og høyde) som skal være i
    /// midten av rammen.
    pub focus: BTreeMap<String, (u8, u8)>,
    /// Bilder som skal vises hele i rammen sin (ingen beskjæring).
    pub whole: BTreeSet<String>,
    /// «Egen historie»: bildene i hver historie.
    pub stories: Vec<Vec<String>>,
}

/// Høyst så mange bilder på én side når bilder samles.
pub const GATHER_MAX: usize = 12;

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
    /// Størrelsen på bildet (-1 til 4, se `DraftHints::sizes`); `None` = appen velger.
    Storrelse {
        photo: String,
        size: Option<i8>,
    },
    /// Utsnitt: midtpunktet i prosent, eller hele bildet i rammen.
    Utsnitt {
        photo: String,
        focus: Option<(u8, u8)>,
        whole: bool,
    },
    /// «Sett på én side»: bildene samles på en ny side (flere ved over 12), merket fornøyd.
    SamleSide {
        photos: Vec<String>,
    },
    /// «Egen historie»: bildene blir en egen historie med egne sider.
    EgenHistorie {
        photos: Vec<String>,
    },
    /// Legg bildene i historien tilbake der de hørte til (nøkkel: et bilde i den).
    LeggTilbake {
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

/// Kolonner i rutenettet for en samlet side med `n` bilder (som `p2a_core::layout`).
fn columns(n: usize) -> u8 {
    match n {
        0..=2 => 2,
        3 => 3,
        4 => 2,
        5..=9 => 3,
        _ => 4,
    }
}

impl AlbumChoices {
    /// Fornøyd-sider som inneholder et av bildene, er ikke lenger fornøyd.
    fn unlock(&mut self, photos: &[String]) {
        self.locked_pages
            .retain(|p| !p.photos.iter().any(|id| photos.contains(id)));
    }

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
            Change::Storrelse { photo, size } => {
                let old = self.size.get(&photo).copied();
                let size = size.map(|s| s.clamp(-1, 4));
                // Egen side eller ikke: siden bildet står på, lages på nytt.
                let alone = |s: Option<i8>| s.is_some_and(|s| s >= 3);
                if alone(old) != alone(size) {
                    self.unlock(std::slice::from_ref(&photo));
                }
                match size {
                    Some(s) => self.size.insert(photo, s),
                    None => self.size.remove(&photo),
                };
            }
            Change::Utsnitt {
                photo,
                focus,
                whole,
            } => {
                match focus {
                    Some((x, y)) => self.focus.insert(photo.clone(), (x.min(100), y.min(100))),
                    None => self.focus.remove(&photo),
                };
                if whole {
                    self.whole.insert(photo);
                } else {
                    self.whole.remove(&photo);
                }
            }
            Change::SamleSide { photos } => {
                if photos.is_empty() {
                    return;
                }
                self.unlock(&photos);
                for chunk in photos.chunks(GATHER_MAX) {
                    let kind = if chunk.len() == 1 {
                        PageKind::Helside
                    } else {
                        PageKind::Rutenett {
                            kolonner: columns(chunk.len()),
                        }
                    };
                    self.locked_pages
                        .push(SavedPage::from_kind(kind, chunk.to_vec()));
                }
            }
            Change::EgenHistorie { photos } => {
                if photos.is_empty() {
                    return;
                }
                let ids: HashSet<&String> = photos.iter().collect();
                for st in &mut self.stories {
                    st.retain(|id| !ids.contains(id));
                }
                self.stories.retain(|st| !st.is_empty());
                for m in &mut self.merged {
                    m.retain(|id| !ids.contains(id));
                }
                self.merged.retain(|m| m.len() > 1);
                self.unlock(&photos);
                self.stories.push(photos);
            }
            Change::LeggTilbake { photo } => {
                if let Some(k) = self.stories.iter().position(|st| st.contains(&photo)) {
                    let st = self.stories.remove(k);
                    self.unlock(&st);
                }
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

    /// Hvordan bildet skal stå på siden: størrelse og utsnitt.
    pub fn look_of(&self, h: &ContentHash) -> crate::layout::Look {
        let id = h.to_hex();
        crate::layout::Look {
            size: self.size.get(&id).copied().unwrap_or(0),
            focus: self
                .focus
                .get(&id)
                .map_or((0.5, 0.5), |&(x, y)| (x as f32 / 100.0, y as f32 / 100.0)),
            whole: self.whole.contains(&id),
        }
    }

    /// Rammene brukeren har valgt for en fornøyd side med akkurat disse bildene.
    pub fn template_of(&self, photos: &[ContentHash]) -> Option<&str> {
        let ids: HashSet<String> = photos.iter().map(ContentHash::to_hex).collect();
        self.locked_pages
            .iter()
            .find(|p| p.photos.len() == ids.len() && p.photos.iter().all(|id| ids.contains(id)))
            .and_then(|p| p.mal.as_deref())
    }

    /// Bildet hører til en historie brukeren har laget.
    pub fn in_story(&self, id: &str) -> bool {
        self.stories.iter().any(|st| st.iter().any(|x| x == id))
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
            sizes: self
                .size
                .iter()
                .filter_map(|(p, s)| Some((h(p)?, *s)))
                .collect(),
            own_stories: self
                .stories
                .iter()
                .map(|st| st.iter().filter_map(h).collect())
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
    fn storrelse_samle_og_egen_historie() {
        let mut c = AlbumChoices::default();
        c.apply(Change::Fornoyd {
            page: SavedPage::from_kind(PageKind::Rutenett { kolonner: 2 }, vec![hex(1), hex(2)]),
            on: true,
        });
        // Større på siden: siden er fortsatt fornøyd.
        c.apply(Change::Storrelse {
            photo: hex(1),
            size: Some(2),
        });
        assert_eq!(c.locked_pages.len(), 1);
        // Egen side: siden lages på nytt.
        c.apply(Change::Storrelse {
            photo: hex(1),
            size: Some(9),
        });
        assert!(c.locked_pages.is_empty());
        assert_eq!(c.size.get(&hex(1)), Some(&4));
        c.apply(Change::Storrelse {
            photo: hex(1),
            size: None,
        });
        assert!(c.size.is_empty());

        let many: Vec<String> = (10..24).map(hex).collect();
        c.apply(Change::SamleSide {
            photos: many.clone(),
        });
        assert_eq!(c.locked_pages.len(), 2);
        assert_eq!(c.locked_pages[0].photos.len(), GATHER_MAX);
        assert_eq!(c.locked_pages[1].kolonner, Some(2));

        c.apply(Change::SlaaSammen {
            photos: vec![hex(30), hex(10), hex(31)],
        });
        c.apply(Change::EgenHistorie {
            photos: vec![hex(10), hex(11)],
        });
        assert_eq!(c.stories.len(), 1);
        assert_eq!(c.merged, vec![vec![hex(30), hex(31)]]);
        assert_eq!(
            c.locked_pages.len(),
            1,
            "siden med bildene er ikke lenger fornøyd"
        );
        let h = c.hints(DraftHints::default());
        assert_eq!(h.own_stories[0].len(), 2);
        c.apply(Change::LeggTilbake { photo: hex(11) });
        assert!(c.stories.is_empty());
    }

    #[test]
    fn utsnitt_og_rammer() {
        let mut c = AlbumChoices::default();
        c.apply(Change::Utsnitt {
            photo: hex(1),
            focus: Some((20, 150)),
            whole: false,
        });
        let look = c.look_of(&ContentHash([1; 32]));
        assert_eq!(look.focus, (0.2, 1.0));
        assert!(!look.whole);
        c.apply(Change::Utsnitt {
            photo: hex(1),
            focus: None,
            whole: true,
        });
        assert!(c.look_of(&ContentHash([1; 32])).whole);

        let mut page = SavedPage::from_kind(PageKind::Luft, vec![hex(2), hex(3)]);
        page.mal = Some("2-side".into());
        c.apply(Change::Fornoyd { page, on: true });
        let hs = [ContentHash([3; 32]), ContentHash([2; 32])];
        assert_eq!(c.template_of(&hs), Some("2-side"));
        assert_eq!(c.template_of(&hs[..1]), None);
    }

    #[test]
    fn tolererer_gamle_eller_ufullstendige_data() {
        let c: AlbumChoices = serde_json::from_str(r#"{"cover":"abc"}"#).unwrap();
        assert_eq!(c.cover.as_deref(), Some("abc"));
        assert!(c.hints(DraftHints::default()).locked_pages.is_empty());
    }
}
