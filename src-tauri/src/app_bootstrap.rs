use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

use tauri::Manager;
use tauri_specta::Event;

use crate::{
    build_provider, lcu_connector, lcu_injector, models, profile_provider, storage, ws_listener,
};

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

fn spawn_prewarm_task(
    app_handle: tauri::AppHandle,
    bootstrap_state: BootstrapState,
    paths: storage::AppPaths,
    settings: storage::AppSettings,
    active_locale: models::AppLocale,
) {
    tauri::async_runtime::spawn(async move {
        let ddragon_opt = build_provider::ddragon::prewarm_ddragon_from_disk(active_locale);
        bootstrap_state.update(&app_handle, "ddragon_disk", 40, false, false);

        profile_provider::prewarm_skins_from_disk(active_locale);
        let _ = profile_provider::get_cached_profile_for_last_summoner(active_locale);
        bootstrap_state.update(&app_handle, "profile_disk", 60, false, false);

        if let Some(ref dd) = ddragon_opt {
            build_provider::analytics::ranked::prewarm_ranked_tier_list_from_dir(
                &paths.opgg_ranked_dir,
                &settings.navigation.ranked_filter_region,
                &settings.navigation.ranked_filter_tier,
                settings.navigation.ranked_filter_patch.as_deref(),
                settings.automation.patch_48h_protection,
                active_locale,
                dd,
            );
            build_provider::analytics::aram::prewarm_aram_tier_lists_from_dir(
                &paths.opgg_aram_dir,
                active_locale,
                dd,
            );
        }
        bootstrap_state.update(&app_handle, "tier_lists_disk", 80, true, false);

        let client = build_provider::shared_http_client();
        let (_, opgg_versions) = tokio::join!(
            build_provider::refresh_ddragon_if_outdated(client, active_locale),
            build_provider::fetch_opgg_versions(client, "global"),
        );

        storage::run_garbage_collector(&paths, &opgg_versions, storage::now_unix_secs());
        bootstrap_state.update(&app_handle, "ready", 100, true, true);
    });
}

/// Initializes local storage, loads settings, builds the main window, and spawns background tasks.
pub fn setup_app(
    app: &mut tauri::App,
    window_tracker: &Arc<Mutex<storage::WindowSettings>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle().clone();
    let bootstrap_state = BootstrapState::default();
    app.manage(bootstrap_state.clone());

    // 1. Resolve %LOCALAPPDATA%\schumazgg\ and ensure Config/ & Cache/ subfolders
    let paths = storage::AppPaths::resolve_from_app(&app_handle).map_err(std::io::Error::other)?;
    paths.ensure_dirs().map_err(std::io::Error::other)?;
    storage::set_global_paths(paths.clone());

    // 2. Load UserSettings.ini BEFORE creating window or spawning background tasks
    let settings = storage::load_user_settings(&paths.user_settings_ini_path());
    storage::init_global_settings(paths.user_settings_ini_path(), settings.clone());

    let active_locale = settings.general.language.unwrap_or_default();
    lcu_injector::set_active_locale(active_locale);

    if let Ok(mut guard) = window_tracker.lock() {
        *guard = settings.window.clone();
    }

    app.manage(storage::GraphicsState::new(
        settings.graphics.hardware_acceleration,
    ));

    bootstrap_state.update(&app_handle, "settings_loaded", 20, false, false);

    // 3. Validate window bounds against connected monitors in logical coordinates
    let monitors: Vec<storage::MonitorRect> = app
        .available_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|m| {
            let scale = m.scale_factor();
            let pos = m.position().to_logical::<f64>(scale);
            let size = m.size().to_logical::<f64>(scale);
            storage::MonitorRect {
                x: pos.x,
                y: pos.y,
                width: size.width,
                height: size.height,
            }
        })
        .collect();
    let bounds = storage::validate_window_bounds(&settings.window, &monitors);

    if let Some(mut window_config) = app.config().app.windows.first().cloned() {
        window_config.width = bounds.width;
        window_config.height = bounds.height;
        window_config.x = bounds.x;
        window_config.y = bounds.y;
        window_config.center = bounds.center;
        window_config.maximized = bounds.maximized;
        window_config.additional_browser_args =
            Some(storage::browser_args_for_hardware_acceleration(
                settings.graphics.hardware_acceleration,
            ));
        tauri::WebviewWindowBuilder::from_config(&app_handle, &window_config)?.build()?;
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

    // Background task 3: Pre-warming (0ms disk hydration -> network version check -> GC)
    spawn_prewarm_task(app_handle, bootstrap_state, paths, settings, active_locale);

    Ok(())
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
    fn test_bootstrap_state_defaults_and_snapshot() {
        let state = BootstrapState::default();
        let snap = state.snapshot();
        assert_eq!(snap.stage, "initializing");
        assert_eq!(snap.progress, 0);
        assert!(!snap.disk_ready);
        assert!(!snap.network_ready);
    }
}
