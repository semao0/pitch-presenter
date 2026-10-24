//! Tauri commands: the API the frontend calls via `invoke`.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::config::{self, RecentDeck};
use crate::error::{Error, Result};
use crate::render::Progress;
use crate::session::{Deck, EVENT_VIEW, Nav, Session, ViewState};
use crate::state::AppState;
use crate::windows;

/// Render widths, px. The smallest one covering the widest display is used,
/// so plugging in a slightly different monitor doesn't trigger a re-render.
const RENDER_WIDTHS: [u32; 3] = [1920, 2560, 3840];

/// What the start screen is pre-filled with.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchInfo {
    pub deck: Option<PathBuf>,
    pub notes: Option<PathBuf>,
    pub minutes: f64,
    pub recent: Vec<RecentDeck>,
}

/// Sets the language of backend messages (errors, notes warnings).
#[tauri::command]
pub fn set_language(english: bool) {
    crate::i18n::set_english(english);
}

/// Files from the command line win; otherwise the last opened deck.
#[tauri::command]
pub fn launch_info(state: State<'_, AppState>) -> LaunchInfo {
    let files = std::mem::take(&mut *state.launch_files.lock().expect("launch files lock"));
    let (deck, notes) = classify(&files);
    let config = state.config.lock().expect("config lock");
    let recent = config.existing_recent();

    let (deck, notes) = match deck {
        Some(deck) => {
            let notes = notes.or_else(|| config::guess_notes(&deck));
            (Some(deck), notes)
        }
        None => match recent.first() {
            Some(last) => (
                Some(last.deck.clone()),
                notes.or_else(|| last.notes.clone()),
            ),
            None => (None, notes),
        },
    };
    LaunchInfo {
        deck,
        notes,
        minutes: config.minutes,
        recent,
    }
}

/// Splits dropped or passed files into a deck and a notes file.
#[tauri::command]
pub fn classify_files(paths: Vec<PathBuf>) -> (Option<PathBuf>, Option<PathBuf>) {
    classify(&paths)
}

#[tauri::command]
pub fn guess_notes(deck: PathBuf) -> Option<PathBuf> {
    config::guess_notes(&deck)
}

/// Renders the deck, loads notes and starts the session.
#[tauri::command]
pub async fn open_deck(
    app: AppHandle,
    deck: PathBuf,
    notes: Option<PathBuf>,
    minutes: f64,
    on_progress: Channel<Progress>,
) -> Result<Deck> {
    if !deck.is_file() {
        return Err(Error::io(&deck, std::io::ErrorKind::NotFound.into()));
    }
    if let Some(notes) = notes.as_deref().filter(|n| !n.is_file()) {
        return Err(Error::io(notes, std::io::ErrorKind::NotFound.into()));
    }
    let minutes = if minutes.is_finite() {
        minutes.max(0.0)
    } else {
        0.0
    };
    let width = windows::widest_monitor_px(&app)
        .and_then(|px| RENDER_WIDTHS.into_iter().find(|&w| w >= px))
        .unwrap_or(RENDER_WIDTHS[0]);

    let rendered = {
        let (app, deck) = (app.clone(), deck.clone());
        tauri::async_runtime::spawn_blocking(move || {
            let state = app.state::<AppState>();
            let report = |p: Progress| {
                let _ = on_progress.send(p);
            };
            state.renderer.render(&deck, width, &report)
        })
        .await
        .map_err(Error::Tauri)??
    };

    let session = Session::open(&app, &deck, rendered, notes.clone(), minutes)?;
    let payload = session.deck.clone();
    let view = session.view();

    let state = app.state::<AppState>();
    state.replace_session(Some(session));
    {
        let mut config = state.config.lock().expect("config lock");
        config.remember(&deck, notes.as_deref(), minutes);
        config.save(&state.config_path);
    }
    app.emit(EVENT_VIEW, view)?;
    Ok(payload)
}

/// Ends the session and returns to the start screen.
#[tauri::command]
pub fn close_deck(app: AppHandle, state: State<'_, AppState>) {
    app.state::<crate::speech::SpeechState>().stop();
    windows::close_audience(&app);
    state.replace_session(None);
}

/// The open deck, e.g. after the presenter page was reloaded.
#[tauri::command]
pub fn current_deck(state: State<'_, AppState>) -> Option<Deck> {
    state.with_session(|s| s.deck.clone())
}

#[tauri::command]
pub fn current_view(state: State<'_, AppState>) -> Option<ViewState> {
    state.with_session(|s| s.view())
}

/// Moves the show and broadcasts the result to every window.
#[tauri::command]
pub fn navigate(app: AppHandle, state: State<'_, AppState>, nav: Nav) -> Result<ViewState> {
    let view = state
        .with_session(|s| s.navigate(nav))
        .ok_or(Error::NoSession)?;
    app.emit(EVENT_VIEW, &view)?;
    Ok(view)
}

#[tauri::command]
pub fn open_audience(app: AppHandle) -> Result<()> {
    windows::open_audience(&app)
}

#[tauri::command]
pub fn audience_open(app: AppHandle) -> bool {
    windows::audience_window(&app).is_some()
}

fn classify(paths: &[PathBuf]) -> (Option<PathBuf>, Option<PathBuf>) {
    let is_notes = |p: &Path| {
        p.extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("toml"))
    };
    let notes = paths.iter().find(|p| is_notes(p)).cloned();
    let deck = paths.iter().find(|p| !is_notes(p)).cloned();
    (deck, notes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_by_extension() {
        let files = vec![PathBuf::from("/a/notes.TOML"), PathBuf::from("/a/talk.pdf")];
        assert_eq!(
            classify(&files),
            (
                Some(PathBuf::from("/a/talk.pdf")),
                Some(PathBuf::from("/a/notes.TOML"))
            )
        );
        assert_eq!(classify(&[]), (None, None));
    }
}
