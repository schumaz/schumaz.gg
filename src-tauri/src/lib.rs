pub mod build_provider;
pub mod lcu_connector;
pub mod lcu_injector;
pub mod ws_listener;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Tauri command to fetch the recommended build (with visual metadata) for a champion.
#[tauri::command]
async fn get_recommended_build_for_champion(
    champion_id: i32,
    queue_id: Option<i32>,
) -> Result<build_provider::ChampionBuild, String> {
    let http_client = reqwest::Client::new();
    let q_id = queue_id.unwrap_or(ws_listener::QUEUE_SOLO_DUO);
    build_provider::get_recommended_build(&http_client, champion_id, q_id).await
}

/// Tauri command to fetch the current summoner status if the client is connected.
#[tauri::command]
async fn get_summoner_status() -> Result<Option<lcu_connector::SummonerInfo>, String> {
    match lcu_connector::LcuConnection::connect() {
        Ok(conn) => match conn.get_current_summoner().await {
            Ok(summoner) => Ok(Some(summoner)),
            Err(_) => Ok(None),
        },
        Err(_) => Ok(None),
    }
}

/// Tauri command to sync user preferences from the frontend to the backend engine.
#[tauri::command]
fn sync_user_preferences(preferences: lcu_injector::UserPreferences) {
    lcu_injector::set_user_preferences(preferences);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_recommended_build_for_champion,
            get_summoner_status,
            sync_user_preferences
        ])
        .setup(|app| {
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

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
