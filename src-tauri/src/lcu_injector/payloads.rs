use crate::build_provider::ChampionBuild;
use serde_json::json;
use super::preferences::FlashKeyPreference;
use super::selections::get_champion_selections;
use super::spell_rules::{apply_flash_key_preference, apply_jungle_smite_and_flash_preference};

pub const SCHUMAZ_PAGE_SUFFIX: &str = " | schumaz.gg";

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
    let selections = get_champion_selections(build.champion_id);
    let mut blocks = Vec::new();

    for block in &build.item_blocks {
        let mut item_ids = block.item_ids.clone();

        // If unified build option is selected, use it for both Primeiros 3 Itens and Build Final
        if let Some(sel) = selections.as_ref()
            && let Some(b_idx) = sel.build_index
            && b_idx < build.build_options.len()
        {
            if block.title.contains("Primeiros 3 Itens") {
                item_ids = build.build_options[b_idx].core_item_ids.clone();
            } else if block.title.contains("Build Final") {
                item_ids = build.build_options[b_idx].final_item_ids.clone();
            } else if (block.title.contains("Itens Situacionais")
                || block.title.contains("Úteis")
                || block.title.contains("Uteis"))
                && !build.build_options[b_idx].situational_item_ids.is_empty()
            {
                item_ids = build.build_options[b_idx].situational_item_ids.clone();
            }
        } else {
            // Fallback for independent core / final build options
            if block.title.contains("Primeiros 3 Itens")
                && let Some(sel) = selections.as_ref()
                && let Some(core_idx) = sel.core_build_index
                && core_idx < build.core_build_options.len()
            {
                item_ids = build.core_build_options[core_idx].item_ids.clone();
            } else if block.title.contains("Build Final")
                && let Some(sel) = selections.as_ref()
                && let Some(final_idx) = sel.final_build_index
                && final_idx < build.final_build_options.len()
            {
                item_ids = build.final_build_options[final_idx].item_ids.clone();
            }
        }

        let items: Vec<serde_json::Value> = item_ids
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

#[cfg(test)]
#[path = "payloads_tests.rs"]
mod tests;
