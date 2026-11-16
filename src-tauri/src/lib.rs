pub mod app_bootstrap;
pub mod build_provider;
pub mod commands;
pub mod lcu_connector;
pub mod lcu_injector;
pub mod models;
pub mod profile_provider;
pub mod storage;
pub mod ws_listener;

pub use app_bootstrap::{request_taskbar_attention, should_pause_window_animations};

use specta_typescript::Typescript;
use tauri_specta::{Builder, collect_commands, collect_events};

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
            commands::complete_flash_onboarding,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let builder = specta_builder();

    #[cfg(debug_assertions)]
    builder
        .export(Typescript::default(), "../src/types/bindings.ts")
        .expect("Failed to export TypeScript bindings");

    let runtime_state = app_bootstrap::WindowRuntimeState::default();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .register_asynchronous_uri_scheme_protocol("schumaz-asset", |_ctx, request, responder| {
            let uri_path = request.uri().path().to_string();
            app_bootstrap::handle_schumaz_asset_protocol(uri_path, responder);
        })
        .on_window_event({
            let window_state = runtime_state.clone();
            move |window, event| {
                app_bootstrap::handle_window_event(window, event, &window_state);
            }
        })
        .invoke_handler(builder.invoke_handler())
        .setup({
            let window_tracker = runtime_state.window_tracker.clone();
            move |app| {
                builder.mount_events(app);
                app_bootstrap::setup_app(app, &window_tracker)
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
