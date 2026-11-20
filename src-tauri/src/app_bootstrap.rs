pub mod window_events;

pub use window_events::{
    WindowRuntimeState, focus_main_window, handle_window_event, request_taskbar_attention,
    should_pause_window_animations,
};

use std::sync::{Arc, Mutex, atomic::Ordering};

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
            build_provider::analytics::arena::prewarm_arena_tier_list_from_dir(
                &paths.opgg_arena_dir,
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

fn collect_monitor_rects(app: &tauri::App) -> Vec<storage::MonitorRect> {
    let primary_key = app.primary_monitor().ok().flatten().map(|m| {
        (
            m.name().cloned(),
            m.position().x,
            m.position().y,
            m.size().width,
            m.size().height,
        )
    });

    app.available_monitors()
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

/// Initializes local storage, loads settings, builds the main window, and spawns background tasks.
pub fn setup_app(
    app: &mut tauri::App,
    window_state: &WindowRuntimeState,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle().clone();
    let bootstrap_state = BootstrapState::default();
    app.manage(bootstrap_state.clone());

    // 1. Resolve %LOCALAPPDATA%\schumazgg\ and ensure Config/, Cache/, & WebView/ subfolders
    let paths = storage::AppPaths::resolve_from_app(&app_handle).map_err(std::io::Error::other)?;
    paths.ensure_dirs().map_err(std::io::Error::other)?;
    storage::set_global_paths(paths.clone());

    // 2. Load UserSettings.ini BEFORE creating window or spawning background tasks
    let settings = storage::load_user_settings(&paths.user_settings_ini_path());
    storage::init_global_settings(paths.user_settings_ini_path(), settings.clone());

    let active_locale = settings.general.language.unwrap_or_default();
    lcu_injector::set_active_locale(active_locale);

    if let Ok(mut guard) = window_state.window_tracker.lock() {
        *guard = settings.window.clone();
    }

    app.manage(storage::GraphicsState::new(
        settings.graphics.hardware_acceleration,
    ));

    bootstrap_state.update(&app_handle, "settings_loaded", 20, false, false);

    // 3. Validate window bounds & monitor against connected displays (physical work-area + DPI)
    let monitors = collect_monitor_rects(app);
    let bounds = storage::validate_window_bounds(&settings.window, &monitors);

    if let Some(mut window_config) = app.config().app.windows.first().cloned() {
        window_state.restoring_window.store(true, Ordering::SeqCst);

        window_config.width = bounds.width;
        window_config.height = bounds.height;
        window_config.x = None;
        window_config.y = None;
        window_config.center = bounds.x.is_none() || bounds.y.is_none();
        window_config.maximized = false;
        window_config.visible = false;
        window_config.additional_browser_args =
            Some(storage::browser_args_for_hardware_acceleration(
                settings.graphics.hardware_acceleration,
            ));

        let build_result = tauri::WebviewWindowBuilder::from_config(&app_handle, &window_config)
            .and_then(|builder| builder.data_directory(paths.webview_dir.clone()).build());

        match build_result {
            Ok(window) => {
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
            }
            Err(err) => {
                window_state.restoring_window.store(false, Ordering::SeqCst);
                return Err(err.into());
            }
        }
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
    fn test_bootstrap_state_defaults_and_snapshot() {
        let state = BootstrapState::default();
        let snap = state.snapshot();
        assert_eq!(snap.stage, "initializing");
        assert_eq!(snap.progress, 0);
        assert!(!snap.disk_ready);
        assert!(!snap.network_ready);
    }
}
