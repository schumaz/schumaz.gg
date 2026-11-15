use crate::{lcu_connector, lcu_injector, models, profile_provider, ws_listener};

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

/// Tauri command to return the cached profile for the last known summoner in 0ms (even before LCU connects).
#[tauri::command]
#[specta::specta]
pub fn get_cached_player_profile(summoner_id: Option<u64>) -> Option<models::CachedProfileResult> {
    let locale = lcu_injector::get_active_locale();
    match summoner_id {
        Some(id) if id > 0 => profile_provider::get_cached_profile_for_summoner_id(id, locale),
        _ => profile_provider::get_cached_profile_for_last_summoner(locale),
    }
}

/// Tauri command to fetch aggregated player profile (wallpaper, ranked stats, top 3 masteries, 10 matches).
/// When `force_refresh` is `Some(true)`, bypasses the `.lz4` disk cache.
#[tauri::command]
#[specta::specta]
pub async fn get_player_profile(
    force_refresh: Option<bool>,
) -> Result<Option<models::PlayerProfile>, String> {
    let Ok(conn) = lcu_connector::LcuConnection::connect() else {
        return Ok(None);
    };

    match profile_provider::load_player_profile_with_cache(&conn, force_refresh.unwrap_or(false))
        .await
    {
        Ok(profile) => Ok(Some(profile)),
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
