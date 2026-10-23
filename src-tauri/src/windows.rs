//! The audience window: slides only, for the projector or screen sharing.

use tauri::utils::config::BackgroundThrottlingPolicy;
use tauri::webview::Color;
use tauri::{
    AppHandle, Emitter, Manager, Monitor, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

use crate::error::Result;

pub const MAIN: &str = "main";
pub const AUDIENCE: &str = "audience";
/// Emitted to the main window with `true`/`false` when the audience window opens or closes.
pub const EVENT_AUDIENCE: &str = "audience";

pub fn audience_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(AUDIENCE)
}

/// Opens the audience window, or brings it to front if it is already open.
///
/// With a second display connected the window goes fullscreen there and focus
/// returns to the presenter, which is what one wants when plugging in a
/// projector. On a single display it opens as a regular window to be shared.
pub fn open_audience(app: &AppHandle) -> Result<()> {
    if let Some(window) = audience_window(app) {
        window.show()?;
        window.unminimize()?;
        window.set_focus()?;
        return Ok(());
    }

    let builder = WebviewWindowBuilder::new(app, AUDIENCE, WebviewUrl::App("index.html".into()))
        .title("Показ")
        .inner_size(1280.0, 720.0)
        .min_inner_size(320.0, 180.0)
        .background_color(Color(0, 0, 0, 255))
        .visible(false)
        // The audience window is often covered by the presenter or a video
        // call; macOS would otherwise throttle it and delay slide changes.
        .background_throttling(BackgroundThrottlingPolicy::Disabled);
    let window = builder.build()?;

    let external = external_monitor(app);
    if let Some(monitor) = &external {
        window.set_position(*monitor.position())?;
        window.set_fullscreen(true)?;
    }
    window.show()?;
    if external.is_some() {
        if let Some(main) = app.get_webview_window(MAIN) {
            main.set_focus()?;
        }
    } else {
        window.set_focus()?;
    }

    let handle = app.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            let _ = handle.emit_to(MAIN, EVENT_AUDIENCE, false);
        }
    });
    app.emit_to(MAIN, EVENT_AUDIENCE, true)?;
    Ok(())
}

pub fn close_audience(app: &AppHandle) {
    if let Some(window) = audience_window(app) {
        let _ = window.destroy();
    }
}

/// A monitor other than the one showing the presenter window.
fn external_monitor(app: &AppHandle) -> Option<Monitor> {
    let main = app.get_webview_window(MAIN)?;
    let current = main.current_monitor().ok()??;
    main.available_monitors()
        .ok()?
        .into_iter()
        .find(|m| m.position() != current.position() || m.size() != current.size())
}

/// Widest connected display, used to pick the slide render resolution.
pub fn widest_monitor_px(app: &AppHandle) -> Option<u32> {
    let main = app.get_webview_window(MAIN)?;
    main.available_monitors()
        .ok()?
        .iter()
        .map(|m| m.size().width)
        .max()
}
