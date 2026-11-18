use crate::{lcu_connector, models, profile_provider, ws_listener};

/// Tauri command to fetch the current summoner status if the client is connected.
#[tauri::command]
#[specta::specta]
pub async fn get_summoner_status() -> Result<Option<models::SummonerInfo>, String> {
    Ok(lcu_connector::fetch_current_summoner_status().await)
}

/// Tauri command to fetch the current LCU gameflow phase if the client is connected.
#[tauri::command]
#[specta::specta]
pub async fn get_gameflow_phase() -> Option<String> {
    lcu_connector::fetch_current_gameflow_phase().await
}

/// Tauri command to return the cached profile for the last known summoner in 0ms (even before LCU connects).
#[tauri::command]
#[specta::specta]
pub fn get_cached_player_profile(summoner_id: Option<u64>) -> Option<models::CachedProfileResult> {
    profile_provider::get_cached_profile_for_active_locale(summoner_id)
}

/// Tauri command to fetch aggregated player profile (wallpaper, ranked stats, top 3 masteries, 10 matches).
/// When `force_refresh` is `Some(true)`, bypasses the `.lz4` disk cache.
#[tauri::command]
#[specta::specta]
pub async fn get_player_profile(
    force_refresh: Option<bool>,
) -> Result<Option<models::PlayerProfile>, String> {
    Ok(profile_provider::fetch_live_player_profile(force_refresh.unwrap_or(false)).await)
}

/// Syncs the champion ID currently displayed in the frontend UI to backend session state
/// so final Champ Select reconciliation can detect UI/backend divergence.
#[tauri::command]
#[specta::specta]
pub async fn sync_displayed_champion(champion_id: Option<i32>) {
    ws_listener::set_ui_displayed_champion(champion_id).await;
}
