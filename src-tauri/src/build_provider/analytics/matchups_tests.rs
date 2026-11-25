use super::*;
use crate::build_provider::analytics::rest_parser::parse_opgg_analysis_payload_for_mode;
use crate::build_provider::models::ChampionMetadata;
use serde_json::json;
use std::collections::{HashMap, HashSet};

fn minimal_rest_payload(counters: serde_json::Value) -> serde_json::Value {
    json!({
        "data": {
            "position": "mid",
            "runes": [{
                "primary_page_id": 8100,
                "primary_rune_ids": [8112, 8126, 8138, 8135],
                "secondary_page_id": 8200,
                "secondary_rune_ids": [8226, 8210],
                "stat_mod_ids": [5008, 5008, 5001],
                "play": 1000,
                "win": 520
            }],
            "core_items": [{ "ids": [3118, 4645, 3089], "play": 500, "win": 270 }],
            "counters": counters
        }
    })
}

#[test]
fn test_extract_rest_lane_matchups_splits_top5_hard_and_favorable_disjoint() {
    let counters = json!([
        { "champion_id": 1, "play": 1000, "win": 440 },
        { "champion_id": 2, "play": 800, "win": 360 },
        { "champion_id": 3, "play": 700, "win": 322 },
        { "champion_id": 4, "play": 600, "win": 282 },
        { "champion_id": 5, "play": 500, "win": 240 },
        { "champion_id": 6, "play": 500, "win": 250 },
        { "champion_id": 7, "play": 400, "win": 212 },
        { "champion_id": 8, "play": 300, "win": 165 },
        { "champion_id": 9, "play": 200, "win": 114 },
        { "champion_id": 10, "play": 250, "win": 148 },
        { "champion_id": 11, "play": 300, "win": 186 },
        { "champion_id": 12, "play": 5, "win": 0 },
        { "champion_id": 13, "play": 4, "win": 4 }
    ]);

    let summary = extract_rest_lane_matchups(&json!({ "counters": counters }), 103, "mid")
        .expect("should parse matchups");

    let hard_ids: Vec<i32> = summary.hard.iter().map(|e| e.champion_id).collect();
    let fav_ids: Vec<i32> = summary.favorable.iter().map(|e| e.champion_id).collect();
    assert_eq!(hard_ids, vec![1, 2, 3, 4, 5]);
    assert_eq!(fav_ids, vec![11, 10, 9, 8, 7]);
    assert_eq!(summary.hard[0].win_rate, 44.0);
    assert_eq!(summary.favorable[0].win_rate, 62.0);

    let hard_set: HashSet<i32> = hard_ids.into_iter().collect();
    assert!(fav_ids.iter().all(|id| !hard_set.contains(id)));
}

#[test]
fn test_extract_rest_lane_matchups_uses_summary_positions_fallback_for_hard_only() {
    let payload = json!({
        "summary": {
            "positions": [
                {
                    "name": "TOP",
                    "counters": [{ "champion_id": 86, "play": 500, "win": 210 }]
                },
                {
                    "name": "MID",
                    "counters": [
                        { "champion_id": 950, "play": 511, "win": 239 },
                        { "champion_id": 1, "play": 2761, "win": 1310 }
                    ]
                }
            ]
        }
    });

    let summary =
        extract_rest_lane_matchups(&payload, 103, "mid").expect("should use MID fallback");
    assert_eq!(summary.hard.len(), 2);
    assert_eq!(summary.hard[0].champion_id, 950);
    assert_eq!(summary.hard[0].win_rate, 46.8);
    assert_eq!(summary.hard[1].champion_id, 1);
    assert_eq!(summary.hard[1].win_rate, 47.4);
    assert!(summary.favorable.is_empty());
}

#[test]
fn test_extract_rest_lane_matchups_sanitizes_invalid_duplicates_and_small_lists() {
    let payload = json!({
        "counters": [
            { "champion_id": 103, "play": 500, "win": 100 },
            { "champion_id": -1, "play": 500, "win": 100 },
            { "champion_id": 55, "play": 0, "win": 0 },
            { "champion_id": 7, "play": 50, "win": 20 },
            { "champion_id": 7, "play": 200, "win": 120 },
            { "champion_id": 238, "play": 100, "win": 45 }
        ]
    });

    let summary =
        extract_rest_lane_matchups(&payload, 103, "mid").expect("should partition <10 entries");
    assert_eq!(summary.hard.len(), 1);
    assert_eq!(summary.hard[0].champion_id, 238);
    assert_eq!(summary.hard[0].win_rate, 45.0);
    assert_eq!(summary.favorable.len(), 1);
    assert_eq!(summary.favorable[0].champion_id, 7);
    assert_eq!(summary.favorable[0].play, 200);
    assert_eq!(summary.favorable[0].win_rate, 60.0);
    assert!(extract_rest_lane_matchups(&payload, 103, "none").is_none());
}

#[test]
fn test_rest_parser_skips_matchups_in_aram_and_cherry_modes() {
    let payload = minimal_rest_payload(json!([
        { "champion_id": 1, "play": 500, "win": 200 },
        { "champion_id": 2, "play": 500, "win": 300 }
    ]));

    let sr = parse_opgg_analysis_payload_for_mode(&payload, 103, "Ahri", true, Some("CLASSIC"), None)
        .expect("SR parse");
    assert!(sr.lane_matchups.is_some());

    for mode in ["ARAM", "KIWI", "CHERRY"] {
        let non_sr =
            parse_opgg_analysis_payload_for_mode(&payload, 103, "Ahri", true, Some(mode), None)
                .expect("non-SR parse");
        assert!(
            non_sr.lane_matchups.is_none(),
            "mode {mode} should omit lane_matchups"
        );
    }
}

#[test]
fn test_enrich_lane_matchups_with_ddragon_hydrates_names_and_fallbacks() {
    let mut build = ChampionBuild {
        champion_id: 103,
        champion_name: "Ahri".to_string(),
        lane_matchups: Some(LaneMatchupsSummary {
            hard: vec![LaneMatchupEntry {
                champion_id: 1,
                play: 100,
                win: 45,
                win_rate: 45.0,
                ..Default::default()
            }],
            favorable: vec![LaneMatchupEntry {
                champion_id: 9999,
                play: 80,
                win: 50,
                win_rate: 62.5,
                ..Default::default()
            }],
        }),
        ..Default::default()
    };

    let mut champions = HashMap::new();
    champions.insert(
        1,
        ChampionMetadata {
            champion_id: 1,
            key_name: "Annie".to_string(),
            display_name: "Annie".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/16.23.1/img/champion/Annie.png"
                .to_string(),
        },
    );
    let cache = DataDragonCache {
        version: "16.23.1".to_string(),
        champions,
        ..Default::default()
    };

    enrich_lane_matchups_with_ddragon(&mut build, &cache);
    let matchups = build.lane_matchups.expect("matchups");
    assert_eq!(matchups.hard[0].name, "Annie");
    assert_eq!(matchups.hard[0].key, "annie");
    assert!(matchups.hard[0].icon_url.contains("Annie.png"));
    assert_eq!(matchups.favorable[0].name, "Champion 9999");
}

#[test]
fn test_extract_rest_lane_matchups_for_roster_excludes_off_role_opponents() {
    let payload = json!({
        "counters": [
            { "champion_id": 950, "play": 511, "win": 239 }, // Naafiri off-role
            { "champion_id": 1, "play": 2761, "win": 1310 }, // Annie mid
            { "champion_id": 901, "play": 196, "win": 123 }, // Smolder off-role
            { "champion_id": 268, "play": 1835, "win": 1014 } // Azir mid
        ]
    });
    let mid_roster: HashSet<i32> = [1, 268].into_iter().collect();
    let summary = extract_rest_lane_matchups_for_roster(&payload, 103, "mid", Some(&mid_roster))
        .expect("should keep only mid roster");
    assert_eq!(summary.hard.iter().map(|m| m.champion_id).collect::<Vec<_>>(), vec![1]);
    assert_eq!(summary.favorable.iter().map(|m| m.champion_id).collect::<Vec<_>>(), vec![268]);
}
