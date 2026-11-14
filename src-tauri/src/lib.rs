pub mod build_provider;
pub mod commands;
pub mod lcu_connector;
pub mod lcu_injector;
pub mod models;
pub mod profile_provider;
pub mod ws_listener;

use specta_typescript::Typescript;
use tauri::Manager;
use tauri_specta::{Builder, collect_commands, collect_events};

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
            commands::inject_summoner_spells,
            commands::inject_rune_page,
            commands::inject_item_build
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
            models::MatchEnded
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

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Focused(true)) {
                let _ = window.request_user_attention(None);
            }
        })
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            let app_handle = app.handle().clone();

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
