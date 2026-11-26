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

/// Tauri command returning a consolidated bootstrap snapshot of the active LCU gameflow phase,
/// Champ Select payload, and compatible cached build (probing LCU on demand if not yet polled).
#[tauri::command]
#[specta::specta]
pub async fn get_active_champ_select_snapshot(
    app_handle: tauri::AppHandle,
) -> models::ActiveChampSelectSnapshot {
    ws_listener::get_active_champ_select_snapshot(&app_handle).await
}

/// Tauri command to return the cached profile for the last known summoner in 0ms (even before LCU connects).
#[tauri::command]
#[specta::specta]
pub async fn get_cached_player_profile(
    summoner_id: Option<u64>,
) -> Option<models::CachedProfileResult> {
    tokio::task::spawn_blocking(move || {
        profile_provider::get_cached_profile_for_active_locale(summoner_id)
    })
    .await
    .ok()
    .flatten()
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

/// Tauri command to fetch an incremental match history page (`beg_index..=beg_index+count-1`) from LCU.
#[tauri::command]
#[specta::specta]
pub async fn get_player_match_history_page(
    beg_index: u32,
    count: Option<u32>,
) -> Result<models::MatchHistoryPage, String> {
    profile_provider::fetch_live_match_history_page(beg_index, count).await
}

/// Tauri command to fetch the full on-demand scoreboard for a single match (`5v5` or `Arena`).
#[tauri::command]
#[specta::specta]
pub async fn get_match_scoreboard_detail(
    game_id: u64,
) -> Result<models::MatchScoreboardDetail, String> {
    profile_provider::fetch_live_match_scoreboard_detail(game_id).await
}

/// Syncs the champion ID currently displayed in the frontend UI to backend session state
/// so final Champ Select reconciliation can detect UI/backend divergence.
#[tauri::command]
#[specta::specta]
pub async fn sync_displayed_champion(champion_id: Option<i32>) {
    ws_listener::set_ui_displayed_champion(champion_id).await;
}

/// Switches the active Swiftplay/Quickplay pre-selection slot (Primary vs Secondary)
/// and emits the corresponding build & selection events.
#[tauri::command]
#[specta::specta]
pub async fn select_lobby_preselect_slot(
    slot_index: u32,
    app_handle: tauri::AppHandle,
) -> Result<Option<models::ActiveChampSelectSnapshot>, String> {
    ws_listener::select_lobby_preselect_slot_service(slot_index, &app_handle).await
}
