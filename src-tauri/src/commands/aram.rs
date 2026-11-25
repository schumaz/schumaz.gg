use crate::{build_provider, models};

/// Tauri command to fetch ARAM or ARAM Mayhem champion tier list.
#[tauri::command]
#[specta::specta]
pub async fn get_aram_tier_list(
    mode: models::AramMode,
) -> Result<Vec<models::AramChampionRank>, String> {
    build_provider::get_aram_tier_list_service(mode).await
}

/// Tauri command to fetch ARAM Mayhem augments for a specific champion key.
#[tauri::command]
#[specta::specta]
pub async fn get_aram_mayhem_augments(
    champion_key: String,
) -> Result<Vec<models::AramAugmentEntry>, String> {
    build_provider::get_aram_mayhem_augments_service(&champion_key).await
}

/// Tauri command to fetch a read-only ARAM or ARAM Mayhem champion build preview
/// without emitting Champ Select events or injecting into LCU.
#[tauri::command]
#[specta::specta]
pub async fn get_aram_champion_build(
    champion_id: i32,
    mode: models::AramMode,
    tier: Option<String>,
    region: Option<String>,
    patch: Option<String>,
) -> Result<models::ChampionBuild, String> {
    build_provider::get_aram_champion_build_service(
        champion_id,
        mode,
        tier.as_deref(),
        region.as_deref(),
        patch.as_deref(),
    )
    .await
}
