use super::injection::{inject_item_sets, inject_specific_rune_page};
use super::payloads::build_spells_payload;
use super::preferences::{get_user_preferences, set_active_locale, set_user_preferences};
use super::selections::{
    set_selected_build, set_selected_core_build, set_selected_final_build, set_selected_rune_page,
    set_selected_secondary_spell,
};
use crate::models::{AppLocale, FlashKeyPreference, QUEUE_SOLO_DUO, UserPreferences};
use crate::storage::{AppSettings, LegacyLocalStoragePayload};
use crate::{build_provider, lcu_connector, storage};

/// Resolves the active queue ID for item build injection without hardcoding 420.
pub fn normalize_item_build_queue_id(
    provided_queue_id: Option<i32>,
    lcu_queue_id: Option<i32>,
) -> i32 {
    match provided_queue_id {
        Some(q) if q > 0 => q,
        Some(_) => QUEUE_SOLO_DUO,
        None => match lcu_queue_id {
            Some(q) if q > 0 => q,
            _ => QUEUE_SOLO_DUO,
        },
    }
}

/// Connects to LCU, saves the selected rune page index, and injects it immediately.
pub async fn apply_manual_rune_page(
    champion_id: i32,
    champion_name: &str,
    page_index: usize,
    primary_style_id: i32,
    sub_style_id: i32,
    selected_perk_ids: &[i32],
) -> Result<(), String> {
    set_selected_rune_page(champion_id, page_index);
    let conn = lcu_connector::LcuConnection::connect()
        .map_err(|e| format!("Não foi possível conectar ao cliente do LoL: {}", e))?;
    inject_specific_rune_page(
        &conn,
        champion_id,
        champion_name,
        page_index,
        primary_style_id,
        sub_style_id,
        selected_perk_ids,
    )
    .await
}

/// Saves optional secondary spell selection, connects to LCU, and patches summoner spells.
pub async fn apply_manual_summoner_spells(
    spell1_id: i32,
    spell2_id: i32,
    champion_id: Option<i32>,
    secondary_spell_id: Option<i32>,
) -> Result<(), String> {
    if let (Some(champ_id), Some(sec_spell)) = (champion_id, secondary_spell_id) {
        set_selected_secondary_spell(champ_id, sec_spell);
    }
    let conn = lcu_connector::LcuConnection::connect()
        .map_err(|e| format!("Não foi possível conectar ao cliente do LoL: {}", e))?;
    let payload = build_spells_payload(spell1_id, spell2_id)?;
    conn.patch_summoner_spells(&payload)
        .await
        .map_err(|e| format!("Falha ao injetar feitiços no Champ Select: {}", e))?;
    Ok(())
}

/// Records item build selections, resolves the active queue, fetches the build, and injects item sets.
pub async fn apply_manual_item_build(
    champion_id: i32,
    queue_id: Option<i32>,
    build_index: Option<usize>,
    core_index: Option<usize>,
    final_index: Option<usize>,
) -> Result<(), String> {
    if let Some(idx) = build_index {
        set_selected_build(champion_id, idx);
    }
    if let Some(idx) = core_index {
        set_selected_core_build(champion_id, idx);
    }
    if let Some(idx) = final_index {
        set_selected_final_build(champion_id, idx);
    }

    let conn = lcu_connector::LcuConnection::connect()
        .map_err(|e| format!("Não foi possível conectar ao cliente do LoL: {}", e))?;
    let summoner = conn
        .get_current_summoner()
        .await
        .map_err(|e| format!("Falha ao obter invocador: {}", e))?;

    let lcu_queue_id = if queue_id.is_none() {
        conn.get_current_queue_id().await.ok()
    } else {
        None
    };
    let resolved_queue_id = normalize_item_build_queue_id(queue_id, lcu_queue_id);

    let client = build_provider::shared_http_client();
    let build =
        build_provider::get_recommended_build(client, champion_id, resolved_queue_id).await?;

    inject_item_sets(&conn, summoner.summoner_id, &build).await
}

/// Syncs user preferences to memory + `UserSettings.ini`, warms DataDragon on locale change,
/// and runs daily mission auto-mark if enabled.
pub fn sync_user_preferences_service(preferences: UserPreferences) {
    let prev = get_user_preferences();
    let should_auto_mark = preferences.auto_mark_daily_missions_read;
    let target_locale = preferences.language;
    set_user_preferences(preferences.clone());
    let _ = storage::update_global_settings(|s| {
        s.automation = preferences;
    });

    if prev.language != target_locale {
        let client = build_provider::shared_http_client().clone();
        tauri::async_runtime::spawn(async move {
            let _ =
                build_provider::get_or_init_ddragon_cache_for_locale(&client, target_locale).await;
        });
    }
    if should_auto_mark {
        if !prev.auto_mark_daily_missions_read {
            lcu_connector::reset_auto_mark_session_state();
        }
        lcu_connector::spawn_auto_mark_daily_missions_if_enabled();
    }
}

/// Updates the active backend locale, persists it to `UserSettings.ini`, and pre-warms DataDragon.
pub fn set_locale_service(locale: AppLocale) {
    set_active_locale(locale);
    let _ = storage::update_global_settings(|s| {
        s.general.language = Some(locale);
        s.automation.language = locale;
    });
    let client = build_provider::shared_http_client().clone();
    tauri::async_runtime::spawn(async move {
        let _ = build_provider::get_or_init_ddragon_cache_for_locale(&client, locale).await;
    });
}

/// Marks language onboarding completed, persists locale to `UserSettings.ini`, and pre-warms DataDragon.
pub fn complete_language_onboarding_service(locale: AppLocale) -> Result<AppSettings, String> {
    set_active_locale(locale);
    let updated = storage::update_global_settings(|s| {
        s.general.language = Some(locale);
        s.general.language_onboarding_completed = true;
        s.automation.language = locale;
    })?;

    let client = build_provider::shared_http_client().clone();
    tauri::async_runtime::spawn(async move {
        let _ = build_provider::get_or_init_ddragon_cache_for_locale(&client, locale).await;
    });

    Ok(updated)
}

/// Marks Flash key onboarding completed, persists `flashKey` to `UserSettings.ini`, and syncs preferences.
pub fn complete_flash_onboarding_service(
    flash_key: FlashKeyPreference,
) -> Result<AppSettings, String> {
    let updated = storage::update_global_settings(|s| {
        s.general.flash_onboarding_completed = true;
        s.automation.flash_key = flash_key;
    })?;

    set_user_preferences(updated.automation.clone());
    Ok(updated)
}

/// Migrates legacy `localStorage` keys into `UserSettings.ini` and syncs active automation preferences.
pub fn migrate_legacy_local_storage_service(
    payload: LegacyLocalStoragePayload,
) -> Result<AppSettings, String> {
    let updated = storage::update_global_settings(|s| {
        let _ = storage::apply_legacy_local_storage_migration(s, &payload);
    })?;

    set_user_preferences(updated.automation.clone());
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_item_build_queue_id_respects_active_queue() {
        assert_eq!(normalize_item_build_queue_id(Some(450), None), 450);
        assert_eq!(normalize_item_build_queue_id(Some(2400), None), 2400);
        assert_eq!(normalize_item_build_queue_id(Some(440), None), 440);
        assert_eq!(normalize_item_build_queue_id(Some(-1), None), 420);
        assert_eq!(normalize_item_build_queue_id(None, Some(450)), 450);
        assert_eq!(normalize_item_build_queue_id(None, Some(2400)), 2400);
        assert_eq!(normalize_item_build_queue_id(None, None), 420);
    }

    #[test]
    fn test_complete_flash_onboarding_service_persists_flash_key_and_flag() {
        let updated = complete_flash_onboarding_service(FlashKeyPreference::F).unwrap();
        assert!(updated.general.flash_onboarding_completed);
        assert_eq!(updated.automation.flash_key, FlashKeyPreference::F);
        assert_eq!(get_user_preferences().flash_key, FlashKeyPreference::F);
    }
}
