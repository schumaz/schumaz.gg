use super::*;
use crate::build_provider::*;
use crate::lcu_injector::*;

#[test]
fn test_build_item_sets_payload_structure() {
    let _test_guard = TEST_SELECTIONS_LOCK.blocking_lock();
    clear_champion_selections();

    let build = ChampionBuild {
        champion_id: 222,
        champion_name: "Jinx".to_string(),
        item_blocks: vec![
            ItemBlock {
                kind: ItemBlockKind::Starter,
                title: "Itens Iniciais".to_string(),
                item_ids: vec![1055, 2003],
                item_icon_urls: vec![],
            },
            ItemBlock {
                kind: ItemBlockKind::Core,
                title: "Primeiros 3 Itens".to_string(),
                item_ids: vec![3031, 3006, 3094],
                item_icon_urls: vec![],
            },
            ItemBlock {
                kind: ItemBlockKind::Final,
                title: "Build Final".to_string(),
                item_ids: vec![3031, 3006, 3094, 3036, 3026],
                item_icon_urls: vec![],
            },
            ItemBlock {
                kind: ItemBlockKind::Boots,
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
    assert_eq!(
        first_set.get("associatedMaps").unwrap(),
        &serde_json::json!([11, 12, 30])
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
fn test_build_item_sets_payload_respects_unified_build_selection() {
    let _test_guard = TEST_SELECTIONS_LOCK.blocking_lock();
    clear_champion_selections();
    let build = ChampionBuild {
        champion_id: 103,
        champion_name: "Ahri".to_string(),
        item_blocks: vec![
            crate::build_provider::ItemBlock {
                kind: ItemBlockKind::Starter,
                title: "1. Itens Iniciais".to_string(),
                item_ids: vec![1056, 2003],
                item_icon_urls: vec![],
            },
            crate::build_provider::ItemBlock {
                kind: ItemBlockKind::Core,
                title: "2. Primeiros 3 Itens".to_string(),
                item_ids: vec![3118, 4645, 3089],
                item_icon_urls: vec![],
            },
            crate::build_provider::ItemBlock {
                kind: ItemBlockKind::Boots,
                title: "3. 3 Botas Mais Feitas".to_string(),
                item_ids: vec![3020, 3158, 3111],
                item_icon_urls: vec![],
            },
            crate::build_provider::ItemBlock {
                kind: ItemBlockKind::Final,
                title: "4. Build Final".to_string(),
                item_ids: vec![3118, 4645, 3089, 3157, 3135, 4629],
                item_icon_urls: vec![],
            },
            crate::build_provider::ItemBlock {
                kind: ItemBlockKind::Situational,
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
fn test_build_item_sets_payload_uses_typed_block_kind_regardless_of_title() {
    let _test_guard = TEST_SELECTIONS_LOCK.blocking_lock();
    clear_champion_selections();
    let build = ChampionBuild {
        champion_id: 103,
        champion_name: "Ahri".to_string(),
        item_blocks: vec![
            crate::build_provider::ItemBlock {
                kind: ItemBlockKind::Starter,
                title: "Custom Starter".to_string(),
                item_ids: vec![1056, 2003],
                item_icon_urls: vec![],
            },
            crate::build_provider::ItemBlock {
                kind: ItemBlockKind::Core,
                title: "Custom Core".to_string(),
                item_ids: vec![3118, 4645, 3089],
                item_icon_urls: vec![],
            },
            crate::build_provider::ItemBlock {
                kind: ItemBlockKind::Boots,
                title: "Custom Footwear".to_string(),
                item_ids: vec![3020, 3158, 3111],
                item_icon_urls: vec![],
            },
            crate::build_provider::ItemBlock {
                kind: ItemBlockKind::Final,
                title: "Custom Endgame".to_string(),
                item_ids: vec![3118, 4645, 3089, 3157, 3135, 4629],
                item_icon_urls: vec![],
            },
            crate::build_provider::ItemBlock {
                kind: ItemBlockKind::Situational,
                title: "Custom Tech".to_string(),
                item_ids: vec![3102, 3165],
                item_icon_urls: vec![],
            },
        ],
        build_options: vec![
            crate::build_provider::UnifiedBuildOption {
                name: "Build 1 (Most Popular)".to_string(),
                core_item_ids: vec![3118, 4645, 3089],
                final_item_ids: vec![3118, 4645, 3089, 3157, 3135, 4629],
                situational_item_ids: vec![3102, 3165],
                ..Default::default()
            },
            crate::build_provider::UnifiedBuildOption {
                name: "Build 2".to_string(),
                core_item_ids: vec![3157, 3089, 4629],
                final_item_ids: vec![3157, 3089, 4629, 3118, 3135, 3102],
                situational_item_ids: vec![4646, 6653],
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    set_selected_build(103, 1);
    let payload = build_item_sets_payload(12345, &build);
    let blocks = payload["itemSets"][0]["blocks"].as_array().unwrap();
    assert_eq!(blocks[1]["items"][0]["id"], "3157");
    assert_eq!(blocks[3]["items"][5]["id"], "3102");
    assert_eq!(blocks[4]["items"][0]["id"], "4646");

    clear_champion_selections();
}
