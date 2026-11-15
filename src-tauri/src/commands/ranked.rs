use crate::{build_provider, models};

/// Tauri command to fetch Ranked Summoner's Rift tier list filtered by region, elo tier, and patch.
#[tauri::command]
#[specta::specta]
pub async fn get_ranked_tier_list(
    region: Option<String>,
    tier: Option<String>,
    patch: Option<String>,
) -> Result<models::RankedTierListResponse, String> {
    build_provider::get_ranked_tier_list_service(
        region.as_deref(),
        tier.as_deref(),
        patch.as_deref(),
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
    build_provider::get_ranked_champion_build_service(
        champion_id,
        position.as_deref(),
        tier.as_deref(),
        region.as_deref(),
        patch.as_deref(),
    )
    .await
}
