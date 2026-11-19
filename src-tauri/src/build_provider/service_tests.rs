use super::*;
use crate::models::ChampionMetadata;
use std::collections::HashMap;
use std::time::Instant;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub(super) fn test_ddragon_cache() -> DataDragonCache {
    let mut champions = HashMap::new();
    champions.insert(
        18,
        ChampionMetadata {
            champion_id: 18,
            key_name: "Tristana".to_string(),
            display_name: "Tristana".to_string(),
            icon_url: "https://example.com/tristana.png".to_string(),
        },
    );
    champions.insert(
        103,
        ChampionMetadata {
            champion_id: 103,
            key_name: "Ahri".to_string(),
            display_name: "Ahri".to_string(),
            icon_url: "https://example.com/ahri.png".to_string(),
        },
    );
    DataDragonCache {
        version: "16.22.1".to_string(),
        champions,
        runes: HashMap::new(),
        spells: HashMap::new(),
        ..Default::default()
    }
}

#[tokio::test]
async fn test_aram_and_aram_mayhem_ignore_position_and_return_empty_active_position() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/aram_service_test/global/champions/ranked/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": ["16.22", "16.21"]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/aram_service_test/global/champions/ranked"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/aram_service_test/global/champions/aram/18/mid"))
        .respond_with(ResponseTemplate::new(422))
        .expect(0)
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/aram_service_test/global/champions/aram/18/top"))
        .respond_with(ResponseTemplate::new(422))
        .expect(0)
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/aram_service_test/global/champions/aram/103/mid"))
        .respond_with(ResponseTemplate::new(422))
        .expect(0)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/aram_service_test/global/champions/aram/18/none"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "damage_type": "AD",
                "summary": { "average_stats": { "win_rate": 0.53, "pick_rate": 0.08, "play": 50000 } },
                "runes": [],
                "summoner_spells": [
                    { "ids": [4, 7], "play": 32000, "win": 17000 },
                    { "ids": [4, 21], "play": 18000, "win": 9500 },
                    { "ids": [4, 32], "play": 5000, "win": 2500 }
                ],
                "starter_items": [{ "ids": [1038, 2003], "play": 27000, "win": 14400 }],
                "boots": [{ "ids": [3006], "play": 40000, "win": 21000 }],
                "core_items": [{ "ids": [3032, 6675, 3031], "play": 18000, "win": 9500 }],
                "last_items": [{ "ids": [3036], "play": 15000, "win": 8000 }]
            }
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/aram_service_test/global/champions/aram/103/none"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "damage_type": "AP",
                "summary": { "average_stats": { "win_rate": 0.52, "pick_rate": 0.09, "play": 48000 } },
                "runes": [{
                    "primary_page_id": 8100,
                    "secondary_page_id": 8200,
                    "primary_rune_ids": [8128, 8126, 8138, 8106],
                    "secondary_rune_ids": [8226, 8210],
                    "stat_mod_ids": [5008, 5008, 5001],
                    "play": 30000,
                    "win": 16000,
                    "pick_rate": 0.62
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
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "type": "text",
                    "text": "class LolGetChampionAnalysis: data\nclass Data: fourth_items,fifth_items\nLolGetChampionAnalysis(Data([FourthItem([3036],579,335,0.29)],[FourthItem([3072],248,155,0.29)]))"
                }]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let cache = test_ddragon_cache();
    let base_uri = format!("{}/aram_service_test", mock_server.uri());
    let mcp_url = format!("{}/mcp", mock_server.uri());

    let mayhem_build = get_recommended_build_with_filters_and_endpoints(
        &client,
        18,
        crate::ws_listener::QUEUE_ARAM_MAYHEM,
        Some("top"),
        None,
        None,
        Some("16.22"),
        &cache,
        &base_uri,
        &mcp_url,
    )
    .await
    .expect("ARAM Mayhem build should succeed");

    assert_eq!(mayhem_build.active_position, "");
    assert!(!mayhem_build.supports_runes);
    assert_eq!(mayhem_build.spell1_id, 4);
    assert_eq!(mayhem_build.spell2_id, 7);
    assert_eq!(
        mayhem_build
            .spell_choices
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>(),
        vec![7, 21]
    );

    let aram_build = get_recommended_build_with_filters_and_endpoints(
        &client,
        103,
        crate::ws_listener::QUEUE_ARAM,
        Some("mid"),
        None,
        None,
        Some("16.22"),
        &cache,
        &base_uri,
        &mcp_url,
    )
    .await
    .expect("Classic ARAM build should succeed");

    assert_eq!(aram_build.active_position, "");
    assert!(aram_build.supports_runes);
    assert_eq!(aram_build.primary_style_id, 8100);
    assert_eq!(aram_build.sub_style_id, 8200);
}

#[tokio::test]
async fn test_fast_spells_channel_resolves_without_waiting_for_slow_mcp_and_avoids_duplicate_rest()
{
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/fast_spells/global/champions/ranked/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": ["16.22", "16.21"]
        })))
        .expect(0..=1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/fast_spells/global/champions/aram/18/none"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "damage_type": "AD",
                "summary": { "average_stats": { "win_rate": 0.53, "pick_rate": 0.08, "play": 50000 } },
                "runes": [{
                    "primary_page_id": 8000,
                    "secondary_page_id": 8100,
                    "primary_rune_ids": [8008, 9111, 9104, 8014],
                    "secondary_rune_ids": [8139, 8135],
                    "stat_mod_ids": [5005, 5008, 5001],
                    "play": 30000,
                    "win": 16000
                }],
                "summoner_spells": [
                    { "ids": [4, 7], "play": 35000, "win": 18500 },
                    { "ids": [4, 21], "play": 18000, "win": 9500 }
                ],
                "starter_items": [{ "ids": [1038, 2003], "play": 27000, "win": 14400 }],
                "boots": [{ "ids": [3006], "play": 40000, "win": 21000 }],
                "core_items": [{ "ids": [3032, 6675, 3031], "play": 18000, "win": 9500 }],
                "last_items": [{ "ids": [3036], "play": 15000, "win": 8000 }]
            }
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_millis(1200))
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": "class LolGetChampionAnalysis: data\nclass Data: fourth_items\nLolGetChampionAnalysis(Data([FourthItem([3036],579,335,0.29)]))"
                        }]
                    }
                })),
        )
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let cache = test_ddragon_cache();
    let mcp_url = format!("{}/mcp", mock_server.uri());
    let base_uri = format!("{}/fast_spells", mock_server.uri());
    let (fast_tx, fast_rx) = tokio::sync::oneshot::channel();

    let started = Instant::now();
    let build_handle = tokio::spawn(async move {
        get_recommended_build_with_filters_and_endpoints_internal(
            &client,
            18,
            crate::ws_listener::QUEUE_ARAM,
            None,
            None,
            None,
            None,
            &cache,
            &base_uri,
            &mcp_url,
            Some(fast_tx),
        )
        .await
    });

    let fast_spells = fast_rx
        .await
        .expect("Fast spell channel should receive spells");
    let fast_elapsed = started.elapsed();

    assert_eq!(fast_spells.champion_id, 18);
    assert_eq!(fast_spells.spell1_id, 4);
    assert_eq!(fast_spells.spell2_id, 7);
    assert!(!fast_spells.is_jungle);
    assert!(
        fast_elapsed < std::time::Duration::from_millis(600),
        "Fast spells took {:?}, should NOT block on 1200ms MCP request!",
        fast_elapsed
    );

    let build = build_handle
        .await
        .unwrap()
        .expect("Full build should still complete");
    assert_eq!(build.spell1_id, 4);
    assert_eq!(build.spell2_id, 7);
}
