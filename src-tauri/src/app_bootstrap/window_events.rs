use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

use tauri::Manager;
use tauri_specta::Event;

use crate::{models, storage};

const WIN32_MINIMIZED_COORD_THRESHOLD: i32 = -30_000;

#[derive(Clone, Default)]
pub struct WindowRuntimeState {
    pub was_paused: Arc<AtomicBool>,
    pub restoring_window: Arc<AtomicBool>,
    pub window_tracker: Arc<Mutex<storage::WindowSettings>>,
    pub window_save_seq: Arc<AtomicU64>,
}

/// Returns true when the main window should pause UI animations (minimized, hidden, or unfocused).
pub fn should_pause_window_animations(
    is_minimized: bool,
    is_visible: bool,
    is_focused: bool,
) -> bool {
    is_minimized || !is_visible || !is_focused
}

/// Returns true when closing the main window should terminate the application process.
pub fn should_allow_exit_on_window_close(close_to_tray: bool) -> bool {
    !close_to_tray
}

/// Flashes the Windows taskbar icon continuously until the user clicks/focuses the window.
pub fn request_taskbar_attention(app_handle: &tauri::AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.request_user_attention(Some(tauri::UserAttentionType::Critical));
    }
}

/// Shows/focuses (or lazily recreates) the `"main"` window on second-instance launch or tray open.
pub fn focus_main_window(app_handle: &tauri::AppHandle) {
    let _ = super::window_factory::show_or_create_main_window(app_handle);
}

fn apply_captured_window_metrics(
    target: &mut storage::WindowSettings,
    is_maximized: bool,
    monitor_name: Option<String>,
    scale_factor: f64,
    inner_size: Option<(u32, u32)>,
    outer_pos: Option<(i32, i32)>,
) -> bool {
    if let Some((w, h)) = inner_size
        && (w == 0 || h == 0)
    {
        return false;
    }

    if !is_maximized
        && let Some((x, y)) = outer_pos
        && (x <= WIN32_MINIMIZED_COORD_THRESHOLD || y <= WIN32_MINIMIZED_COORD_THRESHOLD)
    {
        return false;
    }

    target.window_maximized = is_maximized;
    if let Some(name) = monitor_name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        target.window_monitor = Some(name.to_string());
    }

    // Only update logical size & physical position when not maximized so restoring un-maximizes to the normal rect.
    if !is_maximized {
        let scale = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor
        } else {
            1.0
        };
        if let Some((w, h)) = inner_size {
            let logical_w = f64::from(w) / scale;
            let logical_h = f64::from(h) / scale;
            if logical_w >= storage::MIN_WINDOW_WIDTH && logical_h >= storage::MIN_WINDOW_HEIGHT {
                target.window_width = logical_w;
                target.window_height = logical_h;
            }
        }
        if let Some((x, y)) = outer_pos {
            target.window_x = Some(f64::from(x));
            target.window_y = Some(f64::from(y));
        }
    }

    true
}

fn capture_window_settings(
    window: &tauri::Window,
    tracker: &Arc<Mutex<storage::WindowSettings>>,
) -> Option<storage::WindowSettings> {
    if window.is_minimized().unwrap_or(false) || !window.is_visible().unwrap_or(true) {
        return None;
    }

    let is_maximized = window.is_maximized().unwrap_or(false);
    let current_monitor = window.current_monitor().ok().flatten();
    let monitor_name = current_monitor.as_ref().and_then(|m| m.name().cloned());
    let scale = current_monitor
        .as_ref()
        .map(|m| m.scale_factor())
        .or_else(|| window.scale_factor().ok())
        .unwrap_or(1.0);
    let inner_size = window.inner_size().ok().map(|s| (s.width, s.height));
    let outer_pos = window.outer_position().ok().map(|p| (p.x, p.y));

    let mut guard = tracker.lock().ok()?;
    if !apply_captured_window_metrics(
        &mut guard,
        is_maximized,
        monitor_name,
        scale,
        inner_size,
        outer_pos,
    ) {
        return None;
    }

    Some(guard.clone())
}

/// Handles window focus, pause-animation tracking, debounced bounds persistence, and close-to-tray lifecycle.
pub fn handle_window_event(
    window: &tauri::Window,
    event: &tauri::WindowEvent,
    state: &WindowRuntimeState,
) {
    if matches!(event, tauri::WindowEvent::Focused(true)) {
        let _ = window.request_user_attention(None);
    }

    if window.label() != "main" {
        return;
    }

    if matches!(
        event,
        tauri::WindowEvent::Resized(_)
            | tauri::WindowEvent::Moved(_)
            | tauri::WindowEvent::Focused(_)
    ) {
        let is_minimized = window.is_minimized().unwrap_or(false);
        let is_visible = window.is_visible().unwrap_or(true);
        let is_focused = match event {
            tauri::WindowEvent::Focused(focused) => *focused,
            _ => window.is_focused().unwrap_or(true),
        };
        let paused = should_pause_window_animations(is_minimized, is_visible, is_focused);
        let prev = state.was_paused.swap(paused, Ordering::SeqCst);
        if prev != paused {
            let _ = models::WindowMinimizedChanged(paused).emit(window.app_handle());
        }
    }

    match event {
        tauri::WindowEvent::Resized(_)
        | tauri::WindowEvent::Moved(_)
        | tauri::WindowEvent::ScaleFactorChanged { .. } => {
            if state.restoring_window.load(Ordering::SeqCst) {
                return;
            }
            if let Some(snapshot) = capture_window_settings(window, &state.window_tracker) {
                let seq = state.window_save_seq.fetch_add(1, Ordering::SeqCst) + 1;
                let seq_ref = Arc::clone(&state.window_save_seq);
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(450)).await;
                    if seq_ref.load(Ordering::SeqCst) == seq {
                        let _ = storage::update_global_settings(|s| {
                            s.window = snapshot;
                        });
                    }
                });
            }
        }
        tauri::WindowEvent::CloseRequested { .. } => {
            state.window_save_seq.fetch_add(1, Ordering::SeqCst);
            let snapshot = capture_window_settings(window, &state.window_tracker)
                .or_else(|| state.window_tracker.lock().ok().map(|g| g.clone()));
            if let Some(snapshot) = snapshot {
                let _ = storage::update_global_settings(|s| {
                    s.window = snapshot;
                });
            }
            let close_to_tray = storage::get_global_settings().general.close_to_tray;
            if should_allow_exit_on_window_close(close_to_tray) {
                super::window_factory::allow_app_exit(window.app_handle());
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_pause_window_animations_when_minimized_hidden_or_unfocused() {
        assert!(!should_pause_window_animations(false, true, true));
        assert!(should_pause_window_animations(true, true, true));
        assert!(should_pause_window_animations(false, false, true));
        assert!(should_pause_window_animations(false, true, false));
        assert!(should_pause_window_animations(true, false, false));
    }

    #[test]
    fn test_should_allow_exit_on_window_close_respects_close_to_tray() {
        assert!(should_allow_exit_on_window_close(false));
        assert!(!should_allow_exit_on_window_close(true));
    }

    #[test]
    fn test_apply_captured_window_metrics_tracks_monitor_dpi_and_preserves_normal_rect_when_maximized()
     {
        let mut settings = storage::WindowSettings::default();

        // Normal capture on 200% DPI secondary monitor
        assert!(apply_captured_window_metrics(
            &mut settings,
            false,
            Some(r"\\.\DISPLAY2".to_string()),
            2.0,
            Some((2560, 1440)),
            Some((2200, 150)),
        ));
        assert_eq!(settings.window_width, 1280.0);
        assert_eq!(settings.window_height, 720.0);
        assert_eq!(settings.window_x, Some(2200.0));
        assert_eq!(settings.window_y, Some(150.0));
        assert!(!settings.window_maximized);
        assert_eq!(settings.window_monitor.as_deref(), Some(r"\\.\DISPLAY2"));

        // Maximizing on DISPLAY3 keeps normal rect intact while updating maximized + monitor
        assert!(apply_captured_window_metrics(
            &mut settings,
            true,
            Some(r"\\.\DISPLAY3".to_string()),
            1.5,
            Some((2560, 1400)),
            Some((0, 0)),
        ));
        assert_eq!(settings.window_width, 1280.0);
        assert_eq!(settings.window_height, 720.0);
        assert_eq!(settings.window_x, Some(2200.0));
        assert_eq!(settings.window_y, Some(150.0));
        assert!(settings.window_maximized);
        assert_eq!(settings.window_monitor.as_deref(), Some(r"\\.\DISPLAY3"));

        // Win32 minimized sentinel (-32000, -32000) and zero-size events are rejected
        assert!(!apply_captured_window_metrics(
            &mut settings,
            false,
            Some(r"\\.\DISPLAY1".to_string()),
            1.0,
            Some((1280, 720)),
            Some((-32000, -32000)),
        ));
        assert!(!apply_captured_window_metrics(
            &mut settings,
            false,
            Some(r"\\.\DISPLAY1".to_string()),
            1.0,
            Some((0, 0)),
            Some((100, 100)),
        ));
        assert_eq!(settings.window_monitor.as_deref(), Some(r"\\.\DISPLAY3"));
    }
}
