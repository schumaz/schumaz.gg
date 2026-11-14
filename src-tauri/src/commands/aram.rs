use crate::{build_provider, models};

/// Tauri command to fetch ARAM or ARAM Mayhem champion tier list.
#[tauri::command]
#[specta::specta]
pub async fn get_aram_tier_list(
    mode: models::AramMode,
) -> Result<Vec<models::AramChampionRank>, String> {
    let http_client = build_provider::shared_http_client();
    let ddragon = build_provider::get_or_init_ddragon_cache(http_client).await?;
    build_provider::fetch_aram_tier_list(http_client, mode, ddragon).await
}

/// Tauri command to fetch ARAM Mayhem augments for a specific champion key.
#[tauri::command]
#[specta::specta]
pub async fn get_aram_mayhem_augments(
    champion_key: String,
) -> Result<Vec<models::AramAugmentEntry>, String> {
    let http_client = build_provider::shared_http_client();
    build_provider::fetch_aram_mayhem_augments(http_client, &champion_key).await
}
