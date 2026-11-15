use super::*;
use crate::build_provider::get_recommended_build_with_filters_and_endpoints_with_dir;
use crate::models::ChampionMetadata;
use std::collections::HashMap;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_ddragon_cache() -> DataDragonCache {
    let mut champions = HashMap::new();
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
    }
}

#[tokio::test]
async fn test_build_disk_cache_ttl_patch_invalidation_and_stale_while_error() {
    let dir =
        std::env::temp_dir().join(format!("schumazgg_builds_disk_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/global/champions/ranked/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": ["16.99"]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/global/champions/aram/103/none"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "damage_type": "AP",
                "summary": { "average_stats": { "win_rate": 0.54, "pick_rate": 0.08, "play": 50000 } },
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
        .expect(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let cache = test_ddragon_cache();
    let mcp_url = format!("{}/mcp", mock_server.uri());

    let b1 = get_recommended_build_with_filters_and_endpoints_with_dir(
        &client,
        103,
        crate::ws_listener::QUEUE_ARAM,
        None,
        None,
        None,
        Some("16.99"),
        &cache,
        &mock_server.uri(),
        &mcp_url,
        None,
        false,
        Some(&dir),
    )
    .await
    .unwrap();
    assert_eq!(b1.active_patch, "16.99");
    assert_eq!(b1.item_blocks[0].item_ids, vec![3802, 2003]);

    // Clear L1 RAM cache -> second call hits L2 .lz4 disk with 0 network calls!
    if let Ok(mut g) = get_build_cache().write() {
        g.clear();
    }
    let b2 = get_recommended_build_with_filters_and_endpoints_with_dir(
        &client,
        103,
        crate::ws_listener::QUEUE_ARAM,
        None,
        None,
        None,
        Some("16.99"),
        &cache,
        &mock_server.uri(),
        &mcp_url,
        None,
        false,
        Some(&dir),
    )
    .await
    .unwrap();
    assert_eq!(b2.active_patch, "16.99");
    assert_eq!(b2.item_blocks[0].item_ids, vec![3802, 2003]);

    // Test Stale-While-Error when envelope is >48h expired and network fails
    if let Ok(mut g) = get_build_cache().write() {
        g.clear();
    }
    let locale = crate::lcu_injector::get_user_preferences().language;
    let cache_file_path = build_cache_file_path(
        &dir,
        103,
        crate::ws_listener::QUEUE_ARAM,
        "",
        "emerald_plus",
        "global",
        "16.99",
        false,
        locale,
    );
    crate::storage::write_lz4_envelope_at(&cache_file_path, "16.99", &b2, 1).unwrap();

    let offline = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/global/champions/aram/103/none"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&offline)
        .await;
    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&offline)
        .await;

    let offline_mcp = format!("{}/mcp", offline.uri());
    let rescued = get_recommended_build_with_filters_and_endpoints_with_dir(
        &client,
        103,
        crate::ws_listener::QUEUE_ARAM,
        None,
        None,
        None,
        Some("16.99"),
        &cache,
        &offline.uri(),
        &offline_mcp,
        None,
        false,
        Some(&dir),
    )
    .await
    .unwrap();
    assert_eq!(rescued.active_patch, "16.99");
    assert_eq!(rescued.item_blocks[0].item_ids, vec![3802, 2003]);

    let _ = std::fs::remove_dir_all(&dir);
}
