pub mod build_provider;
pub mod commands;
pub mod lcu_connector;
pub mod lcu_injector;
pub mod models;
pub mod profile_provider;
pub mod ws_listener;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let builder = specta_builder();

    #[cfg(debug_assertions)]
    builder
        .export(Typescript::default(), "../src/types/bindings.ts")
        .expect("Failed to export TypeScript bindings");

    let was_paused = Arc::new(AtomicBool::new(false));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .on_window_event({
            let was_paused = Arc::clone(&was_paused);
            move |window, event| {
                if matches!(event, tauri::WindowEvent::Focused(true)) {
                    let _ = window.request_user_attention(None);
                }

                if window.label() == "main"
                    && matches!(
                        event,
                        tauri::WindowEvent::Resized(_)
                            | tauri::WindowEvent::Moved(_)
                            | tauri::WindowEvent::Focused(_)
                    )
                {
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
            }
        })
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            let app_handle = app.handle().clone();

            let graphics_config_path = commands::resolve_graphics_config_path(&app_handle);
            let hardware_acceleration = graphics_config_path
                .as_deref()
                .map(commands::load_hardware_acceleration_from_path)
                .unwrap_or(true);

            app.manage(commands::GraphicsState::new(
                hardware_acceleration,
                graphics_config_path,
            ));

            if let Some(mut window_config) = app.config().app.windows.first().cloned() {
                window_config.additional_browser_args = Some(
                    commands::browser_args_for_hardware_acceleration(hardware_acceleration),
                );
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

            // Background task 3: Pre-warm DataDragon cache and OP.GG versions on startup
            // so the first Champion Select has zero cold-start network latency.
            tauri::async_runtime::spawn(async move {
                let client = build_provider::shared_http_client();
                let _ = tokio::join!(
                    build_provider::get_or_init_ddragon_cache(client),
                    build_provider::fetch_opgg_versions(client, "global"),
                );
            });

            Ok(())
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
