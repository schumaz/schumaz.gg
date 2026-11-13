use crate::{lcu_connector, models};

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
