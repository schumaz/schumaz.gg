use super::*;

#[test]
fn test_build_lobby_slot_perks_string_supports_standard_and_legacy_formats() {
    let perks = [8112, 8143, 8138, 8106, 8210, 8236, 5008, 5008, 5001];

    let std_str = build_lobby_slot_perks_string(8100, 8200, &perks, None).unwrap();
    let std_val: serde_json::Value = serde_json::from_str(&std_str).unwrap();
    assert_eq!(std_val["perkStyle"], 8100);
    assert_eq!(std_val["perkSubStyle"], 8200);
    assert_eq!(std_val["perkIds"], serde_json::json!(perks));

    let legacy_existing = r#"{"primaryStyleId":8000,"subStyleId":8100,"selectedPerkIds":[8005,9111,9104,8014,8139,8135,5008,5008,5001]}"#;
    let legacy_str =
        build_lobby_slot_perks_string(8100, 8200, &perks, Some(legacy_existing)).unwrap();
    let legacy_val: serde_json::Value = serde_json::from_str(&legacy_str).unwrap();
    assert_eq!(legacy_val["primaryStyleId"], 8100);
    assert_eq!(legacy_val["subStyleId"], 8200);
    assert_eq!(legacy_val["selectedPerkIds"], serde_json::json!(perks));

    assert!(build_lobby_slot_perks_string(8100, 8100, &perks, None).is_err());
    assert!(build_lobby_slot_perks_string(8100, 8200, &[8112, 8143], None).is_err());
}

#[test]
fn test_patch_lobby_player_slots_preserves_metadata_and_detects_noops() {
    let raw_slots = serde_json::json!([
        {
            "championId": 103,
            "positionPreference": "MIDDLE",
            "skinId": 103015,
            "spell1": 4,
            "spell2": 12,
            "perks": "{\"perkIds\":[8112,8143,8138,8106,8210,8236,5008,5008,5001],\"perkStyle\":8100,\"perkSubStyle\":8200}"
        },
        {
            "championId": 222,
            "positionPreference": "BOTTOM",
            "skinId": 222001,
            "spell1": 4,
            "spell2": 7,
            "perks": ""
        },
        {
            "championId": 0,
            "positionPreference": "FILL"
        }
    ]);

    let ahri_perks = vec![8112, 8143, 8138, 8106, 8210, 8236, 5008, 5008, 5001];
    let jinx_perks = vec![8008, 9111, 9104, 8014, 8233, 8236, 5005, 5008, 5001];
    let patches = vec![
        LobbySlotPatch {
            slot_index: Some(0),
            champion_id: 103,
            runes: Some((8100, 8200, ahri_perks)),
            spells: Some((4, 14)),
        },
        LobbySlotPatch {
            slot_index: Some(1),
            champion_id: 222,
            runes: Some((8000, 8200, jinx_perks)),
            spells: Some((4, 21)),
        },
    ];

    let (patched, changed) = patch_lobby_player_slots_value(&raw_slots, &patches).unwrap();
    assert!(changed);
    assert_eq!(patched[0]["skinId"], 103015);
    assert_eq!(patched[0]["positionPreference"], "MIDDLE");
    assert_eq!(patched[0]["spell1"], 4);
    assert_eq!(patched[0]["spell2"], 14);
    assert_eq!(patched[1]["skinId"], 222001);
    assert_eq!(patched[1]["spell2"], 21);
    assert_eq!(patched[2]["championId"], 0);

    // Re-applying identical patches (even with reordered JSON keys in existing `perks`) must be a no-op
    let mut reordered_existing = patched.clone();
    reordered_existing[0]["perks"] = serde_json::json!(
        "{\"perkSubStyle\":8200, \"perkStyle\":8100, \"perkIds\":[8112,8143,8138,8106,8210,8236,5008,5008,5001]}"
    );
    let (_again, changed_again) = patch_lobby_player_slots_value(&reordered_existing, &patches).unwrap();
    assert!(!changed_again);
}
