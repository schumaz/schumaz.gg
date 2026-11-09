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
mod tests {
    use super::*;
    use crate::build_provider::*;
    use crate::lcu_injector::*;

    #[test]
    fn test_build_spells_payload_with_flash_preference() {
        let payload = build_spells_payload_with_preference(4, 14, FlashKeyPreference::F)
            .expect("Payload should be valid");

        assert_eq!(payload.get("spell1Id").unwrap().as_i64().unwrap(), 14);
        assert_eq!(payload.get("spell2Id").unwrap().as_i64().unwrap(), 4);
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
    fn test_build_item_sets_payload_respects_unified_build_selection() {
        clear_champion_selections();
        let build = ChampionBuild {
            champion_id: 103,
            champion_name: "Ahri".to_string(),
            item_blocks: vec![
                crate::build_provider::ItemBlock {
                    title: "1. Itens Iniciais".to_string(),
                    item_ids: vec![1056, 2003],
                    item_icon_urls: vec![],
                },
                crate::build_provider::ItemBlock {
                    title: "2. Primeiros 3 Itens".to_string(),
                    item_ids: vec![3118, 4645, 3089],
                    item_icon_urls: vec![],
                },
                crate::build_provider::ItemBlock {
                    title: "3. 3 Botas Mais Feitas".to_string(),
                    item_ids: vec![3020, 3158, 3111],
                    item_icon_urls: vec![],
                },
                crate::build_provider::ItemBlock {
                    title: "4. Build Final".to_string(),
                    item_ids: vec![3118, 4645, 3089, 3157, 3135, 4629],
                    item_icon_urls: vec![],
                },
                crate::build_provider::ItemBlock {
                    title: "5. Itens Situacionais / Úteis".to_string(),
                    item_ids: vec![3102, 3165],
                    item_icon_urls: vec![],
                },
            ],
            build_options: vec![
                crate::build_provider::UnifiedBuildOption {
                    name: "Build 1 (Mais Feita)".to_string(),
                    core_item_ids: vec![3118, 4645, 3089],
                    final_item_ids: vec![3118, 4645, 3089, 3157, 3135, 4629],
                    situational_item_ids: vec![3102, 3165],
                    ..Default::default()
                },
                crate::build_provider::UnifiedBuildOption {
                    name: "Build 2".to_string(),
                    core_item_ids: vec![3157, 3089, 4629],
                    final_item_ids: vec![3157, 3089, 4629, 3118, 3135, 3102],
                    situational_item_ids: vec![4646, 6653, 3165],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        // Select Unified Build Option 1 (index 1)
        set_selected_build(103, 1);

        let p = build_item_sets_payload(12345, &build);
        let blocks = p["itemSets"][0]["blocks"].as_array().unwrap();
        // Block 2: 3157, 3089, 4629
        assert_eq!(blocks[1]["items"][0]["id"], "3157");
        assert_eq!(blocks[1]["items"][1]["id"], "3089");
        assert_eq!(blocks[1]["items"][2]["id"], "4629");

        // Block 4: 3157, 3089, 4629, 3118, 3135, 3102
        assert_eq!(blocks[3]["items"][0]["id"], "3157");
        assert_eq!(blocks[3]["items"][5]["id"], "3102");

        // Block 5: Situational items for Option 2 -> 4646, 6653, 3165
        assert_eq!(blocks[4]["items"][0]["id"], "4646");
        assert_eq!(blocks[4]["items"][1]["id"], "6653");
        assert_eq!(blocks[4]["items"][2]["id"], "3165");

        clear_champion_selections();
    }

    #[test]
    fn test_build_item_sets_payload_respects_core_and_final_selections() {
        clear_champion_selections();
        let build = ChampionBuild {
            champion_id: 103,
            champion_name: "Ahri".to_string(),
            item_blocks: vec![
                crate::build_provider::ItemBlock {
                    title: "1. Itens Iniciais".to_string(),
                    item_ids: vec![1056, 2003],
                    item_icon_urls: vec![],
                },
                crate::build_provider::ItemBlock {
                    title: "2. Primeiros 3 Itens".to_string(),
                    item_ids: vec![3118, 4645, 3089],
                    item_icon_urls: vec![],
                },
                crate::build_provider::ItemBlock {
                    title: "3. 3 Botas Mais Feitas".to_string(),
                    item_ids: vec![3020, 3158, 3111],
                    item_icon_urls: vec![],
                },
                crate::build_provider::ItemBlock {
                    title: "4. Build Final".to_string(),
                    item_ids: vec![3118, 4645, 3089, 3157, 3135, 4629],
                    item_icon_urls: vec![],
                },
            ],
            core_build_options: vec![
                crate::build_provider::ItemBuildOption { item_ids: vec![3118, 4645, 3089], ..Default::default() },
                crate::build_provider::ItemBuildOption { item_ids: vec![3118, 3157, 3089], ..Default::default() },
            ],
            final_build_options: vec![
                crate::build_provider::ItemBuildOption { item_ids: vec![3118, 4645, 3089, 3157, 3135, 4629], ..Default::default() },
                crate::build_provider::ItemBuildOption { item_ids: vec![3118, 3157, 3089, 4645, 3135, 3102], ..Default::default() },
            ],
            ..Default::default()
        };

        // 1. Default payload
        let p1 = build_item_sets_payload(12345, &build);
        let blocks1 = p1["itemSets"][0]["blocks"].as_array().unwrap();
        assert_eq!(blocks1[1]["items"][1]["id"], "4645");

        // 2. Select Core Option 1 (index 1) and Final Option 1 (index 1)
        set_selected_core_build(103, 1);
        set_selected_final_build(103, 1);

        let p2 = build_item_sets_payload(12345, &build);
        let blocks2 = p2["itemSets"][0]["blocks"].as_array().unwrap();
        // Core items now reflect option 1: 3118, 3157, 3089
        assert_eq!(blocks2[1]["items"][1]["id"], "3157");
        // Final items now reflect option 1
        assert_eq!(blocks2[3]["items"][5]["id"], "3102");

        clear_champion_selections();
    }
}
