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
