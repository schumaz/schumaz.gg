use crate::{lcu_connector, models, ws_listener};

#[tauri::command]
#[specta::specta]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Tauri command to fetch the current summoner status if the client is connected.
#[tauri::command]
#[specta::specta]
pub async fn get_summoner_status() -> Result<Option<models::SummonerInfo>, String> {
    match lcu_connector::LcuConnection::connect() {
        Ok(conn) => match conn.get_current_summoner().await {
            Ok(summoner) => Ok(Some(summoner)),
            Err(_) => Ok(None),
        },
        Err(_) => Ok(None),
    }
}

/// Syncs the champion ID currently displayed in the frontend UI to backend session state
/// so final Champ Select reconciliation can detect UI/backend divergence.
#[tauri::command]
#[specta::specta]
pub async fn sync_displayed_champion(champion_id: Option<i32>) {
    ws_listener::set_ui_displayed_champion(champion_id).await;
}
