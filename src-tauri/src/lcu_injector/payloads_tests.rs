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
    let _test_guard = TEST_SELECTIONS_LOCK.blocking_lock();
    clear_champion_selections();

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
    let _test_guard = TEST_SELECTIONS_LOCK.blocking_lock();
    clear_champion_selections();

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
    let _test_guard = TEST_SELECTIONS_LOCK.blocking_lock();
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
    let _test_guard = TEST_SELECTIONS_LOCK.blocking_lock();
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
    let _test_guard = TEST_SELECTIONS_LOCK.blocking_lock();
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
            crate::build_provider::ItemBuildOption {
                item_ids: vec![3118, 4645, 3089],
                ..Default::default()
            },
            crate::build_provider::ItemBuildOption {
                item_ids: vec![3118, 3157, 3089],
                ..Default::default()
            },
        ],
        final_build_options: vec![
            crate::build_provider::ItemBuildOption {
                item_ids: vec![3118, 4645, 3089, 3157, 3135, 4629],
                ..Default::default()
            },
            crate::build_provider::ItemBuildOption {
                item_ids: vec![3118, 3157, 3089, 4645, 3135, 3102],
                ..Default::default()
            },
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
