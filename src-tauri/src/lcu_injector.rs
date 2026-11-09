//! Module 4: Client Injection (LCU)
//! Handles applying rune pages, summoner spells, and item sets back to the local LCU client.

use crate::build_provider::ChampionBuild;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::{OnceLock, RwLock};

pub const PERKS_CURRENT_PAGE_ENDPOINT: &str = "/lol-perks/v1/currentpage";
pub const PERKS_PAGES_ENDPOINT: &str = "/lol-perks/v1/pages";
pub const CHAMP_SELECT_MY_SELECTION_ENDPOINT: &str = "/lol-champ-select/v1/session/my-selection";
pub const SCHUMAZ_PAGE_SUFFIX: &str = " | schumaz.gg";
pub const SPELL_FLASH_ID: i32 = 4;
pub const SPELL_SMITE_ID: i32 = 11;

/// Represents the user's preferred key for Flash (D vs F).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum FlashKeyPreference {
    #[default]
    D,
    F,
}

/// User preferences synchronized between frontend and backend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserPreferences {
    pub auto_inject_runes: bool,
    pub auto_inject_spells: bool,
    pub auto_inject_items: bool,
    pub flash_key: FlashKeyPreference,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            auto_inject_runes: true,
            auto_inject_spells: true,
            auto_inject_items: true,
            flash_key: FlashKeyPreference::D,
        }
    }
}

static USER_PREFERENCES: OnceLock<RwLock<UserPreferences>> = OnceLock::new();

fn get_preferences_lock() -> &'static RwLock<UserPreferences> {
    USER_PREFERENCES.get_or_init(|| RwLock::new(UserPreferences::default()))
}

/// Retrieves a clone of the current global user preferences.
pub fn get_user_preferences() -> UserPreferences {
    get_preferences_lock()
        .read()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

/// Updates the global user preferences.
pub fn set_user_preferences(prefs: UserPreferences) {
    if let Ok(mut guard) = get_preferences_lock().write() {
        *guard = prefs;
    }
}

/// Applies Flash key preference (D vs F), swapping spells if needed so that Flash (ID 4)
/// matches the user's preferred slot (spell1Id = D, spell2Id = F).
pub fn apply_flash_key_preference(
    spell1_id: i32,
    spell2_id: i32,
    flash_key: FlashKeyPreference,
) -> (i32, i32) {
    let should_swap = (spell1_id == SPELL_FLASH_ID && flash_key == FlashKeyPreference::F)
        || (spell2_id == SPELL_FLASH_ID && flash_key == FlashKeyPreference::D);

    if should_swap {
        (spell2_id, spell1_id)
    } else {
        (spell1_id, spell2_id)
    }
}

/// Applies mandatory Jungle Smite rule (ID 11) and positions Flash (ID 4)
/// according to the user's preferred slot (D vs F).
pub fn apply_jungle_smite_and_flash_preference(
    mut spell1_id: i32,
    mut spell2_id: i32,
    flash_key: FlashKeyPreference,
    is_jungle: bool,
) -> (i32, i32) {
    if !is_jungle {
        return apply_flash_key_preference(spell1_id, spell2_id, flash_key);
    }

    // 1. Ensure Smite (11) is present
    if spell1_id != SPELL_SMITE_ID && spell2_id != SPELL_SMITE_ID {
        if spell1_id == SPELL_FLASH_ID {
            spell2_id = SPELL_SMITE_ID;
        } else if spell2_id == SPELL_FLASH_ID {
            spell1_id = SPELL_SMITE_ID;
        } else {
            spell2_id = SPELL_SMITE_ID;
        }
    }

    // 2. Position Flash and Smite respecting flash_key
    if spell1_id == SPELL_FLASH_ID || spell2_id == SPELL_FLASH_ID {
        let other_spell = if spell1_id == SPELL_FLASH_ID {
            spell2_id
        } else {
            spell1_id
        };
        match flash_key {
            FlashKeyPreference::D => (SPELL_FLASH_ID, other_spell),
            FlashKeyPreference::F => (other_spell, SPELL_FLASH_ID),
        }
    } else {
        // Champion doesn't use Flash (e.g. Ghost 6 + Smite 11 or Ignite 14 + Smite 11)
        (spell1_id, spell2_id)
    }
}

/// Summary report returned after attempting an injection into the LCU.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InjectionReport {
    pub champion_name: String,
    pub runes_injected: bool,
    pub spells_injected: bool,
    pub items_injected: bool,
    pub message: String,
}


#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChampionSelections {
    pub champion_id: i32,
    pub rune_page_index: Option<usize>,
    pub secondary_spell_id: Option<i32>,
}

static CHAMPION_SELECTIONS: OnceLock<RwLock<Option<ChampionSelections>>> = OnceLock::new();

fn get_selections_lock() -> &'static RwLock<Option<ChampionSelections>> {
    CHAMPION_SELECTIONS.get_or_init(|| RwLock::new(None))
}

pub fn set_selected_rune_page(champion_id: i32, page_index: usize) {
    if let Ok(mut guard) = get_selections_lock().write() {
        let current = guard.get_or_insert_with(|| ChampionSelections {
            champion_id,
            rune_page_index: None,
            secondary_spell_id: None,
        });
        if current.champion_id != champion_id {
            *current = ChampionSelections {
                champion_id,
                rune_page_index: Some(page_index),
                secondary_spell_id: None,
            };
        } else {
            current.rune_page_index = Some(page_index);
        }
    }
}

pub fn set_selected_secondary_spell(champion_id: i32, spell_id: i32) {
    if let Ok(mut guard) = get_selections_lock().write() {
        let current = guard.get_or_insert_with(|| ChampionSelections {
            champion_id,
            rune_page_index: None,
            secondary_spell_id: None,
        });
        if current.champion_id != champion_id {
            *current = ChampionSelections {
                champion_id,
                rune_page_index: None,
                secondary_spell_id: Some(spell_id),
            };
        } else {
            current.secondary_spell_id = Some(spell_id);
        }
    }
}

pub fn get_champion_selections(champion_id: i32) -> Option<ChampionSelections> {
    if let Ok(guard) = get_selections_lock().read()
        && let Some(sel) = guard.as_ref()
        && sel.champion_id == champion_id
    {
        return Some(sel.clone());
    }
    None
}

pub fn clear_champion_selections() {
    if let Ok(mut guard) = get_selections_lock().write() {
        *guard = None;
    }
}

/// Builds the JSON payload for creating/replacing a rune page from individual components.
pub fn build_rune_page_payload_from_parts(
    champion_name: &str,
    primary_style_id: i32,
    sub_style_id: i32,
    selected_perk_ids: &[i32],
) -> Result<serde_json::Value, String> {
    if selected_perk_ids.len() != 9 {
        return Err(format!(
            "Invalid perk count: expected 9 perks, found {}",
            selected_perk_ids.len()
        ));
    }

    if primary_style_id == sub_style_id {
        return Err("Primary style and sub style cannot be identical".to_string());
    }

    let payload = json!({
        "name": format!("{}{}", champion_name, SCHUMAZ_PAGE_SUFFIX),
        "primaryStyleId": primary_style_id,
        "subStyleId": sub_style_id,
        "selectedPerkIds": selected_perk_ids,
        "current": true
    });

    Ok(payload)
}

/// Builds the JSON payload for creating/replacing a rune page via POST /lol-perks/v1/pages.
/// Formats page name as "%Champname% | schumaz.gg".
/// Automatically respects user's selected rune page option for this champion if present in ChampionSelections.
pub fn build_rune_page_payload(build: &ChampionBuild) -> Result<serde_json::Value, String> {
    if let Some(sel) = get_champion_selections(build.champion_id)
        && let Some(idx) = sel.rune_page_index
        && idx < build.rune_pages.len()
    {
        let page = &build.rune_pages[idx];
        return build_rune_page_payload_from_parts(
            &build.champion_name,
            page.primary_style_id,
            page.sub_style_id,
            &page.selected_perk_ids,
        );
    }

    build_rune_page_payload_from_parts(
        &build.champion_name,
        build.primary_style_id,
        build.sub_style_id,
        &build.selected_perk_ids,
    )
}

/// Checks an array of rune pages and finds the first deletable custom page ID (if any).
/// Explicitly ignores Riot preset pages where `isDeletable == false`.
pub fn find_first_deletable_page_id(pages_json: &serde_json::Value) -> Option<i64> {
    let pages = pages_json.as_array()?;
    for page in pages {
        let is_deletable = page
            .get("isDeletable")
            .and_then(|d| d.as_bool())
            .unwrap_or(false);

        if is_deletable {
            return page.get("id").and_then(|id| id.as_i64());
        }
    }
    None
}

/// Checks all rune pages and finds the ID of an existing "schumaz.gg" page if already present.
/// Reusing and modifying the existing page in-place avoids creating multiple pages and hitting page limits.
pub fn find_schumaz_page_id(pages_json: &serde_json::Value) -> Option<i64> {
    let pages = pages_json.as_array()?;
    for page in pages {
        let is_deletable = page
            .get("isDeletable")
            .and_then(|d| d.as_bool())
            .unwrap_or(false);
        let name = page.get("name").and_then(|n| n.as_str()).unwrap_or("");

        if is_deletable && name.contains(SCHUMAZ_PAGE_SUFFIX) {
            return page.get("id").and_then(|id| id.as_i64());
        }
    }
    None
}

/// Checks an array of rune pages and finds any custom page ID that is deletable.
/// Explicitly ignores Riot presets (`isDeletable: false`).
pub fn extract_deletable_page_id_from_pages(pages_json: &serde_json::Value) -> Option<i64> {
    let pages = pages_json.as_array()?;
    for page in pages {
        let is_deletable = page
            .get("isDeletable")
            .and_then(|d| d.as_bool())
            .unwrap_or(false);

        if is_deletable {
            return page.get("id").and_then(|id| id.as_i64());
        }
    }
    None
}

/// Extracts a deletable page ID from the current page JSON response.
/// Returns None if the page is a locked Riot preset (isDeletable == false).
pub fn extract_deletable_rune_page_id(current_page_json: &serde_json::Value) -> Option<i64> {
    let is_deletable = current_page_json
        .get("isDeletable")
        .and_then(|d| d.as_bool())
        .unwrap_or(false);

    if is_deletable {
        current_page_json.get("id").and_then(|id| id.as_i64())
    } else {
        None
    }
}

/// Builds the JSON payload for updating summoner spells via PATCH /lol-champ-select/v1/session/my-selection.
pub fn build_spells_payload(spell1_id: i32, spell2_id: i32) -> Result<serde_json::Value, String> {
    if spell1_id <= 0 || spell2_id <= 0 {
        return Err("Summoner spell IDs must be positive integers".to_string());
    }

    if spell1_id == spell2_id {
        return Err("Summoner spell 1 and spell 2 cannot be identical".to_string());
    }

    Ok(json!({
        "spell1Id": spell1_id,
        "spell2Id": spell2_id
    }))
}

/// Builds the spells payload while applying the user's Flash key preference (D vs F).
pub fn build_spells_payload_with_preference(
    spell1_id: i32,
    spell2_id: i32,
    flash_key: FlashKeyPreference,
) -> Result<serde_json::Value, String> {
    let (s1, s2) = apply_flash_key_preference(spell1_id, spell2_id, flash_key);
    build_spells_payload(s1, s2)
}

/// Builds the spells payload applying mandatory Jungle Smite and Flash key preference.
pub fn build_spells_payload_with_jungle_and_preference(
    spell1_id: i32,
    spell2_id: i32,
    flash_key: FlashKeyPreference,
    is_jungle: bool,
) -> Result<serde_json::Value, String> {
    let (s1, s2) =
        apply_jungle_smite_and_flash_preference(spell1_id, spell2_id, flash_key, is_jungle);
    build_spells_payload(s1, s2)
}

/// Builds the JSON payload for custom item sets via PUT /lol-item-sets/v1/item-sets/{summonerId}/sets.
/// Sets the item set title as "%Champname% | schumaz.gg".
pub fn build_item_sets_payload(summoner_id: u64, build: &ChampionBuild) -> serde_json::Value {
    let mut blocks = Vec::new();

    for block in &build.item_blocks {
        let items: Vec<serde_json::Value> = block
            .item_ids
            .iter()
            .map(|id| {
                json!({
                    "id": id.to_string(),
                    "count": 1
                })
            })
            .collect();

        if !items.is_empty() {
            blocks.push(json!({
                "type": block.title,
                "items": items
            }));
        }
    }

    json!({
        "accountId": summoner_id,
        "itemSets": [
            {
                "title": format!("{}{}", build.champion_name, SCHUMAZ_PAGE_SUFFIX),
                "associatedChampions": [build.champion_id],
                "associatedMaps": [11, 12],
                "blocks": blocks,
                "mode": "any",
                "type": "custom"
            }
        ]
    })
}

/// Injects rune page into the LCU. Reuses and overwrites the single schumaz.gg rune page across matches
/// instead of creating a new page every match.
pub async fn inject_rune_payload(
    conn: &crate::lcu_connector::LcuConnection,
    payload: &serde_json::Value,
) -> Result<(), String> {
    // 1. Check all rune pages to see if a schumaz.gg page already exists
    if let Ok(pages) = conn.get_all_rune_pages().await {
        if let Some(existing_page_id) = find_schumaz_page_id(&pages) {
            // Overwrite the existing page in-place via PUT
            if conn.put_rune_page(existing_page_id, payload).await.is_ok() {
                return Ok(());
            }
            // If PUT fails, delete the existing schumaz page and recreate
            let _ = conn.delete_rune_page(existing_page_id).await;
            conn.create_rune_page(payload).await?;
            return Ok(());
        }

        // 2. No schumaz.gg page exists yet. Try creating a new one.
        match conn.create_rune_page(payload).await {
            Ok(_) => return Ok(()),
            Err(_) => {
                // If creation failed (e.g. max page limit reached), overwrite or delete the first custom page
                if let Some(deletable_id) = find_first_deletable_page_id(&pages) {
                    if conn.put_rune_page(deletable_id, payload).await.is_ok() {
                        return Ok(());
                    }
                    let _ = conn.delete_rune_page(deletable_id).await;
                    conn.create_rune_page(payload).await?;
                    return Ok(());
                }
            }
        }
    } else {
        // Fallback: direct create if pages endpoint query failed
        conn.create_rune_page(payload).await?;
    }

    Ok(())
}

/// Injects rune page into the LCU. Reuses and overwrites the single schumaz.gg rune page across matches
/// instead of creating a new page every match.
pub async fn inject_runes(
    conn: &crate::lcu_connector::LcuConnection,
    build: &ChampionBuild,
) -> Result<(), String> {
    let payload = build_rune_page_payload(build)?;
    inject_rune_payload(conn, &payload).await
}

/// Injects a specific chosen rune page directly into the LCU and saves the user choice in ChampionSelections.
pub async fn inject_specific_rune_page(
    conn: &crate::lcu_connector::LcuConnection,
    champion_id: i32,
    champion_name: &str,
    page_index: usize,
    primary_style_id: i32,
    sub_style_id: i32,
    selected_perk_ids: &[i32],
) -> Result<(), String> {
    set_selected_rune_page(champion_id, page_index);
    let payload = build_rune_page_payload_from_parts(
        champion_name,
        primary_style_id,
        sub_style_id,
        selected_perk_ids,
    )?;
    inject_rune_payload(conn, &payload).await
}

/// Injects summoner spells into the active Champion Select session adapting to the Flash key preference
/// and mandatory Jungle Smite rules.
pub async fn inject_spells(
    conn: &crate::lcu_connector::LcuConnection,
    build: &ChampionBuild,
    flash_key: FlashKeyPreference,
) -> Result<(), String> {
    let is_jungle = build.active_position.eq_ignore_ascii_case("jungle");
    let (mut s1, mut s2) = (build.spell1_id, build.spell2_id);

    if let Some(sel) = get_champion_selections(build.champion_id)
        && let Some(sec_spell) = sel.secondary_spell_id
    {
        if s1 == SPELL_FLASH_ID {
            s2 = sec_spell;
        } else if s2 == SPELL_FLASH_ID {
            s1 = sec_spell;
        } else {
            s2 = sec_spell;
        }
    }

    let payload = build_spells_payload_with_jungle_and_preference(
        s1,
        s2,
        flash_key,
        is_jungle,
    )?;
    conn.patch_summoner_spells(&payload).await?;
    Ok(())
}

/// Injects custom item sets for the summoner into the game client shop.
pub async fn inject_item_sets(
    conn: &crate::lcu_connector::LcuConnection,
    summoner_id: u64,
    build: &ChampionBuild,
) -> Result<(), String> {
    let payload = build_item_sets_payload(summoner_id, build);
    conn.put_item_sets(summoner_id, &payload).await?;
    Ok(())
}

/// High-level orchestrator injecting runes, summoner spells, and item sets respecting user preferences.
/// Produces a comprehensive report of all injection outcomes.
pub async fn inject_champion_build(
    conn: &crate::lcu_connector::LcuConnection,
    summoner_id: u64,
    build: &ChampionBuild,
) -> InjectionReport {
    let prefs = get_user_preferences();

    let runes_res = if prefs.auto_inject_runes {
        Some(inject_runes(conn, build).await)
    } else {
        None
    };

    let spells_res = if prefs.auto_inject_spells {
        Some(inject_spells(conn, build, prefs.flash_key).await)
    } else {
        None
    };

    let items_res = if prefs.auto_inject_items {
        Some(inject_item_sets(conn, summoner_id, build).await)
    } else {
        None
    };

    let runes_ok = runes_res.as_ref().is_some_and(|r| r.is_ok());
    let spells_ok = spells_res.as_ref().is_some_and(|r| r.is_ok());
    let items_ok = items_res.as_ref().is_some_and(|r| r.is_ok());

    let mut parts = Vec::new();
    if let Some(r) = runes_res {
        match r {
            Ok(_) => parts.push("Runas injetadas".to_string()),
            Err(e) => parts.push(format!("Falha nas Runas: {}", e)),
        }
    }
    if let Some(s) = spells_res {
        match s {
            Ok(_) => parts.push("Feitiços injetados".to_string()),
            Err(e) => parts.push(format!("Falha nos Feitiços: {}", e)),
        }
    }
    if let Some(i) = items_res {
        match i {
            Ok(_) => parts.push("Itens injetados".to_string()),
            Err(e) => parts.push(format!("Falha nos Itens: {}", e)),
        }
    }

    let message = if parts.is_empty() {
        "Nenhuma injeção configurada nas preferências".to_string()
    } else {
        parts.join(" | ")
    };

    InjectionReport {
        champion_name: build.champion_name.clone(),
        runes_injected: runes_ok,
        spells_injected: spells_ok,
        items_injected: items_ok,
        message,
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::build_provider::ItemBlock;

    #[test]
    fn test_apply_flash_key_preference_flash_on_d() {
        // Flash is already on D (spell1) -> stays (4, 14)
        assert_eq!(
            apply_flash_key_preference(4, 14, FlashKeyPreference::D),
            (4, 14)
        );

        // Flash is on F (spell2) -> inverted to D (4, 14)
        assert_eq!(
            apply_flash_key_preference(14, 4, FlashKeyPreference::D),
            (4, 14)
        );
    }

    #[test]
    fn test_apply_flash_key_preference_flash_on_f() {
        // Flash is on D (spell1) -> inverted to F (14, 4)
        assert_eq!(
            apply_flash_key_preference(4, 14, FlashKeyPreference::F),
            (14, 4)
        );

        // Flash is already on F (spell2) -> stays (14, 4)
        assert_eq!(
            apply_flash_key_preference(14, 4, FlashKeyPreference::F),
            (14, 4)
        );
    }

    #[test]
    fn test_apply_flash_key_preference_without_flash_preserves_order() {
        // Neither spell is Flash (e.g. Teleport 12 + Ignite 14) -> order preserved
        assert_eq!(
            apply_flash_key_preference(12, 14, FlashKeyPreference::D),
            (12, 14)
        );
        assert_eq!(
            apply_flash_key_preference(12, 14, FlashKeyPreference::F),
            (12, 14)
        );
    }

    #[test]
    fn test_jungle_smite_rule_and_flash_preference() {
        // Scenario 1: Jungle with Flash on D -> Flash (4) on D, Smite (11) on F
        assert_eq!(
            apply_jungle_smite_and_flash_preference(4, 14, FlashKeyPreference::D, true),
            (4, 11)
        );

        // Scenario 2: Jungle with Flash on F -> Smite (11) on D, Flash (4) on F
        assert_eq!(
            apply_jungle_smite_and_flash_preference(4, 14, FlashKeyPreference::F, true),
            (11, 4)
        );

        // Scenario 3: Jungle with already Smite + Flash
        assert_eq!(
            apply_jungle_smite_and_flash_preference(11, 4, FlashKeyPreference::D, true),
            (4, 11)
        );
        assert_eq!(
            apply_jungle_smite_and_flash_preference(11, 4, FlashKeyPreference::F, true),
            (11, 4)
        );

        // Scenario 4: Jungle without Flash (Ghost 6 + Ignite 14) -> Ghost 6 + Smite 11
        assert_eq!(
            apply_jungle_smite_and_flash_preference(6, 14, FlashKeyPreference::D, true),
            (6, 11)
        );

        // Scenario 5: Non-jungle ignores jungle rule and only applies standard flash preference
        assert_eq!(
            apply_jungle_smite_and_flash_preference(4, 14, FlashKeyPreference::F, false),
            (14, 4)
        );
    }

    #[test]
    fn test_build_spells_payload_with_flash_preference() {
        let payload = build_spells_payload_with_preference(4, 14, FlashKeyPreference::F)
            .expect("Payload should be valid");

        assert_eq!(payload.get("spell1Id").unwrap().as_i64().unwrap(), 14);
        assert_eq!(payload.get("spell2Id").unwrap().as_i64().unwrap(), 4);
    }

    #[test]
    fn test_user_preferences_global_sync() {
        let prefs = UserPreferences {
            auto_inject_runes: false,
            auto_inject_spells: true,
            auto_inject_items: false,
            flash_key: FlashKeyPreference::F,
        };

        set_user_preferences(prefs.clone());
        assert_eq!(get_user_preferences(), prefs);

        // Reset to default
        set_user_preferences(UserPreferences::default());
        assert_eq!(get_user_preferences(), UserPreferences::default());
    }

    #[test]
    fn test_build_rune_page_payload_validates_and_formats_correctly() {
        let valid_build = ChampionBuild {
            champion_id: 103,
            champion_name: "Ahri".to_string(),
            win_rate: 51.5,
            pick_rate: 10.2,
            primary_style_id: 8100,
            sub_style_id: 8200,
            selected_perk_ids: vec![8112, 8126, 8138, 8135, 8226, 8210, 5008, 5008, 5001],
            spell1_id: 4,
            spell2_id: 14,
            item_blocks: vec![],
            ..Default::default()
        };

        let payload = build_rune_page_payload(&valid_build).unwrap();
        assert_eq!(payload.get("name").unwrap(), "Ahri | schumaz.gg");
        assert_eq!(payload.get("primaryStyleId").unwrap(), 8100);
        assert_eq!(payload.get("subStyleId").unwrap(), 8200);
        assert_eq!(payload.get("current").unwrap(), true);

        let perks = payload.get("selectedPerkIds").unwrap().as_array().unwrap();
        assert_eq!(perks.len(), 9);

        // Invalid perk count must fail
        let mut invalid_build = valid_build.clone();
        invalid_build.selected_perk_ids.pop();
        assert!(build_rune_page_payload(&invalid_build).is_err());

        // Identical primary and sub styles must fail
        let mut same_style_build = valid_build.clone();
        same_style_build.sub_style_id = 8100;
        assert!(build_rune_page_payload(&same_style_build).is_err());
    }

    #[test]
    fn test_build_spells_payload_formats_and_prevents_duplicate_spells() {
        let payload = build_spells_payload(4, 14).unwrap();
        assert_eq!(payload.get("spell1Id").unwrap(), 4);
        assert_eq!(payload.get("spell2Id").unwrap(), 14);

        // Duplicate spell IDs must fail
        assert!(build_spells_payload(4, 4).is_err());

        // Non-positive IDs must fail
        assert!(build_spells_payload(0, 14).is_err());
    }

    #[test]
    fn test_find_schumaz_page_id_finds_page_and_ignores_presets() {
        let pages_json = serde_json::json!([
            { "id": 1, "name": "Riot Preset: Domination", "isDeletable": false },
            { "id": 2, "name": "Custom Runes", "isDeletable": true },
            { "id": 42, "name": "Ahri | schumaz.gg", "isDeletable": true }
        ]);

        assert_eq!(find_schumaz_page_id(&pages_json), Some(42));
    }

    #[test]
    fn test_find_schumaz_page_id_returns_none_when_no_schumaz_page_exists() {
        let pages_json = serde_json::json!([
            { "id": 1, "name": "Riot Preset: Domination", "isDeletable": false },
            { "id": 2, "name": "Custom Runes", "isDeletable": true }
        ]);

        assert_eq!(find_schumaz_page_id(&pages_json), None);
    }

    #[test]
    fn test_find_first_deletable_page_id_skips_riot_presets() {
        let pages_json = serde_json::json!([
            { "id": 1, "name": "Riot Preset 1", "isDeletable": false },
            { "id": 2, "name": "Riot Preset 2", "isDeletable": false },
            { "id": 55, "name": "Deletable Custom Slot", "isDeletable": true }
        ]);

        assert_eq!(find_first_deletable_page_id(&pages_json), Some(55));
    }

    #[test]
    fn test_extract_deletable_rune_page_id_respects_riot_preset_pages() {
        let riot_preset = serde_json::json!(
            { "id": 10, "name": "Preset: Inspiration", "isDeletable": false }
        );
        assert_eq!(extract_deletable_rune_page_id(&riot_preset), None);

        let custom_page = serde_json::json!(
            { "id": 88, "name": "My Custom Runes", "isDeletable": true }
        );
        assert_eq!(extract_deletable_rune_page_id(&custom_page), Some(88));
    }

    #[test]
    fn test_build_item_sets_payload_structure() {
        let build = ChampionBuild {
            champion_id: 222,
            champion_name: "Jinx".to_string(),
            item_blocks: vec![
                ItemBlock {
                    title: "Itens Iniciais".to_string(),
                    item_ids: vec![1055, 2003],
                    item_icon_urls: vec![],
                },
                ItemBlock {
                    title: "Primeiros 3 Itens".to_string(),
                    item_ids: vec![3031, 3006, 3094],
                    item_icon_urls: vec![],
                },
                ItemBlock {
                    title: "Build Final".to_string(),
                    item_ids: vec![3031, 3006, 3094, 3036, 3026],
                    item_icon_urls: vec![],
                },
                ItemBlock {
                    title: "3 Botas Mais Feitas".to_string(),
                    item_ids: vec![3006, 3047, 3111],
                    item_icon_urls: vec![],
                },
            ],
            ..Default::default()
        };

        let payload = build_item_sets_payload(123456789, &build);
        assert_eq!(payload.get("accountId").unwrap(), 123456789);

        let item_sets = payload.get("itemSets").unwrap().as_array().unwrap();
        assert_eq!(item_sets.len(), 1);

        let first_set = &item_sets[0];
        assert_eq!(
            first_set.get("title").unwrap().as_str().unwrap(),
            "Jinx | schumaz.gg"
        );
        assert_eq!(
            first_set
                .pointer("/associatedChampions/0")
                .unwrap()
                .as_i64()
                .unwrap(),
            222
        );

        let blocks = first_set.get("blocks").unwrap().as_array().unwrap();
        assert_eq!(blocks.len(), 4);
        assert_eq!(
            blocks[0].get("type").unwrap().as_str().unwrap(),
            "Itens Iniciais"
        );
        assert_eq!(
            blocks[1].get("type").unwrap().as_str().unwrap(),
            "Primeiros 3 Itens"
        );
        assert_eq!(
            blocks[2].get("type").unwrap().as_str().unwrap(),
            "Build Final"
        );
        assert_eq!(
            blocks[3].get("type").unwrap().as_str().unwrap(),
            "3 Botas Mais Feitas"
        );

        let first_item_id = blocks[0].pointer("/items/0/id").unwrap().as_str().unwrap();
        assert_eq!(first_item_id, "1055");
    }

    #[test]
    fn test_injection_report_serialization() {
        let report = InjectionReport {
            champion_name: "Ahri".to_string(),
            runes_injected: true,
            spells_injected: true,
            items_injected: false,
            message: "Runas injetadas | Feitiços injetados".to_string(),
        };

        let json = serde_json::to_string(&report).unwrap();
        let parsed: InjectionReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, parsed);
    }

    #[test]
    fn test_build_rune_page_payload_respects_champion_selections() {
        clear_champion_selections();
        let mut build = ChampionBuild {
            champion_id: 103,
            champion_name: "Ahri".to_string(),
            primary_style_id: 8000,
            sub_style_id: 8100,
            selected_perk_ids: vec![8005, 9101, 9104, 8299, 8126, 8135, 5008, 5008, 5002],
            ..Default::default()
        };
        build.rune_pages = vec![
            crate::build_provider::RunePageOption {
                primary_style_id: 8000,
                sub_style_id: 8100,
                selected_perk_ids: vec![8005, 9101, 9104, 8299, 8126, 8135, 5008, 5008, 5002],
                play: 5000,
                win: 2600,
                pick_rate: 65.0,
                ..Default::default()
            },
            crate::build_provider::RunePageOption {
                primary_style_id: 8200,
                sub_style_id: 8300,
                selected_perk_ids: vec![8214, 8224, 8210, 8237, 8304, 8345, 5008, 5008, 5001],
                play: 1200,
                win: 600,
                pick_rate: 15.0,
                ..Default::default()
            },
        ];

        // Default: index 0 (build.primary_style_id)
        let default_payload = build_rune_page_payload(&build).unwrap();
        assert_eq!(default_payload["primaryStyleId"], 8000);

        // Select page index 1
        set_selected_rune_page(build.champion_id, 1);
        let selected_payload = build_rune_page_payload(&build).unwrap();
        assert_eq!(selected_payload["primaryStyleId"], 8200);
        assert_eq!(selected_payload["subStyleId"], 8300);

        clear_champion_selections();
    }
    #[test]
    fn test_champion_selections_lock_in_persistence() {
        clear_champion_selections();
        assert_eq!(get_champion_selections(103), None);

        // User selected rune page index 1 and secondary spell 3 (Exhaust) for Ahri (103)
        set_selected_rune_page(103, 1);
        set_selected_secondary_spell(103, 3);

        let sel = get_champion_selections(103).expect("Selections should exist for champ 103");
        assert_eq!(sel.rune_page_index, Some(1));
        assert_eq!(sel.secondary_spell_id, Some(3));

        // Different champion query should return None
        assert_eq!(get_champion_selections(804), None);

        // Clearing selections resets back to None
        clear_champion_selections();
        assert_eq!(get_champion_selections(103), None);
    }
}
