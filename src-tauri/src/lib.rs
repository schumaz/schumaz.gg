pub mod build_provider;
pub mod commands;
pub mod lcu_connector;
pub mod lcu_injector;
pub mod models;
pub mod profile_provider;
pub mod storage;
pub mod ws_listener;

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

use specta_typescript::Typescript;
use tauri::Manager;
use tauri_specta::{Builder, Event, collect_commands, collect_events};

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

pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::greet,
            commands::get_recommended_build_for_champion,
            commands::update_build_filters,
            commands::get_summoner_status,
            commands::get_cached_player_profile,
            commands::get_player_profile,
            commands::sync_displayed_champion,
            commands::sync_user_preferences,
            commands::set_locale,
            commands::inject_summoner_spells,
            commands::inject_rune_page,
            commands::inject_item_build,
            commands::get_aram_tier_list,
            commands::get_aram_mayhem_augments,
            commands::get_ranked_tier_list,
            commands::get_ranked_champion_build,
            commands::get_hardware_acceleration_status,
            commands::set_hardware_acceleration,
            commands::get_app_settings,
            commands::update_profile_appearance,
            commands::update_navigation_state,
            commands::complete_language_onboarding,
            commands::migrate_legacy_local_storage,
            commands::restart_app
        ])
        .events(collect_events![
            models::LcuConnected,
            models::LcuDisconnected,
            models::ChampSelected,
            models::ChampSelectEnded,
            models::RecommendedBuild,
            models::BuildInjected,
            models::ChampSelectReconciled,
            models::ProfileRefreshRequested,
            models::GameflowPhaseChanged,
            models::MatchEnded,
            models::WindowMinimizedChanged
        ])
        .dangerously_cast_bigints_to_number()
}

pub fn export_bindings() {
    specta_builder()
        .export(Typescript::default(), "../src/types/bindings.ts")
        .expect("Failed to export TypeScript bindings");
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let builder = specta_builder();

    #[cfg(debug_assertions)]
    builder
        .export(Typescript::default(), "../src/types/bindings.ts")
        .expect("Failed to export TypeScript bindings");

    let was_paused = Arc::new(AtomicBool::new(false));
    let window_tracker = Arc::new(Mutex::new(storage::WindowSettings::default()));
    let window_save_seq = Arc::new(AtomicU64::new(0));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .register_asynchronous_uri_scheme_protocol("schumaz-asset", |_ctx, request, responder| {
            let uri_path = request.uri().path().to_string();
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
        })
        .on_window_event({
            let was_paused = Arc::clone(&was_paused);
            let window_tracker = Arc::clone(&window_tracker);
            let window_save_seq = Arc::clone(&window_save_seq);
            move |window, event| {
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
                    let paused =
                        should_pause_window_animations(is_minimized, is_visible, is_focused);
                    let prev = was_paused.swap(paused, Ordering::SeqCst);
                    if prev != paused {
                        let _ = models::WindowMinimizedChanged(paused).emit(window.app_handle());
                    }
                }

                match event {
                    tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_) => {
                        if let Some(snapshot) = capture_window_settings(window, &window_tracker) {
                            let seq = window_save_seq.fetch_add(1, Ordering::SeqCst) + 1;
                            let seq_ref = Arc::clone(&window_save_seq);
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
                        window_save_seq.fetch_add(1, Ordering::SeqCst);
                        if let Some(snapshot) = capture_window_settings(window, &window_tracker) {
                            let _ = storage::update_global_settings(|s| {
                                s.window = snapshot;
                            });
                        }
                    }
                    _ => {}
                }
            }
        })
        .invoke_handler(builder.invoke_handler())
        .setup({
            let window_tracker = Arc::clone(&window_tracker);
            move |app| {
                builder.mount_events(app);
                let app_handle = app.handle().clone();

                // 1. Resolve %LOCALAPPDATA%\schumazgg\ and ensure Config/ & Cache/ subfolders
                let paths = storage::AppPaths::resolve_from_app(&app_handle)
                    .map_err(|e| std::io::Error::other(e))?;
                paths.ensure_dirs().map_err(|e| std::io::Error::other(e))?;
                storage::set_global_paths(paths.clone());

                // 2. Load or migrate UserSettings.ini BEFORE creating window or spawning background tasks
                let legacy_gpu_path = commands::resolve_legacy_graphics_config_path(&app_handle);
                let settings = storage::load_or_migrate_user_settings(
                    &paths.user_settings_ini_path(),
                    legacy_gpu_path.as_deref(),
                );
                storage::init_global_settings(paths.user_settings_ini_path(), settings.clone());

                let active_locale = settings.general.language.unwrap_or_default();
                lcu_injector::set_active_locale(active_locale);

                if let Ok(mut guard) = window_tracker.lock() {
                    *guard = settings.window.clone();
                }

                app.manage(commands::GraphicsState::new(
                    settings.graphics.hardware_acceleration,
                ));

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
                        Some(commands::browser_args_for_hardware_acceleration(
                            settings.graphics.hardware_acceleration,
                        ));
                    tauri::WebviewWindowBuilder::from_config(&app_handle, &window_config)?
                        .build()?;
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
                tauri::async_runtime::spawn(async move {
                    let ddragon_opt =
                        build_provider::ddragon::prewarm_ddragon_from_disk(active_locale);
                    profile_provider::prewarm_skins_from_disk(active_locale);
                    let _ = profile_provider::get_cached_profile_for_last_summoner(active_locale);

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

                    let client = build_provider::shared_http_client();
                    let (_, opgg_versions) = tokio::join!(
                        build_provider::refresh_ddragon_if_outdated(client, active_locale),
                        build_provider::fetch_opgg_versions(client, "global"),
                    );

                    storage::run_garbage_collector(
                        &paths,
                        &opgg_versions,
                        storage::now_unix_secs(),
                    );
                });

                Ok(())
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
