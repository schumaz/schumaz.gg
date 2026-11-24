use super::tests::test_ddragon_cache;
use super::*;
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

#[tokio::test]
async fn test_tristana_aram_and_mayhem_rest_plus_mcp_merge_builds_exact_opgg_blocks() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/tristana/global/champions/ranked/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": ["16.22", "16.21"]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/tristana/global/champions/aram/18/none"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "summary": {
                    "id": 18,
                    "average_stats": { "play": 3627, "win_rate": 0.542046, "pick_rate": 0.091103 },
                    "roles": [{ "name": "MARKSMAN", "stats": { "play": 3271, "win": 1756 } }]
                },
                "summoner_spells": [
                    { "ids": [4, 7], "win": 599, "play": 1125, "pick_rate": 0.3358 },
                    { "ids": [4, 21], "win": 404, "play": 695, "pick_rate": 0.2075 },
                    { "ids": [4, 6], "win": 370, "play": 678, "pick_rate": 0.2024 }
                ],
                "starter_items": [
                    { "ids": [1038, 2003, 2003], "play": 1397, "win": 745, "pick_rate": 0.415 },
                    { "ids": [1038], "play": 398, "win": 222, "pick_rate": 0.1182 },
                    { "ids": [1042, 3006], "play": 251, "win": 124, "pick_rate": 0.0746 },
                    { "ids": [2031, 3051], "play": 121, "win": 69, "pick_rate": 0.0359 }
                ],
                "boots": [
                    { "ids": [3006], "play": 2640, "win": 1415, "pick_rate": 0.8585 },
                    { "ids": [3008], "play": 264, "win": 166, "pick_rate": 0.0859 },
                    { "ids": [3111], "play": 124, "win": 62, "pick_rate": 0.0403 },
                    { "ids": [3047], "play": 42, "win": 26, "pick_rate": 0.0137 }
                ],
                "core_items": [
                    { "ids": [3032, 6675, 3031], "play": 598, "win": 329, "pick_rate": 0.2025 },
                    { "ids": [3032, 3031, 6675], "play": 292, "win": 179, "pick_rate": 0.0989 },
                    { "ids": [3032, 3031, 3036], "play": 210, "win": 120, "pick_rate": 0.0711 },
                    { "ids": [3032, 6675, 3036], "play": 167, "win": 90, "pick_rate": 0.0566 },
                    { "ids": [3032, 6676, 3031], "play": 113, "win": 65, "pick_rate": 0.0383 },
                    { "ids": [6676, 6675, 3031], "play": 95, "win": 59, "pick_rate": 0.0322 },
                    { "ids": [3032, 6676, 3036], "play": 52, "win": 28, "pick_rate": 0.0176 },
                    { "ids": [6676, 3031, 3036], "play": 46, "win": 26, "pick_rate": 0.0156 },
                    { "ids": [6676, 3031, 6675], "play": 45, "win": 26, "pick_rate": 0.0152 },
                    { "ids": [3032, 3031, 3033], "play": 41, "win": 25, "pick_rate": 0.0139 },
                    { "ids": [3032, 6676, 6675], "play": 39, "win": 19, "pick_rate": 0.0132 },
                    { "ids": [6672, 6675, 3031], "play": 37, "win": 26, "pick_rate": 0.0125 },
                    { "ids": [3032, 6675, 3033], "play": 37, "win": 17, "pick_rate": 0.0125 },
                    { "ids": [3032, 3031, 3072], "play": 33, "win": 21, "pick_rate": 0.0112 },
                    { "ids": [6676, 6675, 3036], "play": 30, "win": 14, "pick_rate": 0.0102 }
                ],
                "last_items": [
                    { "ids": [3031], "play": 2613, "win": 1438 },
                    { "ids": [3032], "play": 2448, "win": 1317 },
                    { "ids": [6675], "play": 2056, "win": 1124 },
                    { "ids": [3036], "play": 1439, "win": 822 },
                    { "ids": [6676], "play": 1007, "win": 551 },
                    { "ids": [3072], "play": 848, "win": 500 },
                    { "ids": [3153], "play": 528, "win": 322 },
                    { "ids": [6672], "play": 429, "win": 239 },
                    { "ids": [1038], "play": 408, "win": 224 },
                    { "ids": [3094], "play": 274, "win": 136 },
                    { "ids": [3033], "play": 245, "win": 125 },
                    { "ids": [2523], "play": 132, "win": 72 },
                    { "ids": [3046], "play": 113, "win": 65 }
                ],
                "runes": [{
                    "primary_page_id": 8000,
                    "primary_rune_ids": [8008, 9111, 9104, 8014],
                    "secondary_page_id": 8100,
                    "secondary_rune_ids": [8135, 8139],
                    "stat_mod_ids": [5005, 5008, 5001],
                    "play": 1882,
                    "win": 1010,
                    "pick_rate": 0.5402
                }]
            }
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "type": "text",
                    "text": "class LolGetChampionAnalysis: data\nclass Data: fourth_items,fifth_items,counters_meta\nclass FourthItem: ids,play,win,pick_rate\nLolGetChampionAnalysis(Data([FourthItem([3036],593,346,0.29),FourthItem([3072],360,200,0.17),FourthItem([3031],283,159,0.14)],[FourthItem([3072],255,160,0.29),FourthItem([3153],138,87,0.16),FourthItem([3036],95,59,0.11)],[]))"
                }]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let cache = test_ddragon_cache();
    let rest_base = format!("{}/tristana", mock_server.uri());
    let mcp_url = format!("{}/mcp", mock_server.uri());

    for (queue_id, expected_supports_runes) in [
        (crate::ws_listener::QUEUE_ARAM, true),
        (crate::ws_listener::QUEUE_ARAM_MAYHEM, false),
    ] {
        let build = get_recommended_build_with_filters_and_endpoints(
            &client,
            18,
            queue_id,
            None,
            None,
            None,
            Some("16.22"),
            &cache,
            &rest_base,
            &mcp_url,
        )
        .await
        .expect("Tristana ARAM/Mayhem merge should succeed");

        assert_eq!(build.supports_runes, expected_supports_runes);
        assert_eq!(build.spell1_id, 4);
        assert_eq!(build.spell2_id, 7);
        assert_eq!(
            build
                .spell_options
                .iter()
                .map(|o| (o.spell1_id, o.spell2_id))
                .collect::<Vec<_>>(),
            vec![(4, 7), (4, 21)]
        );
        assert_eq!(build.item_blocks[0].item_ids, vec![1038, 2003]);
        assert_eq!(build.item_blocks[1].item_ids, vec![3032, 6675, 3031]);
        assert_eq!(build.item_blocks[2].item_ids, vec![3006, 3008, 3111]);
        assert_eq!(
            build.item_blocks[3].item_ids,
            vec![3032, 6675, 3031, 3036, 3072, 6676]
        );
        assert_eq!(
            build.item_blocks[4].item_ids,
            vec![3153, 6672, 3033, 3094, 2523, 3046]
        );
        assert_eq!(build.build_options[0].core_item_ids, vec![3032, 6675, 3031]);
        assert_eq!(
            build.build_options[0].final_item_ids,
            vec![3032, 6675, 3031, 3036, 3072, 6676]
        );
        assert_eq!(
            build.build_options[0].situational_item_ids,
            vec![3153, 6672, 3033, 3094, 2523, 3046]
        );
    }
}

#[tokio::test]
async fn test_patch_48h_protection_exposes_remaining_hours_and_respects_toggle() {
    let mock_server = MockServer::start().await;

    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let ten_hours_ago_iso = format_unix_iso8601(now_unix - 10 * 3600);

    Mock::given(method("GET"))
        .and(path("/patch48h/global/champions/ranked/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": ["16.22", "16.21"]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/patch48h/global/champions/aram/103/none"))
        .and(query_param("version", "16.22"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "damage_type": "AP",
                "summary": { "average_stats": { "win_rate": 0.50, "pick_rate": 0.05, "play": 120 } },
                "trends": { "win": [{ "created_at": ten_hours_ago_iso }] },
                "runes": [{
                    "primary_page_id": 8100,
                    "secondary_page_id": 8200,
                    "primary_rune_ids": [8112, 8126, 8138, 8135],
                    "secondary_rune_ids": [8226, 8210],
                    "stat_mod_ids": [5008, 5008, 5001],
                    "play": 120,
                    "win": 60
                }],
                "summoner_spells": [{ "ids": [4, 32], "play": 100, "win": 50 }],
                "starter_items": [{ "ids": [3802, 2003], "play": 100, "win": 50 }],
                "boots": [{ "ids": [3020], "play": 100, "win": 50 }],
                "core_items": [{ "ids": [6655, 4645, 3089], "play": 100, "win": 50 }],
                "last_items": [{ "ids": [3157], "play": 100, "win": 50 }]
            }
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/patch48h/global/champions/aram/103/none"))
        .and(query_param("version", "16.21"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "damage_type": "AP",
                "summary": { "average_stats": { "win_rate": 0.52, "pick_rate": 0.09, "play": 48000 } },
                "runes": [{
                    "primary_page_id": 8100,
                    "secondary_page_id": 8200,
                    "primary_rune_ids": [8112, 8126, 8138, 8135],
                    "secondary_rune_ids": [8226, 8210],
                    "stat_mod_ids": [5008, 5008, 5001],
                    "play": 30000,
                    "win": 16000
                }],
                "summoner_spells": [{ "ids": [4, 32], "play": 31000, "win": 16500 }],
                "starter_items": [{ "ids": [3802, 2003], "play": 25000, "win": 13500 }],
                "boots": [{ "ids": [3020], "play": 39000, "win": 20500 }],
                "core_items": [{ "ids": [6655, 4645, 3089], "play": 17000, "win": 9000 }],
                "last_items": [{ "ids": [3157], "play": 14000, "win": 7600 }]
            }
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let cache = test_ddragon_cache();
    let rest_base = format!("{}/patch48h", mock_server.uri());
    let mcp_url = format!("{}/mcp", mock_server.uri());

    let protected_build = get_recommended_build_with_filters_and_endpoints_with_protection(
        &client,
        103,
        crate::ws_listener::QUEUE_ARAM,
        None,
        None,
        None,
        None,
        &cache,
        &rest_base,
        &mcp_url,
        None,
        true,
    )
    .await
    .expect("Protected build should succeed");

    assert_eq!(protected_build.active_patch, "16.21");
    assert!(protected_build.patch_fallback_applied);
    assert_eq!(protected_build.patch_protection_remaining_hours, Some(38));

    let unprotected_build = get_recommended_build_with_filters_and_endpoints_with_protection(
        &client,
        103,
        crate::ws_listener::QUEUE_ARAM,
        None,
        None,
        None,
        None,
        &cache,
        &rest_base,
        &mcp_url,
        None,
        false,
    )
    .await
    .expect("Unprotected build should succeed");

    assert_eq!(unprotected_build.active_patch, "16.22");
    assert!(!unprotected_build.patch_fallback_applied);
    assert_eq!(unprotected_build.patch_protection_remaining_hours, None);

    let manual_build = get_recommended_build_with_filters_and_endpoints_with_protection(
        &client,
        103,
        crate::ws_listener::QUEUE_ARAM,
        None,
        None,
        None,
        Some("16.22"),
        &cache,
        &rest_base,
        &mcp_url,
        None,
        true,
    )
    .await
    .expect("Manual override build should succeed");

    assert_eq!(manual_build.active_patch, "16.22");
    assert!(!manual_build.patch_fallback_applied);
    assert_eq!(manual_build.patch_protection_remaining_hours, None);
}
