use crate::{lcu_injector, models};

/// Tauri command to immediately inject a specific chosen rune page into the LoL client.
#[tauri::command]
#[specta::specta]
pub async fn inject_rune_page(
    champion_id: i32,
    champion_name: String,
    page_index: usize,
    primary_style_id: i32,
    sub_style_id: i32,
    selected_perk_ids: Vec<i32>,
) -> Result<(), String> {
    lcu_injector::apply_manual_rune_page(
        champion_id,
        &champion_name,
        page_index,
        primary_style_id,
        sub_style_id,
        &selected_perk_ids,
    )
    .await
}

/// Tauri command to immediately inject only summoner spells into an active Champ Select session.
#[tauri::command]
#[specta::specta]
pub async fn inject_summoner_spells(
    spell1_id: i32,
    spell2_id: i32,
    champion_id: Option<i32>,
    secondary_spell_id: Option<i32>,
) -> Result<(), String> {
    lcu_injector::apply_manual_summoner_spells(
        spell1_id,
        spell2_id,
        champion_id,
        secondary_spell_id,
    )
    .await
}

/// Tauri command to immediately update and re-inject item sets into the LoL client shop.
#[tauri::command]
#[specta::specta]
pub async fn inject_item_build(
    champion_id: i32,
    queue_id: Option<i32>,
    build_index: Option<usize>,
) -> Result<(), String> {
    lcu_injector::apply_manual_item_build(champion_id, queue_id, build_index).await
}

/// Tauri command to sync user preferences from the frontend to the backend engine and `UserSettings.ini`.
#[tauri::command]
#[specta::specta]
pub async fn sync_user_preferences(preferences: models::UserPreferences) {
    lcu_injector::sync_user_preferences_service(preferences);
}

/// Tauri command to update the active backend locale (`pt_BR` | `en_US`), persist to `UserSettings.ini`, and pre-warm DataDragon cache.
#[tauri::command]
#[specta::specta]
pub async fn set_locale(locale: models::AppLocale) {
    lcu_injector::set_locale_service(locale);
}
