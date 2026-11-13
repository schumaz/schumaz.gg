use crate::{build_provider, lcu_connector, lcu_injector, models, ws_listener};
use tauri_specta::Event;

/// Tauri command to fetch the recommended build (with visual metadata) for a champion.
#[tauri::command]
#[specta::specta]
pub async fn get_recommended_build_for_champion(
    champion_id: i32,
    queue_id: Option<i32>,
) -> Result<models::ChampionBuild, String> {
    let http_client = reqwest::Client::new();
    let q_id = queue_id.unwrap_or(ws_listener::QUEUE_SOLO_DUO);
    build_provider::get_recommended_build(&http_client, champion_id, q_id).await
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
    let http_client = reqwest::Client::new();
    let q_id = queue_id.unwrap_or(ws_listener::QUEUE_SOLO_DUO);

    let build = build_provider::get_recommended_build_with_filters(
        &http_client,
        champion_id,
        q_id,
        position.as_deref(),
        tier.as_deref(),
        region.as_deref(),
        patch.as_deref(),
    )
    .await?;

    let _ = models::RecommendedBuild(build.clone()).emit(&app_handle);

    // If client is connected and summoner is present, inject the updated build
    if let Ok(conn) = lcu_connector::LcuConnection::connect()
        && let Ok(summoner) = conn.get_current_summoner().await
    {
        let report = lcu_injector::inject_champion_build(&conn, summoner.summoner_id, &build).await;
        let _ = models::BuildInjected(report).emit(&app_handle);
    }

    Ok(build)
}
