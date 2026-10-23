//! The open presentation: rendered deck, speaker notes and what is on screen.
//!
//! The backend owns the "show state" (current slide, blank screen). Both
//! windows send navigation requests here and render whatever the `view` event
//! says, so they can never drift apart.

use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::i18n::tr;
use crate::notes::{self, Notes};
use crate::render::Rendered;
use crate::state::AppState;

/// Emitted to all windows when the slide or blank state changes.
pub const EVENT_VIEW: &str = "view";
/// Emitted when the notes file changed on disk and was re-read.
pub const EVENT_DECK: &str = "deck-updated";

/// Everything the presenter window needs to know about the open deck.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Deck {
    pub id: String,
    pub title: String,
    pub count: u32,
    pub aspect: f32,
    /// Talk length; 0 means "no limit".
    pub minutes: f64,
    pub notes: Notes,
    /// File name of the notes, if any.
    pub notes_file: Option<String>,
    /// Set when the notes file is broken; the last good notes stay on screen.
    pub notes_error: Option<String>,
}

/// What the audience sees. Sent with every change.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewState {
    pub deck_id: String,
    pub count: u32,
    pub slide: u32,
    pub blank: bool,
}

/// A navigation request from either window.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Nav {
    Next,
    Prev,
    First,
    Last,
    Goto { slide: u32 },
    SetBlank { blank: bool },
}

type Watcher = Debouncer<notify::RecommendedWatcher, RecommendedCache>;

pub struct Session {
    pub deck: Deck,
    slide: u32,
    blank: bool,
    notes_path: Option<PathBuf>,
    /// Title and length used when the notes file doesn't set them.
    fallback_title: String,
    fallback_minutes: f64,
    /// Dropping the session stops watching the notes file.
    _watcher: Option<Watcher>,
}

impl Session {
    /// Creates a session; fails if the notes file can't be parsed, so a typo
    /// is reported on the start screen instead of an empty presenter view.
    pub fn open(
        app: &AppHandle,
        deck_path: &Path,
        rendered: Rendered,
        notes_path: Option<PathBuf>,
        minutes: f64,
    ) -> crate::error::Result<Self> {
        let notes = match &notes_path {
            Some(path) => notes::load(path, rendered.count)?,
            None => Notes::default(),
        };
        let fallback_title = deck_path.file_stem().map_or_else(
            || tr!("Презентация", "Presentation"),
            |s| s.to_string_lossy().into_owned(),
        );
        let watcher = notes_path.as_deref().and_then(|p| watch(app, p));
        let mut session = Self {
            deck: Deck {
                id: rendered.id,
                title: String::new(),
                count: rendered.count,
                aspect: rendered.aspect,
                minutes: 0.0,
                notes: Notes::default(),
                notes_file: notes_path
                    .as_deref()
                    .and_then(Path::file_name)
                    .map(|n| n.to_string_lossy().into_owned()),
                notes_error: None,
            },
            slide: 1,
            blank: false,
            notes_path,
            fallback_title,
            fallback_minutes: minutes,
            _watcher: watcher,
        };
        session.set_notes(notes);
        Ok(session)
    }

    fn set_notes(&mut self, notes: Notes) {
        self.deck.title = notes
            .title
            .clone()
            .unwrap_or_else(|| self.fallback_title.clone());
        self.deck.minutes = notes.minutes.unwrap_or(self.fallback_minutes);
        self.deck.notes = notes;
        self.deck.notes_error = None;
    }

    /// Re-reads the notes file. On error the previous notes are kept.
    pub fn reload_notes(&mut self) {
        let Some(path) = &self.notes_path else { return };
        match notes::load(path, self.deck.count) {
            Ok(notes) => self.set_notes(notes),
            Err(e) => self.deck.notes_error = Some(e.to_string()),
        }
    }

    pub fn view(&self) -> ViewState {
        ViewState {
            deck_id: self.deck.id.clone(),
            count: self.deck.count,
            slide: self.slide,
            blank: self.blank,
        }
    }

    pub fn navigate(&mut self, nav: Nav) -> ViewState {
        let last = self.deck.count;
        match nav {
            Nav::Next => self.slide = (self.slide + 1).min(last),
            Nav::Prev => self.slide = self.slide.saturating_sub(1).max(1),
            Nav::First => self.slide = 1,
            Nav::Last => self.slide = last,
            Nav::Goto { slide } => self.slide = slide.clamp(1, last),
            Nav::SetBlank { blank } => self.blank = blank,
        }
        self.view()
    }
}

/// Watches the notes file's directory: editors usually save by writing a
/// temporary file and renaming it over the original, which a watch on the file
/// itself would miss.
fn watch(app: &AppHandle, path: &Path) -> Option<Watcher> {
    let dir = path.parent()?.to_path_buf();
    let file_name = path.file_name()?.to_os_string();
    let app = app.clone();

    let handler = move |result: DebounceEventResult| {
        let Ok(events) = result else { return };
        let touched = events
            .iter()
            .flat_map(|e| &e.paths)
            .any(|p| p.file_name() == Some(&file_name));
        if !touched {
            return;
        }
        let state = app.state::<AppState>();
        let deck = {
            let mut session = state.session.lock().expect("session lock");
            let Some(session) = session.as_mut() else {
                return;
            };
            session.reload_notes();
            session.deck.clone()
        };
        log::info!(
            "notes reloaded{}",
            if deck.notes_error.is_some() {
                " with errors"
            } else {
                ""
            }
        );
        let _ = app.emit(EVENT_DECK, deck);
    };

    let mut debouncer = new_debouncer(Duration::from_millis(150), None, handler)
        .inspect_err(|e| log::warn!("cannot watch notes: {e}"))
        .ok()?;
    debouncer
        .watch(&dir, RecursiveMode::NonRecursive)
        .inspect_err(|e| log::warn!("cannot watch {}: {e}", dir.display()))
        .ok()?;
    Some(debouncer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(count: u32) -> Session {
        Session {
            deck: Deck {
                id: "0".repeat(16),
                title: String::new(),
                count,
                aspect: 1.0,
                minutes: 0.0,
                notes: Notes::default(),
                notes_file: None,
                notes_error: None,
            },
            slide: 1,
            blank: false,
            notes_path: None,
            fallback_title: String::new(),
            fallback_minutes: 0.0,
            _watcher: None,
        }
    }

    #[test]
    fn navigation_is_clamped() {
        let mut s = session(3);
        assert_eq!(s.navigate(Nav::Prev).slide, 1);
        assert_eq!(s.navigate(Nav::Next).slide, 2);
        assert_eq!(s.navigate(Nav::Last).slide, 3);
        assert_eq!(s.navigate(Nav::Next).slide, 3);
        assert_eq!(s.navigate(Nav::Goto { slide: 0 }).slide, 1);
        assert_eq!(s.navigate(Nav::Goto { slide: 99 }).slide, 3);
        assert!(s.navigate(Nav::SetBlank { blank: true }).blank);
        assert_eq!(s.navigate(Nav::First).slide, 1);
    }

    #[test]
    fn nav_wire_format() {
        let nav: Nav = serde_json::from_str(r#"{"type":"goto","slide":4}"#).unwrap();
        assert!(matches!(nav, Nav::Goto { slide: 4 }));
        let nav: Nav = serde_json::from_str(r#"{"type":"setBlank","blank":true}"#).unwrap();
        assert!(matches!(nav, Nav::SetBlank { blank: true }));
    }
}
