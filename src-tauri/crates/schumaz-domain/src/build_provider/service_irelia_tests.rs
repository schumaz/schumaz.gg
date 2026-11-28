use super::*;
use crate::models::{ChampionMetadata, QUEUE_ARAM, QUEUE_ARAM_MAYHEM};
use std::collections::HashMap;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn format_unix_iso8601(unix_secs: u64) -> String {
    let days = (unix_secs / 86400) as i64;
    let rem = unix_secs % 86400;
    let hour = rem / 3600;
    let min = (rem % 3600) / 60;
    let sec = rem % 60;

    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, m, d, hour, min, sec
    )
}

fn irelia_ddragon_cache() -> DataDragonCache {
    let mut champions = HashMap::new();
    champions.insert(
        39,
        ChampionMetadata {
            champion_id: 39,
            key_name: "Irelia".to_string(),
            display_name: "Irelia".to_string(),
            icon_url: "https://example.com/irelia.png".to_string(),
        },
    );
    DataDragonCache {
        version: "16.23.1".to_string(),
        champions,
        runes: HashMap::new(),
        spells: HashMap::new(),
        ..Default::default()
    }
}

fn irelia_16_23_low_sample_payload(latest_iso: &str, prev_frozen_iso: &str) -> serde_json::Value {
    serde_json::json!({
        "data": {
            "summary": {
                "id": 39,
                "average_stats": { "play": 79, "win_rate": 0.481, "pick_rate": 0.032 },
                "roles": [{ "name": "FIGHTER", "stats": { "play": 79, "win": 38 } }]
            },
            "trends": {
                "win": [
                    { "version": "16.23", "created_at": latest_iso, "rate": 0.481 },
                    { "version": "16.22", "created_at": prev_frozen_iso, "rate": 0.512 }
                ]
            },
            "summoner_spells": [
                { "ids": [4, 32], "play": 55, "win": 27 },
                { "ids": [4, 7], "play": 18, "win": 9 }
            ],
            "starter_items": [
                { "ids": [3184, 2003], "play": 48, "win": 24 }
            ],
            "boots": [
                { "ids": [3111], "play": 36, "win": 18 },
                { "ids": [3047], "play": 28, "win": 14 }
            ],
            "core_items": [
                { "ids": [3153, 6610, 6333], "play": 14, "win": 9, "pick_rate": 0.177 },
                { "ids": [3153, 3078, 6333], "play": 6, "win": 4, "pick_rate": 0.076 },
                { "ids": [3153, 6610, 3053], "play": 4, "win": 2, "pick_rate": 0.051 },
                { "ids": [6655, 3118, 4646], "play": 1, "win": 1, "pick_rate": 0.013 }
            ],
            "last_items": [
                { "ids": [3153], "play": 64, "win": 33 },
                { "ids": [6610], "play": 40, "win": 21 },
                { "ids": [6333], "play": 34, "win": 18 },
                { "ids": [3053], "play": 22, "win": 11 },
                { "ids": [3078], "play": 18, "win": 9 },
                { "ids": [3091], "play": 14, "win": 7 }
            ],
            "runes": []
        }
    })
}

fn irelia_16_22_mature_payload() -> serde_json::Value {
    serde_json::json!({
        "data": {
            "summary": {
                "id": 39,
                "average_stats": { "play": 2855, "win_rate": 0.514, "pick_rate": 0.041 },
                "roles": [{ "name": "FIGHTER", "stats": { "play": 2855, "win": 1467 } }]
            },
            "summoner_spells": [
                { "ids": [4, 32], "play": 2100, "win": 1090 },
                { "ids": [4, 7], "play": 520, "win": 265 }
            ],
            "starter_items": [
                { "ids": [3184, 2003], "play": 1800, "win": 930 }
            ],
            "boots": [
                { "ids": [3111], "play": 1400, "win": 725 },
                { "ids": [3047], "play": 980, "win": 500 },
                { "ids": [3006], "play": 240, "win": 118 }
            ],
            "core_items": [
                { "ids": [3153, 6610, 6333], "play": 420, "win": 230, "pick_rate": 0.21 },
                { "ids": [3153, 3078, 6333], "play": 245, "win": 131, "pick_rate": 0.12 },
                { "ids": [3153, 3091, 6333], "play": 180, "win": 95, "pick_rate": 0.09 }
            ],
            "last_items": [
                { "ids": [3153], "play": 2410, "win": 1240 },
                { "ids": [6610], "play": 1620, "win": 850 },
                { "ids": [6333], "play": 1410, "win": 745 },
                { "ids": [3053], "play": 980, "win": 510 },
                { "ids": [3091], "play": 840, "win": 430 },
                { "ids": [3078], "play": 790, "win": 410 },
                { "ids": [3065], "play": 510, "win": 270 }
            ],
            "runes": []
        }
    })
}

#[tokio::test]
async fn test_irelia_aram_mayhem_16_23_48h_fallback_tier_all_and_no_ap_troll_build() {
    let mock_server = MockServer::start().await;
    let now_unix = crate::storage::now_unix_secs();
    let latest_13h_iso = format_unix_iso8601(now_unix - 13 * 3600);
    let prev_72h_iso = format_unix_iso8601(now_unix - 72 * 3600);

    Mock::given(method("GET"))
        .and(path("/irelia_fix/global/champions/ranked/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": ["16.23", "16.22"]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/irelia_fix/global/champions/aram/39/none"))
        .and(query_param("tier", "all"))
        .and(query_param("version", "16.23"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(irelia_16_23_low_sample_payload(
                &latest_13h_iso,
                &prev_72h_iso,
            )),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/irelia_fix/global/champions/aram/39/none"))
        .and(query_param("tier", "all"))
        .and(query_param("version", "16.22"))
        .respond_with(ResponseTemplate::new(200).set_body_json(irelia_16_22_mature_payload()))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let cache = irelia_ddragon_cache();
    let rest_base = format!("{}/irelia_fix", mock_server.uri());
    let mcp_url = format!("{}/mcp", mock_server.uri());

    // 1. With 48h protection enabled: defaults to tier=all and falls back from 16.23 (13h, 79 games) to 16.22
    let protected_query = BuildQuery::new(39, QUEUE_ARAM_MAYHEM)
        .with_endpoints(&rest_base, &mcp_url)
        .with_patch_48h_protection(true);
    let protected = resolve_recommended_build(&client, &protected_query, &cache, None)
        .await
        .expect("Protected Irelia ARAM Mayhem build should succeed");

    assert_eq!(protected.active_tier, "all");
    assert_eq!(protected.active_patch, "16.22");
    assert!(protected.patch_fallback_applied);
    assert_eq!(protected.patch_protection_remaining_hours, Some(35));
    assert_eq!(protected.item_blocks[1].item_ids, vec![3153, 6610, 6333]);
    assert_eq!(
        protected.build_options[0].core_item_ids,
        vec![3153, 6610, 6333]
    );

    // 2. Even on manual 16.23, play==1 AP troll build [6655, 3118, 4646] must never overtake [3153, 6610, 6333]
    let manual_query = BuildQuery::new(39, QUEUE_ARAM_MAYHEM)
        .with_filters(None, None, None, Some("16.23"))
        .with_endpoints(&rest_base, &mcp_url)
        .with_patch_48h_protection(true);
    let manual_16_23 = resolve_recommended_build(&client, &manual_query, &cache, None)
        .await
        .expect("Manual 16.23 Irelia build should succeed");

    assert_eq!(manual_16_23.active_patch, "16.23");
    assert_eq!(manual_16_23.item_blocks[1].item_ids, vec![3153, 6610, 6333]);
    assert_eq!(
        manual_16_23.build_options[0].core_item_ids,
        vec![3153, 6610, 6333]
    );
    for opt in &manual_16_23.build_options {
        assert!(!opt.core_item_ids.contains(&6655));
        assert!(!opt.core_item_ids.contains(&3118));
        assert!(!opt.core_item_ids.contains(&4646));
    }
}

#[tokio::test]
async fn test_aram_explicit_emerald_plus_falls_back_to_all_when_sample_under_100() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/aram_tier_fb/global/champions/ranked/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": ["16.22"]
        })))
        .mount(&mock_server)
        .await;

    // Low sample (< 100 games) on emerald_plus
    Mock::given(method("GET"))
        .and(path("/aram_tier_fb/global/champions/aram/39/none"))
        .and(query_param("tier", "emerald_plus"))
        .and(query_param("version", "16.22"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "summary": {
                    "id": 39,
                    "average_stats": { "play": 45, "win_rate": 0.44, "pick_rate": 0.02 },
                    "roles": [{ "name": "FIGHTER", "stats": { "play": 45, "win": 20 } }]
                },
                "summoner_spells": [{ "ids": [4, 32], "play": 30, "win": 14 }],
                "starter_items": [{ "ids": [3184, 2003], "play": 25, "win": 11 }],
                "boots": [{ "ids": [3111], "play": 20, "win": 9 }],
                "core_items": [{ "ids": [3153, 3078, 3053], "play": 5, "win": 2 }],
                "last_items": [{ "ids": [3153], "play": 35, "win": 16 }],
                "runes": [{
                    "primary_page_id": 8000,
                    "primary_rune_ids": [8010, 9111, 9104, 8299],
                    "secondary_page_id": 8400,
                    "secondary_rune_ids": [8473, 8453],
                    "stat_mod_ids": [5005, 5008, 5001],
                    "play": 40,
                    "win": 18
                }]
            }
        })))
        .mount(&mock_server)
        .await;

    // Consolidated sample on tier=all
    let mut all_payload = irelia_16_22_mature_payload();
    all_payload["data"]["runes"] = serde_json::json!([{
        "primary_page_id": 8000,
        "primary_rune_ids": [8010, 9111, 9104, 8299],
        "secondary_page_id": 8400,
        "secondary_rune_ids": [8473, 8453],
        "stat_mod_ids": [5005, 5008, 5001],
        "play": 2100,
        "win": 1100
    }]);
    Mock::given(method("GET"))
        .and(path("/aram_tier_fb/global/champions/aram/39/none"))
        .and(query_param("tier", "all"))
        .and(query_param("version", "16.22"))
        .respond_with(ResponseTemplate::new(200).set_body_json(all_payload))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let cache = irelia_ddragon_cache();
    let rest_base = format!("{}/aram_tier_fb", mock_server.uri());
    let mcp_url = format!("{}/mcp", mock_server.uri());

    let query = BuildQuery::new(39, QUEUE_ARAM)
        .with_filters(None, Some("emerald_plus"), None, Some("16.22"))
        .with_endpoints(&rest_base, &mcp_url);
    let build = resolve_recommended_build(&client, &query, &cache, None)
        .await
        .expect("ARAM build should fall back from emerald_plus to all");

    assert_eq!(build.active_tier, "all");
    assert_eq!(build.item_blocks[1].item_ids, vec![3153, 6610, 6333]);
}
