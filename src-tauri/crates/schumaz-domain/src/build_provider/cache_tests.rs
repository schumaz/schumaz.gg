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
        ..Default::default()
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
        .and(path("/builds_disk_ok/global/champions/ranked/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": ["16.99"]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/builds_disk_ok/global/champions/aram/103/none"))
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
    let ok_base = format!("{}/builds_disk_ok", mock_server.uri());

    let b1 = get_recommended_build_with_filters_and_endpoints_with_dir(
        &client,
        103,
        crate::models::QUEUE_ARAM,
        None,
        None,
        None,
        Some("16.99"),
        &cache,
        &ok_base,
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
        crate::models::QUEUE_ARAM,
        None,
        None,
        None,
        Some("16.99"),
        &cache,
        &ok_base,
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
        crate::models::QUEUE_ARAM,
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
        .and(path("/builds_disk_offline/global/champions/aram/103/none"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&offline)
        .await;
    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&offline)
        .await;

    let offline_base = format!("{}/builds_disk_offline", offline.uri());
    let offline_mcp = format!("{}/mcp", offline.uri());
    let rescued = get_recommended_build_with_filters_and_endpoints_with_dir(
        &client,
        103,
        crate::models::QUEUE_ARAM,
        None,
        None,
        None,
        Some("16.99"),
        &cache,
        &offline_base,
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

#[tokio::test]
async fn test_clash_redirects_to_ranked_cache_while_swiftplay_uses_isolated_partition() {
    let dir = std::env::temp_dir().join(format!(
        "schumazgg_clash_swiftplay_cache_test_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    if let Ok(mut g) = get_build_cache().write() {
        g.clear();
    }

    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sr_partition/global/champions/ranked/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": ["16.99"]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/sr_partition/global/champions/ranked/103/mid"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "damage_type": "AP",
                "summary": { "average_stats": { "win_rate": 0.52, "pick_rate": 0.09, "play": 50000 } },
                "runes": [{
                    "primary_page_id": 8100,
                    "secondary_page_id": 8200,
                    "primary_rune_ids": [8112, 8126, 8138, 8135],
                    "secondary_rune_ids": [8226, 8210],
                    "stat_mod_ids": [5008, 5008, 5001],
                    "play": 30000,
                    "win": 16000
                }],
                "summoner_spells": [{ "ids": [4, 14], "play": 31000, "win": 16500 }],
                "starter_items": [{ "ids": [1056, 2003], "play": 25000, "win": 13500 }],
                "boots": [{ "ids": [3020], "play": 39000, "win": 20500 }],
                "core_items": [{ "ids": [6655, 4645, 3089], "play": 17000, "win": 9000 }],
                "last_items": [{ "ids": [3157], "play": 14000, "win": 7600 }]
            }
        })))
        .expect(2)
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
    let base = format!("{}/sr_partition", mock_server.uri());

    // 1. Fetch Ranked Solo/Duo (420) -> 1st network call, populates 420 cache
    let ranked = get_recommended_build_with_filters_and_endpoints_with_dir(
        &client,
        103,
        crate::models::QUEUE_SOLO_DUO,
        Some("mid"),
        None,
        None,
        Some("16.99"),
        &cache,
        &base,
        &mcp_url,
        None,
        false,
        Some(&dir),
    )
    .await
    .unwrap();

    // 2. Fetch Clash (700) -> redirects to Ranked (420) cache with 0 network calls (even after RAM clear)
    if let Ok(mut g) = get_build_cache().write() {
        g.clear();
    }
    let clash = get_recommended_build_with_filters_and_endpoints_with_dir(
        &client,
        103,
        crate::models::QUEUE_CLASH,
        Some("mid"),
        None,
        None,
        Some("16.99"),
        &cache,
        &base,
        &mcp_url,
        None,
        false,
        Some(&dir),
    )
    .await
    .unwrap();
    assert_eq!(clash.item_blocks, ranked.item_blocks);

    // Verify ZERO build_*_700_*.lz4 files were written on disk
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    assert!(entries.iter().any(|name| name.contains("build_103_420_")));
    assert!(!entries.iter().any(|name| name.contains("build_103_700_")));

    // 3. Fetch Swiftplay (480) -> 2nd network call, writes its own build_103_480_* partition
    let swift = get_recommended_build_with_filters_and_endpoints_with_dir(
        &client,
        103,
        crate::models::QUEUE_SWIFTPLAY,
        Some("mid"),
        None,
        None,
        Some("16.99"),
        &cache,
        &base,
        &mcp_url,
        None,
        false,
        Some(&dir),
    )
    .await
    .unwrap();
    assert_eq!(swift.champion_id, 103);

    // Verify Swiftplay hits its own 480 disk cache on subsequent request with 0 extra network calls
    if let Ok(mut g) = get_build_cache().write() {
        g.clear();
    }
    let swift_cached = get_recommended_build_with_filters_and_endpoints_with_dir(
        &client,
        103,
        crate::models::QUEUE_SWIFTPLAY,
        Some("mid"),
        None,
        None,
        Some("16.99"),
        &cache,
        &base,
        &mcp_url,
        None,
        false,
        Some(&dir),
    )
    .await
    .unwrap();
    assert_eq!(swift_cached.item_blocks, swift.item_blocks);

    let entries_after: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    assert!(entries_after.iter().any(|name| name.contains("build_103_480_")));
    assert!(!entries_after.iter().any(|name| name.contains("build_103_700_")));

    let _ = std::fs::remove_dir_all(&dir);
}
