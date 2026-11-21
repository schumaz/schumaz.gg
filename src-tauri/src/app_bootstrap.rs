pub mod prewarm;
pub mod tray;
pub mod window_events;
pub mod window_factory;

pub use tray::refresh_tray_locale;
pub use window_events::{
    WindowRuntimeState, focus_main_window, handle_window_event, request_taskbar_attention,
    should_pause_window_animations,
};
pub use window_factory::{
    AUTOSTART_LAUNCH_ARG, AppLifecycleState, allow_app_exit, is_autostart_launch_args,
    should_allow_exit, show_or_create_main_window,
};

use std::sync::{Arc, Mutex};

use tauri::Manager;
use tauri_specta::Event;

use crate::{build_provider, lcu_connector, lcu_injector, logging, models, storage, ws_listener};

#[derive(Clone, Default)]
pub struct BootstrapState {
    inner: Arc<Mutex<models::BootstrapStatus>>,
}

impl BootstrapState {
    pub fn snapshot(&self) -> models::BootstrapStatus {
        self.inner
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn update(
        &self,
        app_handle: &tauri::AppHandle,
        stage: &str,
        progress: u8,
        disk_ready: bool,
        network_ready: bool,
    ) {
        let next = models::BootstrapStatus {
            stage: stage.to_string(),
            progress: progress.min(100),
            disk_ready,
            network_ready,
        };
        if let Ok(mut guard) = self.inner.lock() {
            *guard = next.clone();
        }
        let _ = models::BootstrapProgress(next).emit(app_handle);
    }
}

/// Handles asynchronous `schumaz-asset://` custom URI requests backed by disk asset cache.
pub fn handle_schumaz_asset_protocol(uri_path: String, responder: tauri::UriSchemeResponder) {
    tauri::async_runtime::spawn(async move {
        let err_resp = |status: u16| {
            tauri::http::Response::builder()
                .status(status)
                .body(Vec::new())
                .unwrap_or_else(|_| tauri::http::Response::new(Vec::new()))
        };

        let Some(target_url) = storage::decode_asset_request_segment(&uri_path) else {
            responder.respond(err_resp(400));
            return;
        };
        let Some(paths) = storage::global_paths() else {
            responder.respond(err_resp(503));
            return;
        };

        let client = build_provider::shared_http_client();
        match storage::serve_cached_or_fetch_asset(paths, client, &target_url).await {
            Ok((bytes, mime)) => {
                let resp = tauri::http::Response::builder()
                    .status(200)
                    .header("Content-Type", mime)
                    .header("Cache-Control", "public, max-age=31536000, immutable")
                    .body(bytes)
                    .unwrap_or_else(|_| err_resp(500));
                responder.respond(resp);
            }
            Err(status) => responder.respond(err_resp(status)),
        }
    });
}

/// Initializes local storage, loads settings, sets up the reactive system tray, and launches
/// either the main window (interactive startup) or headless tray mode (`--autostart`).
pub fn setup_app(
    app: &mut tauri::App,
    window_state: &WindowRuntimeState,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle().clone();
    let bootstrap_state = BootstrapState::default();
    app.manage(bootstrap_state.clone());
    app.manage(AppLifecycleState::default());
    app.manage(window_state.clone());

    // 1. Resolve %LOCALAPPDATA%\schumazgg\ and ensure Config/, Cache/, Logs/, & WebView/ subfolders
    let paths = storage::AppPaths::resolve_from_app(&app_handle).map_err(std::io::Error::other)?;
    paths.ensure_dirs().map_err(std::io::Error::other)?;
    let _ = logging::init_logging(&paths.logs_dir);
    let ini_path = paths.user_settings_ini_path();
    storage::set_global_paths(paths);

    // 2. Load UserSettings.ini BEFORE creating tray, window, or spawning background tasks
    let settings = storage::load_user_settings(&ini_path);
    storage::init_global_settings(ini_path, settings.clone());

    let active_locale = settings.general.language.unwrap_or_default();
    lcu_injector::set_active_locale(active_locale);

    if let Ok(mut guard) = window_state.window_tracker.lock() {
        *guard = settings.window.clone();
    }

    app.manage(storage::GraphicsState::new(
        settings.graphics.hardware_acceleration,
    ));

    bootstrap_state.update(&app_handle, "settings_loaded", 20, false, false);

    // 3. Setup native reactive system tray icon (always active)
    tray::setup_system_tray(app, active_locale)?;

    // 4. Lazy WebView2 lifecycle: defer window creation and heavy UI prewarm on `--autostart`
    let started_in_tray = is_autostart_launch_args(std::env::args());
    let boot_mode = if started_in_tray {
        "autostart"
    } else {
        "interactive"
    };
    logging::log_info(
        "app.bootstrap",
        format!(
            "event=startup version={} mode={} locale={} gpu_active={} close_to_tray={}",
            app_handle.package_info().version,
            boot_mode,
            active_locale.as_str(),
            settings.graphics.hardware_acceleration,
            settings.general.close_to_tray
        ),
    );
    if started_in_tray {
        prewarm::spawn_headless_lcu_ddragon_prewarm(app_handle.clone());
    } else {
        show_or_create_main_window(&app_handle).map_err(std::io::Error::other)?;
    }

    // Background task 1: Monitor LCU process liveness
    let lcu_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        lcu_connector::start_lcu_monitor(lcu_handle).await;
    });

    // Background task 2: Listen for Champion Select events via WebSocket
    let ws_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        ws_listener::start_ws_listener(ws_handle).await;
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_state_defaults_and_snapshot() {
        let state = BootstrapState::default();
        let snap = state.snapshot();
        assert_eq!(snap.stage, "initializing");
        assert_eq!(snap.progress, 0);
        assert!(!snap.disk_ready);
        assert!(!snap.network_ready);
    }
}
