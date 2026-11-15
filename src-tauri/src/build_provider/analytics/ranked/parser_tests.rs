use super::*;
use crate::models::{ChampionMetadata, RankedLane};
use std::collections::HashMap;

fn test_ddragon() -> DataDragonCache {
    let mut champions = HashMap::new();
    champions.insert(
        103,
        ChampionMetadata {
            champion_id: 103,
            key_name: "Ahri".to_string(),
            display_name: "Ahri".to_string(),
            icon_url: "https://ddragon.example/Ahri.png".to_string(),
        },
    );
    champions.insert(
        245,
        ChampionMetadata {
            champion_id: 245,
            key_name: "Ekko".to_string(),
            display_name: "Ekko".to_string(),
            icon_url: "https://ddragon.example/Ekko.png".to_string(),
        },
    );
    champions.insert(
        86,
        ChampionMetadata {
            champion_id: 86,
            key_name: "Garen".to_string(),
            display_name: "Garen".to_string(),
            icon_url: "https://ddragon.example/Garen.png".to_string(),
        },
    );
    DataDragonCache {
        version: "16.22.1".to_string(),
        champions,
        runes: HashMap::new(),
        spells: HashMap::new(),
    }
}

#[test]
fn test_parse_ranked_tier_list_normalizes_tiers_and_assigns_cross_lane_overall_rank() {
    let json = serde_json::json!({
        "data": [
            {
                "id": 103,
                "positions": [
                    {
                        "name": "MID",
                        "stats": {
                            "win_rate": 0.512,
                            "pick_rate": 0.115,
                            "ban_rate": 0.036,
                            "role_rate": 0.94,
                            "tier_data": { "tier": 1, "rank": 1 }
                        }
                    }
                ]
            },
            {
                "id": 245,
                "positions": [
                    {
                        "name": "MID",
                        "stats": {
                            "win_rate": 0.522,
                            "pick_rate": 0.053,
                            "ban_rate": 0.034,
                            "role_rate": 0.45,
                            "tier_data": { "tier": 2, "rank": 4 }
                        }
                    },
                    {
                        "name": "JUNGLE",
                        "stats": {
                            "win_rate": 0.508,
                            "pick_rate": 0.061,
                            "ban_rate": 0.034,
                            "role_rate": 0.55,
                            "tier_data": { "tier": 3, "rank": 12 }
                        }
                    }
                ]
            },
            {
                "id": 86,
                "positions": [
                    {
                        "name": "TOP",
                        "stats": {
                            "win_rate": 0.507,
                            "pick_rate": 0.066,
                            "ban_rate": 0.044,
                            "role_rate": 0.89,
                            "tier_data": { "tier": 1, "rank": 3 }
                        }
                    }
                ]
            }
        ]
    });

    let list = parse_ranked_tier_list(&json, &test_ddragon()).unwrap();
    assert_eq!(list.len(), 4);

    assert_eq!(list[0].lane, RankedLane::Mid);
    assert_eq!(list[0].name, "Ahri");
    assert_eq!(list[0].tier, 0);
    assert_eq!(list[0].rank, 1);
    assert_eq!(list[0].overall_rank, 1);
    assert_eq!(list[0].pick_rate, Some(11.5));
    assert_eq!(list[0].ban_rate, Some(3.6));

    assert_eq!(list[1].lane, RankedLane::Top);
    assert_eq!(list[1].name, "Garen");
    assert_eq!(list[1].tier, 0);
    assert_eq!(list[1].rank, 3);
    assert_eq!(list[1].overall_rank, 2);
    assert_eq!(list[1].win_rate, Some(50.7));

    assert_eq!(list[2].lane, RankedLane::Mid);
    assert_eq!(list[2].name, "Ekko");
    assert_eq!(list[2].tier, 1);
    assert_eq!(list[2].overall_rank, 3);

    assert_eq!(list[3].lane, RankedLane::Jungle);
    assert_eq!(list[3].name, "Ekko");
    assert_eq!(list[3].tier, 2);
    assert_eq!(list[3].overall_rank, 4);
}
