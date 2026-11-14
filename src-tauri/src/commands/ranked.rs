use crate::{build_provider, models, ws_listener};

/// Tauri command to fetch Ranked Summoner's Rift tier list filtered by region, elo tier, and patch.
#[tauri::command]
#[specta::specta]
pub async fn get_ranked_tier_list(
    region: Option<String>,
    tier: Option<String>,
    patch: Option<String>,
) -> Result<models::RankedTierListResponse, String> {
    let http_client = build_provider::shared_http_client();
    let ddragon = build_provider::get_or_init_ddragon_cache(http_client).await?;
    build_provider::fetch_ranked_tier_list(
        http_client,
        region.as_deref(),
        tier.as_deref(),
        patch.as_deref(),
        ddragon,
    )
    .await
}

/// Tauri command to fetch a read-only champion build preview for the Ranked browser tab
/// without emitting Champ Select events or injecting into LCU.
#[tauri::command]
#[specta::specta]
pub async fn get_ranked_champion_build(
    champion_id: i32,
    position: Option<String>,
    tier: Option<String>,
    region: Option<String>,
    patch: Option<String>,
) -> Result<models::ChampionBuild, String> {
    let http_client = build_provider::shared_http_client();
    build_provider::get_recommended_build_with_filters(
        http_client,
        champion_id,
        ws_listener::QUEUE_SOLO_DUO,
        position.as_deref(),
        tier.as_deref(),
        region.as_deref(),
        patch.as_deref(),
    )
    .await
}
