//! «Angre»: de siste endringene i utkastet, i minnet så lenge appen er åpen. Albumvalg
//! (sider, rammer, størrelse …) og valg per bilde (ta med, ta bort, bytt) angres likt.

use std::sync::Mutex;

use p2a_core::select::draft::Decision;
use p2a_core::ContentHash;
use p2a_print::choices::{self, AlbumChoices};
use p2a_store::{Store, StoreError};
use tauri::State;

use crate::commands::{AppState, CmdResult};

/// Så mange steg huskes.
const MAX_STEPS: usize = 100;

pub enum Step {
    /// Albumvalgene slik de var før endringen.
    Choices {
        year: i32,
        before: Box<AlbumChoices>,
    },
    /// Valgene for bildene slik de var, og loggføringene som skal fjernes.
    Photos {
        year: i32,
        before: Vec<(ContentHash, Option<Decision>)>,
        feedback: Vec<i64>,
    },
}

impl Step {
    fn year(&self) -> i32 {
        match self {
            Step::Choices { year, .. } | Step::Photos { year, .. } => *year,
        }
    }

    fn restore(self, store: &Store) -> Result<(), StoreError> {
        match self {
            Step::Choices { year, before } => choices::save(store, year, &before),
            Step::Photos {
                year,
                before,
                feedback,
            } => {
                for (h, d) in before {
                    store.set_decision(year, &h, d)?;
                }
                for id in feedback {
                    store.delete_feedback(id)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Default)]
pub struct History(Mutex<Vec<Step>>);

impl History {
    pub fn push(&self, step: Step) {
        let mut v = self.0.lock().unwrap_or_else(|e| e.into_inner());
        v.push(step);
        if v.len() > MAX_STEPS {
            v.remove(0);
        }
    }

    /// Tar ut siste steg for året.
    fn pop(&self, year: i32) -> Option<Step> {
        let mut v = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let k = v.iter().rposition(|s| s.year() == year)?;
        Some(v.remove(k))
    }

    fn count(&self, year: i32) -> usize {
        let v = self.0.lock().unwrap_or_else(|e| e.into_inner());
        v.iter().filter(|s| s.year() == year).count()
    }
}

/// Angrer siste endring i årets utkast. Returnerer hvor mange steg som kan angres etterpå,
/// eller `None` når det ikke var noe å angre.
#[tauri::command]
pub fn undo_last(state: State<'_, AppState>, year: i32) -> CmdResult<Option<usize>> {
    let guard = state.store()?;
    let store = guard.as_ref().expect("sjekket");
    let Some(step) = state.undo.pop(year) else {
        return Ok(None);
    };
    step.restore(store)?;
    Ok(Some(state.undo.count(year)))
}

/// Hvor mange endringer i årets utkast som kan angres.
#[tauri::command]
pub fn undo_count(state: State<'_, AppState>, year: i32) -> usize {
    state.undo.count(year)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angrer_siste_steg_for_aaret() {
        let h = History::default();
        for year in [2011, 2012, 2011] {
            h.push(Step::Choices {
                year,
                before: Box::default(),
            });
        }
        assert_eq!(h.count(2011), 2);
        assert!(h.pop(2011).is_some());
        assert!(h.pop(2011).is_some());
        assert!(h.pop(2011).is_none());
        assert_eq!(h.count(2012), 1);
        for _ in 0..MAX_STEPS + 5 {
            h.push(Step::Photos {
                year: 2013,
                before: Vec::new(),
                feedback: Vec::new(),
            });
        }
        assert_eq!(h.count(2013) + h.count(2012), MAX_STEPS);
    }
}
