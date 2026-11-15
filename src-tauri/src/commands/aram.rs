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
