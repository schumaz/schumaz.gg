use crate::{build_provider, lcu_connector, lcu_injector, models};

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
    lcu_injector::set_selected_rune_page(champion_id, page_index);
    let conn = lcu_connector::LcuConnection::connect()
        .map_err(|e| format!("Não foi possível conectar ao cliente do LoL: {}", e))?;
    lcu_injector::inject_specific_rune_page(
        &conn,
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
    if let (Some(champ_id), Some(sec_spell)) = (champion_id, secondary_spell_id) {
        lcu_injector::set_selected_secondary_spell(champ_id, sec_spell);
    }
    let conn = lcu_connector::LcuConnection::connect()
        .map_err(|e| format!("Não foi possível conectar ao cliente do LoL: {}", e))?;
    let payload = lcu_injector::build_spells_payload(spell1_id, spell2_id)?;
    conn.patch_summoner_spells(&payload)
        .await
        .map_err(|e| format!("Falha ao injetar feitiços no Champ Select: {}", e))?;
    Ok(())
}

/// Tauri command to immediately update and re-inject item sets into the LoL client shop.
#[tauri::command]
#[specta::specta]
pub async fn inject_item_build(
    champion_id: i32,
    build_index: Option<usize>,
    core_index: Option<usize>,
    final_index: Option<usize>,
) -> Result<(), String> {
    if let Some(idx) = build_index {
        lcu_injector::set_selected_build(champion_id, idx);
    }
    if let Some(idx) = core_index {
        lcu_injector::set_selected_core_build(champion_id, idx);
    }
    if let Some(idx) = final_index {
        lcu_injector::set_selected_final_build(champion_id, idx);
    }

    let conn = lcu_connector::LcuConnection::connect()
        .map_err(|e| format!("Não foi possível conectar ao cliente do LoL: {}", e))?;
    let summoner = conn
        .get_current_summoner()
        .await
        .map_err(|e| format!("Falha ao obter invocador: {}", e))?;

    let client = reqwest::Client::new();
    let build = build_provider::get_recommended_build(&client, champion_id, 420).await?;

    lcu_injector::inject_item_sets(&conn, summoner.summoner_id, &build).await
}

/// Tauri command to sync user preferences from the frontend to the backend engine.
#[tauri::command]
#[specta::specta]
pub fn sync_user_preferences(preferences: models::UserPreferences) {
    lcu_injector::set_user_preferences(preferences);
}
