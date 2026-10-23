//! Pitch Presenter: a presenter view for PDF and PPTX decks.
//!
//! The backend renders slides, parses speaker notes and owns the show state;
//! the Svelte frontend (`../src`) draws the presenter and audience windows.
//!
//! * [`render`]   — PDF/PPTX → cached PNG slides.
//! * [`notes`]    — the TOML speaker-notes format.
//! * [`session`]  — the open deck, navigation, notes hot-reload.
//! * [`windows`]  — the audience window and display placement.
//! * [`commands`] — the `invoke` API.

mod commands;
mod config;
mod error;
mod notes;
mod render;
mod session;
mod state;
mod windows;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::http::{Request, Response, StatusCode, header};
use tauri::{AppHandle, Emitter, Manager, RunEvent, UriSchemeResponder, WindowEvent};

use config::Config;
use render::{Renderer, Size};
use state::AppState;

/// Custom protocol serving rendered slides: `slide://localhost/<deck>/<n>` and
/// `slide://localhost/<deck>/thumb/<n>`.
const SLIDE_SCHEME: &str = "slide";
/// Emitted to the main window with file paths opened from the OS while running.
const EVENT_OPEN_FILES: &str = "open-files";

/// Application entry point.
///
/// # Panics
/// If the Tauri runtime cannot start (e.g. no display).
pub fn run() {
    let app = tauri::Builder::default()
        // Must be the first plugin: a second launch forwards its files and exits.
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            let files = file_args(argv.iter().skip(1), Path::new(&cwd));
            open_files(app, files);
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .level_for("tao", log::LevelFilter::Warn)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .register_asynchronous_uri_scheme_protocol(SLIDE_SCHEME, |ctx, request, responder| {
            serve_slide(ctx.app_handle(), &request, responder);
        })
        .setup(|app| {
            let state = init_state(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the presenter quits the app, audience window included.
            if window.label() == windows::MAIN
                && matches!(event, WindowEvent::CloseRequested { .. })
            {
                windows::close_audience(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::launch_info,
            commands::classify_files,
            commands::guess_notes,
            commands::open_deck,
            commands::close_deck,
            commands::current_deck,
            commands::current_view,
            commands::navigate,
            commands::open_audience,
            commands::audience_open,
        ])
        .build(tauri::generate_context!())
        .expect("error while building the application");

    app.run(|app, event| {
        // macOS delivers "Open With…" and Dock drops as an event, not argv.
        #[cfg(target_os = "macos")]
        if let RunEvent::Opened { urls } = &event {
            let files = urls.iter().filter_map(|u| u.to_file_path().ok()).collect();
            open_files(app, files);
        }
        if let RunEvent::Exit = event {
            app.state::<AppState>().replace_session(None);
        }
    });
}

fn init_state(app: &AppHandle) -> Result<AppState, Box<dyn std::error::Error>> {
    let paths = app.path();
    let config_path = paths.app_config_dir()?.join("config.json");
    let cache_root = paths.app_cache_dir()?.join("slides");

    let mut pdfium_dirs = Vec::new();
    if let Some(dir) = std::env::var_os("PITCH_PRESENTER_PDFIUM") {
        pdfium_dirs.push(PathBuf::from(dir));
    }
    if let Ok(dir) = paths.resource_dir() {
        pdfium_dirs.push(dir.join("pdfium"));
    }
    if cfg!(debug_assertions) {
        pdfium_dirs.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("pdfium"));
    }

    let cwd = std::env::current_dir().unwrap_or_default();
    Ok(AppState {
        renderer: Renderer::new(cache_root, pdfium_dirs),
        session: Mutex::default(),
        config: Mutex::new(Config::load(&config_path)),
        config_path,
        launch_files: Mutex::new(file_args(std::env::args().skip(1), &cwd)),
    })
}

/// Command-line arguments that name existing files, made absolute.
fn file_args(args: impl IntoIterator<Item = impl AsRef<str>>, cwd: &Path) -> Vec<PathBuf> {
    args.into_iter()
        .map(|a| PathBuf::from(a.as_ref()))
        .filter(|a| !a.to_string_lossy().starts_with('-'))
        .map(|a| cwd.join(a))
        .filter(|p| p.is_file())
        .collect()
}

/// Hands files opened from the OS to the start screen and raises the window.
fn open_files(app: &AppHandle, files: Vec<PathBuf>) {
    if let Some(main) = app.get_webview_window(windows::MAIN) {
        let _ = main.unminimize();
        let _ = main.set_focus();
    }
    if !files.is_empty() {
        let _ = app.emit_to(windows::MAIN, EVENT_OPEN_FILES, files);
    }
}

fn serve_slide(app: &AppHandle, request: &Request<Vec<u8>>, responder: UriSchemeResponder) {
    let path = request.uri().path().to_owned();
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let file = parse_slide_path(&path)
            .and_then(|(id, n, size)| app.state::<AppState>().renderer.slide_path(id, n, size));
        let response = match file.map(std::fs::read) {
            Some(Ok(bytes)) => Response::builder()
                .header(header::CONTENT_TYPE, "image/png")
                // URLs include the deck's content hash, so they never go stale.
                .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(bytes),
            _ => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Vec::new()),
        };
        responder.respond(response.expect("valid response"));
    });
}

/// `/<id>/<n>` or `/<id>/thumb/<n>`.
fn parse_slide_path(path: &str) -> Option<(&str, u32, Size)> {
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    match parts.as_slice() {
        [id, n] => Some((id, n.parse().ok()?, Size::Full)),
        [id, "thumb", n] => Some((id, n.parse().ok()?, Size::Thumb)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slide_urls() {
        assert_eq!(parse_slide_path("/abc/3"), Some(("abc", 3, Size::Full)));
        assert_eq!(
            parse_slide_path("/abc/thumb/12"),
            Some(("abc", 12, Size::Thumb))
        );
        assert_eq!(parse_slide_path("/abc/x"), None);
        assert_eq!(parse_slide_path("/abc/thumb/1/2"), None);
    }
}
