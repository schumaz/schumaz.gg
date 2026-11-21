pub mod app_bootstrap;
pub mod build_provider;
pub mod commands;
pub mod lcu_connector;
pub mod lcu_injector;
pub mod logging;
pub mod models;
pub mod profile_provider;
pub mod storage;
pub mod updater;
pub mod ws_listener;

pub use app_bootstrap::{
    focus_main_window, request_taskbar_attention, should_pause_window_animations,
};

use specta_typescript::Typescript;
use tauri_specta::{Builder, collect_commands, collect_events};

pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::get_recommended_build_for_champion,
            commands::update_build_filters,
            commands::get_summoner_status,
            commands::get_gameflow_phase,
            commands::get_active_champ_select_snapshot,
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
            commands::get_arena_tier_list,
            commands::get_arena_champion_build,
            commands::get_bootstrap_status,
            commands::get_app_distribution_info,
            commands::probe_for_app_update,
            commands::record_update_check_completed,
            commands::get_hardware_acceleration_status,
            commands::set_hardware_acceleration,
            commands::set_close_to_tray,
            commands::get_autostart_status,
            commands::set_autostart_enabled,
            commands::get_app_settings,
            commands::update_profile_appearance,
            commands::update_navigation_state,
            commands::complete_language_onboarding,
            commands::complete_flash_onboarding,
            commands::get_local_cache_info,
            commands::clear_local_cache,
            commands::get_diagnostic_summary,
            commands::open_logs_folder,
            commands::restart_app
        ])
        .events(collect_events![
            models::BootstrapProgress,
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

    let tauri_builder = tauri::Builder::default();
    #[cfg(desktop)]
    let tauri_builder = tauri_builder
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            app_bootstrap::focus_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![app_bootstrap::AUTOSTART_LAUNCH_ARG]),
        ));

    let app = tauri_builder
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
            let window_state = runtime_state.clone();
            move |app| {
                #[cfg(desktop)]
                app.handle()
                    .plugin(tauri_plugin_updater::Builder::new().build())?;

                builder.mount_events(app);
                app_bootstrap::setup_app(app, &window_state)
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| match event {
        tauri::RunEvent::ExitRequested { api, code, .. }
            if code.is_none() && !app_bootstrap::should_allow_exit(app_handle) =>
        {
            api.prevent_exit();
        }
        tauri::RunEvent::Exit => {
            logging::log_info("app.lifecycle", "event=exit");
            logging::shutdown_logging();
        }
        _ => {}
    });
}
