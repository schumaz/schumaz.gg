use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tauri::Manager;

use crate::storage;

use super::prewarm;
use super::window_events::WindowRuntimeState;

pub const AUTOSTART_LAUNCH_ARG: &str = "--autostart";

/// Tracks process-wide exit permission and idempotent prewarm scheduling across lazy WebView lifecycles.
#[derive(Clone, Default)]
pub struct AppLifecycleState {
    allow_exit: Arc<AtomicBool>,
    prewarm_started: Arc<AtomicBool>,
    ddragon_warmed: Arc<AtomicBool>,
}

impl AppLifecycleState {
    pub fn allow_exit(&self) {
        self.allow_exit.store(true, Ordering::SeqCst);
    }

    pub fn reset_allow_exit(&self) {
        self.allow_exit.store(false, Ordering::SeqCst);
    }

    pub fn should_allow_exit(&self) -> bool {
        self.allow_exit.load(Ordering::SeqCst)
    }

    pub fn claim_full_prewarm(&self) -> bool {
        !self.prewarm_started.swap(true, Ordering::SeqCst)
    }

    pub fn is_full_prewarm_started(&self) -> bool {
        self.prewarm_started.load(Ordering::SeqCst)
    }

    pub fn claim_ddragon_prewarm(&self) -> bool {
        !self.ddragon_warmed.swap(true, Ordering::SeqCst)
    }
}

/// Returns true when the process was launched silently via OS autostart (`--autostart`).
pub fn is_autostart_launch_args<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .skip(1)
        .any(|arg| arg.as_ref().eq_ignore_ascii_case(AUTOSTART_LAUNCH_ARG))
}

/// Marks the process as intentionally exiting so `RunEvent::ExitRequested` will not prevent shutdown.
pub fn allow_app_exit(app_handle: &tauri::AppHandle) {
    if let Some(lifecycle) = app_handle.try_state::<AppLifecycleState>() {
        lifecycle.allow_exit();
    }
}

/// Returns true when `RunEvent::ExitRequested` should allow the backend process to terminate.
pub fn should_allow_exit(app_handle: &tauri::AppHandle) -> bool {
    app_handle
        .try_state::<AppLifecycleState>()
        .is_none_or(|lifecycle| lifecycle.should_allow_exit())
}

fn collect_monitor_rects(app_handle: &tauri::AppHandle) -> Vec<storage::MonitorRect> {
    let primary_key = app_handle.primary_monitor().ok().flatten().map(|m| {
        (
            m.name().cloned(),
            m.position().x,
            m.position().y,
            m.size().width,
            m.size().height,
        )
    });

    app_handle
        .available_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|m| {
            let work = m.work_area();
            let is_primary = primary_key.as_ref().is_some_and(|pk| {
                pk.0.as_ref() == m.name()
                    && pk.1 == m.position().x
                    && pk.2 == m.position().y
                    && pk.3 == m.size().width
                    && pk.4 == m.size().height
            });
            storage::MonitorRect {
                name: m.name().cloned(),
                x: f64::from(work.position.x),
                y: f64::from(work.position.y),
                width: f64::from(work.size.width),
                height: f64::from(work.size.height),
                scale_factor: m.scale_factor(),
                is_primary,
            }
        })
        .collect()
}

fn apply_validated_window_bounds(
    window: &tauri::WebviewWindow,
    bounds: &storage::ValidatedWindowBounds,
) {
    let target_phys_size = tauri::PhysicalSize::new(
        (bounds.width * bounds.scale_factor).round().max(1.0) as u32,
        (bounds.height * bounds.scale_factor).round().max(1.0) as u32,
    );

    if let (Some(x), Some(y)) = (bounds.x, bounds.y) {
        let target_pos = tauri::PhysicalPosition::new(x, y);
        // Move to the target monitor first so Windows assigns its DPI, then apply physical size,
        // and re-apply position in case WM_DPICHANGED shifted the origin.
        let _ = window.set_position(tauri::Position::Physical(target_pos));
        let _ = window.set_size(tauri::Size::Physical(target_phys_size));
        let _ = window.set_position(tauri::Position::Physical(target_pos));
    } else {
        let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
            bounds.width,
            bounds.height,
        )));
        if bounds.center {
            let _ = window.center();
        }
    }

    if bounds.maximized {
        let _ = window.maximize();
    }
    let _ = window.show();
}

/// Shows and focuses the `"main"` WebView2 window, lazily creating it on demand if deferred or closed to tray.
pub fn show_or_create_main_window(app_handle: &tauri::AppHandle) -> Result<(), String> {
    if let Some(lifecycle) = app_handle.try_state::<AppLifecycleState>() {
        lifecycle.reset_allow_exit();
    }
    prewarm::ensure_full_prewarm_started(app_handle);

    if let Some(window) = app_handle.get_webview_window("main") {
        if window.is_minimized().unwrap_or(false) {
            let _ = window.unminimize();
        }
        let _ = window.show();
        let _ = window.set_focus();
        if let Some(window_state) = app_handle.try_state::<WindowRuntimeState>() {
            if window_state.was_minimized.swap(false, Ordering::SeqCst) {
                let _ = <crate::models::WindowMinimizedChanged as tauri_specta::Event>::emit(
                    &crate::models::WindowMinimizedChanged(false),
                    app_handle,
                );
            }
            let policy = super::webview_power::evaluate_webview_power_policy(
                false,
                true,
                true,
                crate::ws_listener::is_in_game_phase_active(),
            );
            super::webview_power::apply_webview_power_policy_if_changed(
                &window,
                &window_state.last_power_policy,
                policy,
            );
        }
        return Ok(());
    }

    let paths = storage::global_paths()
        .cloned()
        .ok_or_else(|| "Storage paths are not initialized".to_string())?;
    let settings = storage::get_global_settings();
    let hw_accel = settings.graphics.hardware_acceleration;
    let window_state = app_handle
        .try_state::<WindowRuntimeState>()
        .map(|s| s.inner().clone())
        .unwrap_or_default();

    let monitors = collect_monitor_rects(app_handle);
    let bounds = storage::validate_window_bounds(&settings.window, &monitors);

    let mut window_config = app_handle
        .config()
        .app
        .windows
        .first()
        .cloned()
        .ok_or_else(|| "Main window configuration missing".to_string())?;

    window_state.restoring_window.store(true, Ordering::SeqCst);
    window_state.was_minimized.store(false, Ordering::SeqCst);
    window_state.last_power_policy.store(
        super::webview_power::UNSET_POWER_POLICY,
        Ordering::SeqCst,
    );

    window_config.width = bounds.width;
    window_config.height = bounds.height;
    window_config.x = None;
    window_config.y = None;
    window_config.center = bounds.x.is_none() || bounds.y.is_none();
    window_config.maximized = false;
    window_config.visible = false;
    window_config.additional_browser_args =
        Some(storage::browser_args_for_hardware_acceleration(hw_accel));

    let build_result = tauri::WebviewWindowBuilder::from_config(app_handle, &window_config)
        .and_then(|builder| builder.data_directory(paths.webview_dir).build());

    match build_result {
        Ok(window) => {
            if let Some(graphics) = app_handle.try_state::<storage::GraphicsState>() {
                graphics.mark_webview_created(hw_accel);
            }
            apply_validated_window_bounds(&window, &bounds);
            if let Ok(mut guard) = window_state.window_tracker.lock() {
                guard.window_width = bounds.width;
                guard.window_height = bounds.height;
                if let (Some(x), Some(y)) = (bounds.x, bounds.y) {
                    guard.window_x = Some(f64::from(x));
                    guard.window_y = Some(f64::from(y));
                }
                guard.window_maximized = bounds.maximized;
                if bounds.monitor_name.is_some() {
                    guard.window_monitor = bounds.monitor_name.clone();
                }
            }
            window_state.restoring_window.store(false, Ordering::SeqCst);
            let _ = window.set_focus();
            let policy = super::webview_power::evaluate_webview_power_policy(
                false,
                true,
                true,
                crate::ws_listener::is_in_game_phase_active(),
            );
            super::webview_power::apply_webview_power_policy_if_changed(
                &window,
                &window_state.last_power_policy,
                policy,
            );
            Ok(())
        }
        Err(err) => {
            window_state.restoring_window.store(false, Ordering::SeqCst);
            Err(err.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_autostart_launch_args_detects_flag_ignoring_binary_path() {
        assert!(!is_autostart_launch_args(["schumazgg.exe"]));
        assert!(!is_autostart_launch_args(["--autostart"]));
        assert!(is_autostart_launch_args(["schumazgg.exe", "--autostart"]));
        assert!(is_autostart_launch_args([
            "schumazgg.exe",
            "--foo",
            "--AUTOSTART"
        ]));
    }

    #[test]
    fn test_app_lifecycle_state_exit_and_idempotent_prewarm_flags() {
        let state = AppLifecycleState::default();
        assert!(!state.should_allow_exit());
        state.allow_exit();
        assert!(state.should_allow_exit());
        state.reset_allow_exit();
        assert!(!state.should_allow_exit());

        assert!(state.claim_ddragon_prewarm());
        assert!(!state.claim_ddragon_prewarm());

        assert!(!state.is_full_prewarm_started());
        assert!(state.claim_full_prewarm());
        assert!(state.is_full_prewarm_started());
        assert!(!state.claim_full_prewarm());
    }
}
