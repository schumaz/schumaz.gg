use super::*;
use crate::build_provider::*;

fn make_rest_payload(
    position: &str,
    core_ids: Vec<i32>,
    fourth_ids: Vec<i32>,
    fifth_ids: Vec<i32>,
    sixth_ids: Vec<i32>,
    boots_id: i32,
) -> serde_json::Value {
    serde_json::json!({
        "summary": { "position": position },
        "core_items": [
            { "ids": core_ids, "play": 18000, "win": 9500, "pick_rate": 55.0 }
        ],
        "boots": [
            { "ids": [boots_id], "play": 25000, "win": 13500, "pick_rate": 75.0 }
        ],
        "starters": [
            { "ids": [3070, 2003], "play": 28000, "win": 15000, "pick_rate": 90.0 }
        ],
        "fourth_items": fourth_ids.into_iter().map(|id| serde_json::json!({ "ids": [id], "play": 12000, "win": 6400, "pick_rate": 40.0 })).collect::<Vec<_>>(),
        "fifth_items": fifth_ids.into_iter().map(|id| serde_json::json!({ "ids": [id], "play": 8000, "win": 4300, "pick_rate": 30.0 })).collect::<Vec<_>>(),
        "sixth_items": sixth_ids.into_iter().map(|id| serde_json::json!({ "ids": [id], "play": 5000, "win": 2800, "pick_rate": 20.0 })).collect::<Vec<_>>(),
        "runes": [
            {
                "primary_page_id": 8200,
                "primary_rune_ids": [8214, 8226, 8210, 8237],
                "secondary_page_id": 8300,
                "secondary_rune_ids": [8304, 8345],
                "stat_mod_ids": [5008, 5008, 5001],
                "play": 22000,
                "win": 12000
            }
        ],
        "spells": [
            { "ids": [4, 7], "play": 25000, "win": 13500, "pick_rate": 85.0 }
        ]
    })
}

#[test]
fn test_resolve_tear_item_ids_preserves_order_and_handles_variants() {
    // Rule 1: combination already includes evolved Tear -> drop 3070/323070/773070 and preserve completion slot
    assert_eq!(
        resolve_tear_item_ids(&[3070, 3078, 3042, 3161], Some(3004)),
        vec![3078, 3004, 3161]
    ); // Ezreal
    assert_eq!(
        resolve_tear_item_ids(&[3070, 6617, 2530, 3504], Some(2526)),
        vec![6617, 2526, 3504]
    ); // Lulu
    assert_eq!(
        resolve_tear_item_ids(&[3070, 6657, 3040, 3089], Some(3003)),
        vec![6657, 3003, 3089]
    ); // Veigar
    assert_eq!(
        resolve_tear_item_ids(&[323070, 126697, 3042, 6699], Some(3004)),
        vec![126697, 3004, 6699]
    ); // Jayce with mode Tear variant 323070

    // Rule 2: 3 completed items without evolving Tear -> drop raw Tear without inventing or displacing 3rd item
    assert_eq!(
        resolve_tear_item_ids(&[773070, 6655, 4645, 3089], Some(3003)),
        vec![6655, 4645, 3089]
    );

    // Rule 3: < 3 completed items + raw Tear + confirmed payload upgrade -> append after completed legendaries
    assert_eq!(
        resolve_tear_item_ids(&[3070, 6617, 3107], Some(2526)),
        vec![6617, 3107, 2526]
    );
    assert_eq!(
        resolve_tear_item_ids(&[6617, 3001, 3070], Some(2526)),
        vec![6617, 3001, 2526]
    );

    // No confirmed Tear evolution in payload -> drop raw Tear, never invent
    assert_eq!(
        resolve_tear_item_ids(&[3070, 6655, 4645], None),
        vec![6655, 4645]
    );

    // Single raw Tear depth entry -> discarded even when champion has a Tear evolution
    assert_eq!(
        resolve_tear_item_ids(&[3070], Some(3004)),
        Vec::<i32>::new()
    );
}

#[test]
fn test_real_champions_preserve_actual_tear_completion_slot() {
    // Ezreal: [3070, 3078, 3042, 3161] -> Trinity Force 1st, Manamune 2nd, Shojin 3rd
    let ezreal_json = make_rest_payload(
        "adc",
        vec![3070, 3078, 3042, 3161],
        vec![3036],
        vec![3072],
        vec![3026],
        3158,
    );
    let ezreal = parse_opgg_analysis_payload(&ezreal_json, 81, "Ezreal").unwrap();
    assert_eq!(ezreal.item_blocks[1].item_ids, vec![3078, 3004, 3161]);
    assert_eq!(
        ezreal.item_blocks[3].item_ids,
        vec![3078, 3004, 3161, 3036, 3072, 3026]
    );
    assert_eq!(
        ezreal.build_options[0].core_item_ids,
        vec![3078, 3004, 3161]
    );
    assert_eq!(
        ezreal.build_options[0].final_item_ids,
        vec![3078, 3004, 3161, 3036, 3072, 3026]
    );

    // Lulu: [3070, 6617, 2530, 3504] -> Moonstone 1st, Whispering Circlet 2nd, Ardent 3rd
    let lulu_json = make_rest_payload(
        "support",
        vec![3070, 6617, 2530, 3504],
        vec![3107],
        vec![3222],
        vec![3157],
        3158,
    );
    let lulu = parse_opgg_analysis_payload(&lulu_json, 117, "Lulu").unwrap();
    assert_eq!(lulu.item_blocks[1].item_ids, vec![6617, 2526, 3504]);
    assert_eq!(lulu.build_options[0].core_item_ids, vec![6617, 2526, 3504]);
    assert_eq!(lulu.item_blocks[3].item_ids[..3], [6617, 2526, 3504]);

    // Veigar: [3070, 6657, 3040, 3089] -> Rod of Ages 1st, Archangel's Staff 2nd, Rabadon 3rd
    let veigar_json = make_rest_payload(
        "mid",
        vec![3070, 6657, 3040, 3089],
        vec![3135],
        vec![3157],
        vec![3102],
        3020,
    );
    let veigar = parse_opgg_analysis_payload(&veigar_json, 45, "Veigar").unwrap();
    assert_eq!(veigar.item_blocks[1].item_ids, vec![6657, 3003, 3089]);
    assert_eq!(
        veigar.build_options[0].core_item_ids,
        vec![6657, 3003, 3089]
    );
    assert_eq!(veigar.item_blocks[3].item_ids[..3], [6657, 3003, 3089]);

    // Jayce: [3070, 126697, 3042, 6699] -> 126697 1st, Manamune 2nd, Voltaic 3rd
    let jayce_json = make_rest_payload(
        "top",
        vec![3070, 126697, 3042, 6699],
        vec![6694],
        vec![3814],
        vec![3026],
        3158,
    );
    let jayce = parse_opgg_analysis_payload(&jayce_json, 126, "Jayce").unwrap();
    assert_eq!(jayce.item_blocks[1].item_ids, vec![126697, 3004, 6699]);
    assert_eq!(
        jayce.build_options[0].core_item_ids,
        vec![126697, 3004, 6699]
    );
    assert_eq!(jayce.item_blocks[3].item_ids[..3], [126697, 3004, 6699]);
}

#[test]
fn test_lulu_short_core_with_early_tear_appends_confirmed_upgrade_after_legendaries() {
    // [3070, 6617, 3107] with 2530 in fourth_items -> [6617, 3107, 2526], never forcing slot 1!
    let lulu_short_json = make_rest_payload(
        "support",
        vec![3070, 6617, 3107],
        vec![2530],
        vec![3504],
        vec![3222],
        3158,
    );
    let build = parse_opgg_analysis_payload(&lulu_short_json, 117, "Lulu").unwrap();
    assert_eq!(build.item_blocks[1].item_ids, vec![6617, 3107, 2526]);
    assert_eq!(build.build_options[0].core_item_ids, vec![6617, 3107, 2526]);
    assert_eq!(build.item_blocks[3].item_ids[..3], [6617, 3107, 2526]);
    assert_eq!(
        build.item_blocks[3]
            .item_ids
            .iter()
            .filter(|&&id| id == 2526)
            .count(),
        1
    );
}

#[test]
fn test_mana_only_tear_never_invents_upgrade_or_displaces_third_item() {
    // Case A: 3 completed items + raw Tear (even if a Tear upgrade appears in sixth_items)
    let mana_only_full_json = make_rest_payload(
        "mid",
        vec![3070, 6655, 4645, 3089],
        vec![3157],
        vec![3135],
        vec![3040],
        3020,
    );
    let build_full = parse_opgg_analysis_payload(&mana_only_full_json, 99, "Lux").unwrap();
    assert_eq!(build_full.item_blocks[1].item_ids, vec![6655, 4645, 3089]);
    assert_eq!(
        build_full.build_options[0].core_item_ids,
        vec![6655, 4645, 3089]
    );
    assert_eq!(
        build_full.item_blocks[3].item_ids,
        vec![6655, 4645, 3089, 3157, 3135, 3003]
    );

    // Case B: < 3 completed items + raw Tear, NO Tear evolution anywhere in payload -> never invent 3003
    let mana_only_short_json = make_rest_payload(
        "mid",
        vec![3070, 6655, 4645],
        vec![3089],
        vec![3157],
        vec![3135],
        3020,
    );
    let build_short = parse_opgg_analysis_payload(&mana_only_short_json, 99, "Lux").unwrap();
    assert_eq!(build_short.item_blocks[1].item_ids, vec![6655, 4645, 3089]);
    assert_eq!(
        build_short.build_options[0].core_item_ids,
        vec![6655, 4645, 3089]
    );
    for block in &build_short.item_blocks[1..] {
        assert!(!block.item_ids.contains(&3070));
        assert!(!block.item_ids.contains(&3003));
        assert!(!block.item_ids.contains(&3004));
        assert!(!block.item_ids.contains(&2526));
        assert!(!block.item_ids.contains(&3119));
    }
    for opt in &build_short.build_options {
        assert!(!opt.core_item_ids.contains(&3003));
        assert!(!opt.final_item_ids.contains(&3003));
        assert!(!opt.situational_item_ids.contains(&3003));
    }
}

#[test]
fn test_direct_blocks_and_mcp_parser_preserve_tear_order() {
    // Direct blocks + unified options call with raw un-normalized entries (no prior rest_parser step)
    let core_entries = vec![ItemStatEntry {
        ids: vec![3070, 3078, 3042, 3161],
        play: 12000,
        win: 6300,
        pick_rate: 45.0,
    }];
    let fourth = vec![
        ItemStatEntry {
            ids: vec![3070],
            play: 9000,
            win: 4500,
            pick_rate: 30.0,
        },
        ItemStatEntry {
            ids: vec![3036],
            play: 5000,
            win: 2700,
            pick_rate: 20.0,
        },
    ];
    let fifth = vec![ItemStatEntry {
        ids: vec![3072],
        play: 4000,
        win: 2200,
        pick_rate: 15.0,
    }];
    let sixth = vec![ItemStatEntry {
        ids: vec![3026],
        play: 3000,
        win: 1700,
        pick_rate: 12.0,
    }];
    let blocks = build_5_item_blocks_with_depth_lists(
        vec![ItemStatEntry {
            ids: vec![3070, 2003],
            play: 20000,
            win: 10000,
            pick_rate: 70.0,
        }],
        vec![ItemStatEntry {
            ids: vec![3158],
            play: 20000,
            win: 10000,
            pick_rate: 70.0,
        }],
        core_entries.clone(),
        fourth.clone(),
        fifth.clone(),
        sixth.clone(),
        "AD",
        "adc",
    );
    assert_eq!(blocks[0].item_ids, vec![3070, 2003]);
    assert_eq!(blocks[1].item_ids, vec![3078, 3004, 3161]);
    assert_eq!(blocks[3].item_ids, vec![3078, 3004, 3161, 3036, 3072, 3026]);

    let options = build_unified_build_options(
        &core_entries,
        &fourth,
        &fifth,
        &sixth,
        &core_entries,
        &[],
        &blocks[4].item_ids,
        "AD",
        "adc",
    );
    assert_eq!(options[0].core_item_ids, vec![3078, 3004, 3161]);
    assert_eq!(
        options[0].final_item_ids,
        vec![3078, 3004, 3161, 3036, 3072, 3026]
    );

    // MCP parser path with Ezreal early Tear + Muramana
    let mcp_text = "class LolGetChampionAnalysis: data\n\
class Data: summary,core_items,boots,starter_items,fourth_items,fifth_items,sixth_items,summoner_spells,runes\n\
LolGetChampionAnalysis(Data(Summary(AverageStats(0.51,0.12)),CoreItems([3070,3078,3042,3161]),CoreItems([3158]),CoreItems([3070,2003]),[CoreItems([3036])],[CoreItems([3072])],[CoreItems([3026])],CoreItems([4,7]),Runes(8000,[8005,9101,9104,8014],8300,[8304,8345],[5005,5008,5001])))";
    let mcp_build = parse_opgg_mcp_text(mcp_text, 81, "Ezreal").unwrap();
    assert_eq!(mcp_build.item_blocks[1].item_ids, vec![3078, 3004, 3161]);
    assert_eq!(
        mcp_build.build_options[0].core_item_ids,
        vec![3078, 3004, 3161]
    );
}
