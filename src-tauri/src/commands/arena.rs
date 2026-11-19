use crate::{build_provider, models};

/// Tauri command to fetch the Arena (`CHERRY`) champion tier list, optionally filtered by patch.
#[tauri::command]
#[specta::specta]
pub async fn get_arena_tier_list(
    patch: Option<String>,
) -> Result<models::ArenaTierListResponse, String> {
    build_provider::get_arena_tier_list_service(patch.as_deref()).await
}

/// Tauri command to fetch a read-only Arena (`CHERRY`) champion build preview
/// (items, prismatics, augments, synergies) without injecting into LCU.
#[tauri::command]
#[specta::specta]
pub async fn get_arena_champion_build(
    champion_id: i32,
    patch: Option<String>,
) -> Result<models::ChampionBuild, String> {
    build_provider::get_arena_champion_build_service(champion_id, patch.as_deref()).await
}
