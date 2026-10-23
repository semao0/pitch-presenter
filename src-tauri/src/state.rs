//! Shared application state, managed by Tauri.

use std::path::PathBuf;
use std::sync::Mutex;

use crate::config::Config;
use crate::render::Renderer;
use crate::session::Session;

pub struct AppState {
    pub renderer: Renderer,
    pub session: Mutex<Option<Session>>,
    pub config: Mutex<Config>,
    pub config_path: PathBuf,
    /// Files passed on the command line, consumed by the start screen.
    pub launch_files: Mutex<Vec<PathBuf>>,
}

impl AppState {
    /// Replaces the session. The old one is dropped *outside* the lock: its
    /// file watcher joins a thread that may itself be waiting for this lock.
    pub fn replace_session(&self, session: Option<Session>) {
        let old = std::mem::replace(&mut *self.session.lock().expect("session lock"), session);
        drop(old);
    }

    /// Runs `f` on the open session, if any.
    pub fn with_session<T>(&self, f: impl FnOnce(&mut Session) -> T) -> Option<T> {
        self.session.lock().expect("session lock").as_mut().map(f)
    }
}
