use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
};
use std::time::Duration;

use tauri::Manager;
use tauri_specta::Event;

use crate::{models, storage};

use super::webview_power::{
    UNSET_POWER_POLICY, apply_webview_power_policy_if_changed, evaluate_webview_power_policy,
    is_window_minimized_or_hidden,
};

const WIN32_MINIMIZED_COORD_THRESHOLD: i32 = -30_000;
pub const WINDOW_BOUNDS_SAVE_DEBOUNCE_MS: u64 = 450;

#[derive(Clone)]
pub struct WindowRuntimeState {
    pub was_minimized: Arc<AtomicBool>,
    pub restoring_window: Arc<AtomicBool>,
    pub window_tracker: Arc<Mutex<storage::WindowSettings>>,
    pub window_save_seq: Arc<AtomicU64>,
    pub window_save_worker_active: Arc<AtomicBool>,
    pub last_power_policy: Arc<AtomicU8>,
}

impl Default for WindowRuntimeState {
    fn default() -> Self {
        Self {
            was_minimized: Arc::new(AtomicBool::new(false)),
            restoring_window: Arc::new(AtomicBool::new(false)),
            window_tracker: Arc::new(Mutex::new(storage::WindowSettings::default())),
            window_save_seq: Arc::new(AtomicU64::new(0)),
            window_save_worker_active: Arc::new(AtomicBool::new(false)),
            last_power_policy: Arc::new(AtomicU8::new(UNSET_POWER_POLICY)),
        }
    }
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

/// Returns true when `event` can affect minimized/visibility/power-policy state, avoiding Win32
/// queries on every frame of normal window dragging or resizing.
pub fn should_sync_minimized_state_for_event(
    event: &tauri::WindowEvent,
    was_minimized: bool,
) -> bool {
    match event {
        tauri::WindowEvent::Focused(_) => true,
        tauri::WindowEvent::Resized(size) => was_minimized || size.width == 0 || size.height == 0,
        tauri::WindowEvent::Moved(pos) => {
            was_minimized
                || pos.x <= WIN32_MINIMIZED_COORD_THRESHOLD
                || pos.y <= WIN32_MINIMIZED_COORD_THRESHOLD
        }
        _ => false,
    }
}

/// Returns true when `event` carries Win32 minimized sentinel geometry (`0×0` or `<= -30000`).
pub fn is_minimized_geometry_event(event: &tauri::WindowEvent) -> bool {
    match event {
        tauri::WindowEvent::Resized(size) => size.width == 0 || size.height == 0,
        tauri::WindowEvent::Moved(pos) => {
            pos.x <= WIN32_MINIMIZED_COORD_THRESHOLD || pos.y <= WIN32_MINIMIZED_COORD_THRESHOLD
        }
        _ => false,
    }
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

fn record_window_metrics(
    tracker: &Arc<Mutex<storage::WindowSettings>>,
    is_maximized: bool,
    monitor_name: Option<String>,
    scale: f64,
    inner_size: Option<(u32, u32)>,
    outer_pos: Option<(i32, i32)>,
) -> Option<storage::WindowSettings> {
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

macro_rules! capture_metrics_for_window {
    ($window:expr, $tracker:expr) => {{
        if $window.is_minimized().unwrap_or(false) || !$window.is_visible().unwrap_or(true) {
            None
        } else {
            let is_maximized = $window.is_maximized().unwrap_or(false);
            let current_monitor = $window.current_monitor().ok().flatten();
            let monitor_name = current_monitor.as_ref().and_then(|m| m.name().cloned());
            let scale = current_monitor
                .as_ref()
                .map(|m| m.scale_factor())
                .or_else(|| $window.scale_factor().ok())
                .unwrap_or(1.0);
            let inner_size = $window.inner_size().ok().map(|s| (s.width, s.height));
            let outer_pos = $window.outer_position().ok().map(|p| (p.x, p.y));
            record_window_metrics(
                $tracker,
                is_maximized,
                monitor_name,
                scale,
                inner_size,
                outer_pos,
            )
        }
    }};
}

fn capture_window_settings(
    window: &tauri::Window,
    tracker: &Arc<Mutex<storage::WindowSettings>>,
) -> Option<storage::WindowSettings> {
    capture_metrics_for_window!(window, tracker)
}

fn capture_webview_window_settings(
    window: &tauri::WebviewWindow,
    tracker: &Arc<Mutex<storage::WindowSettings>>,
) -> Option<storage::WindowSettings> {
    capture_metrics_for_window!(window, tracker)
}

async fn capture_main_window_settings_on_main_thread(
    app_handle: &tauri::AppHandle,
    tracker: &Arc<Mutex<storage::WindowSettings>>,
) -> Option<storage::WindowSettings> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let handle_for_main = app_handle.clone();
    let tracker_for_main = Arc::clone(tracker);
    if app_handle
        .run_on_main_thread(move || {
            let snapshot = handle_for_main
                .get_webview_window("main")
                .and_then(|win| capture_webview_window_settings(&win, &tracker_for_main));
            let _ = tx.send(snapshot);
        })
        .is_err()
    {
        return None;
    }
    rx.await.ok().flatten()
}

async fn run_debounced_window_save_loop(app_handle: tauri::AppHandle, state: WindowRuntimeState) {
    loop {
        let observed_seq = state.window_save_seq.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(WINDOW_BOUNDS_SAVE_DEBOUNCE_MS)).await;

        if state.window_save_seq.load(Ordering::SeqCst) != observed_seq {
            continue;
        }

        if !state.restoring_window.load(Ordering::SeqCst)
            && let Some(snapshot) =
                capture_main_window_settings_on_main_thread(&app_handle, &state.window_tracker)
                    .await
            && !state.restoring_window.load(Ordering::SeqCst)
            && state.window_save_seq.load(Ordering::SeqCst) == observed_seq
        {
            let _ = tokio::task::spawn_blocking(move || {
                storage::update_window_settings_if_changed(snapshot)
            })
            .await;
        }

        state
            .window_save_worker_active
            .store(false, Ordering::SeqCst);
        if state.window_save_seq.load(Ordering::SeqCst) != observed_seq
            && state
                .window_save_worker_active
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        {
            continue;
        }
        break;
    }
}

fn schedule_debounced_window_save(app_handle: &tauri::AppHandle, state: &WindowRuntimeState) {
    state.window_save_seq.fetch_add(1, Ordering::SeqCst);
    if state
        .window_save_worker_active
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        let app_handle = app_handle.clone();
        let state = state.clone();
        tauri::async_runtime::spawn(async move {
            run_debounced_window_save_loop(app_handle, state).await;
        });
    }
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

    if should_sync_minimized_state_for_event(event, state.was_minimized.load(Ordering::Relaxed)) {
        let is_minimized = window.is_minimized().unwrap_or(false);
        let is_visible = window.is_visible().unwrap_or(true);
        let is_focused = !matches!(event, tauri::WindowEvent::Focused(false));
        let minimized = is_window_minimized_or_hidden(is_minimized, is_visible);
        let prev = state.was_minimized.swap(minimized, Ordering::SeqCst);
        if prev != minimized {
            let _ = models::WindowMinimizedChanged(minimized).emit(window.app_handle());
        }
        if let Some(webview_win) = window.app_handle().get_webview_window("main") {
            let policy = evaluate_webview_power_policy(
                is_minimized,
                is_visible,
                is_focused,
                crate::ws_listener::is_in_game_phase_active(),
            );
            apply_webview_power_policy_if_changed(&webview_win, &state.last_power_policy, policy);
        }
    }

    match event {
        tauri::WindowEvent::Resized(_)
        | tauri::WindowEvent::Moved(_)
        | tauri::WindowEvent::ScaleFactorChanged { .. } => {
            if state.restoring_window.load(Ordering::Relaxed) || is_minimized_geometry_event(event)
            {
                return;
            }
            schedule_debounced_window_save(window.app_handle(), state);
        }
        tauri::WindowEvent::CloseRequested { .. } => {
            state.window_save_seq.fetch_add(1, Ordering::SeqCst);
            let snapshot = capture_window_settings(window, &state.window_tracker)
                .or_else(|| state.window_tracker.lock().ok().map(|g| g.clone()));
            if let Some(snapshot) = snapshot {
                let _ = storage::update_window_settings_if_changed(snapshot);
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
#[path = "window_events_tests.rs"]
mod tests;
