use crate::{build_provider, models, ws_listener};

/// Tauri command to fetch the recommended build (with visual metadata) for a champion.
#[tauri::command]
#[specta::specta]
pub async fn get_recommended_build_for_champion(
    champion_id: i32,
    queue_id: Option<i32>,
) -> Result<models::ChampionBuild, String> {
    build_provider::get_recommended_build_for_champion_service(champion_id, queue_id).await
}

/// Tauri command to update build filters (position, tier, region, patch) and re-inject if connected.
#[tauri::command]
#[specta::specta]
pub async fn update_build_filters(
    champion_id: i32,
    queue_id: Option<i32>,
    position: Option<String>,
    tier: Option<String>,
    region: Option<String>,
    patch: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<models::ChampionBuild, String> {
    ws_listener::update_build_filters_and_reinject(
        champion_id,
        queue_id,
        position,
        tier,
        region,
        patch,
        &app_handle,
    )
    .await
}
