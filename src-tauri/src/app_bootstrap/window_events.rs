use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

use tauri::Manager;
use tauri_specta::Event;

use crate::{models, storage};

#[derive(Clone, Default)]
pub struct WindowRuntimeState {
    pub was_paused: Arc<AtomicBool>,
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

/// Flashes the Windows taskbar icon continuously until the user clicks/focuses the window.
pub fn request_taskbar_attention(app_handle: &tauri::AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.request_user_attention(Some(tauri::UserAttentionType::Critical));
    }
}

fn capture_window_settings(
    window: &tauri::Window,
    tracker: &Arc<Mutex<storage::WindowSettings>>,
) -> Option<storage::WindowSettings> {
    if window.is_minimized().unwrap_or(false) {
        return None;
    }

    let is_maximized = window.is_maximized().unwrap_or(false);
    let mut guard = tracker.lock().ok()?;
    guard.window_maximized = is_maximized;

    // Only update logical size & position when not maximized so restoring un-maximizes to the normal rect.
    if !is_maximized {
        let scale = window.scale_factor().unwrap_or(1.0);
        if let Ok(size) = window.inner_size() {
            let logical = size.to_logical::<f64>(scale);
            if logical.width >= storage::MIN_WINDOW_WIDTH
                && logical.height >= storage::MIN_WINDOW_HEIGHT
            {
                guard.window_width = logical.width;
                guard.window_height = logical.height;
            }
        }
        if let Ok(pos) = window.outer_position() {
            let logical = pos.to_logical::<f64>(scale);
            if logical.x.is_finite() && logical.y.is_finite() {
                guard.window_x = Some(logical.x);
                guard.window_y = Some(logical.y);
            }
        }
    }

    Some(guard.clone())
}

/// Handles window focus, pause-animation tracking, and debounced bounds persistence.
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
        tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_) => {
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
            if let Some(snapshot) = capture_window_settings(window, &state.window_tracker) {
                let _ = storage::update_global_settings(|s| {
                    s.window = snapshot;
                });
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
}
