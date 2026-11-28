use super::*;
use crate::build_provider::query::BuildQuery;
use crate::build_provider::resolve_recommended_build;
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

    let aram_query = BuildQuery::new(103, crate::models::QUEUE_ARAM)
        .with_filters(None, None, None, Some("16.99"))
        .with_endpoints(&ok_base, &mcp_url)
        .with_patch_48h_protection(false)
        .with_builds_dir(Some(&dir));
    let b1 = resolve_recommended_build(&client, &aram_query, &cache, None)
        .await
        .unwrap();
    assert_eq!(b1.active_patch, "16.99");
    assert_eq!(b1.item_blocks[0].item_ids, vec![3802, 2003]);

    // Clear L1 RAM cache -> second call hits L2 .lz4 disk with 0 network calls!
    build_provider_caches().builds.clear_ram();
    let b2 = resolve_recommended_build(&client, &aram_query, &cache, None)
        .await
        .unwrap();
    assert_eq!(b2.active_patch, "16.99");
    assert_eq!(b2.item_blocks[0].item_ids, vec![3802, 2003]);

    // Test Stale-While-Error when envelope is >48h expired and network fails
    build_provider_caches().builds.clear_ram();
    let cache_file_path = aram_query.cache_file_path(&dir);
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
    let offline_query = aram_query.clone().with_endpoints(&offline_base, &offline_mcp).with_builds_dir(Some(&dir));
    let rescued = resolve_recommended_build(&client, &offline_query, &cache, None)
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
    build_provider_caches().builds.clear_ram();

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
    let ranked_query = BuildQuery::new(103, crate::models::QUEUE_SOLO_DUO)
        .with_filters(Some("mid"), None, None, Some("16.99"))
        .with_endpoints(&base, &mcp_url)
        .with_patch_48h_protection(false)
        .with_builds_dir(Some(&dir));
    let ranked = resolve_recommended_build(&client, &ranked_query, &cache, None)
        .await
        .unwrap();

    // 2. Fetch Clash (700) -> redirects to Ranked (420) cache with 0 network calls (even after RAM clear)
    build_provider_caches().builds.clear_ram();
    let clash_query = BuildQuery::new(103, crate::models::QUEUE_CLASH)
        .with_filters(Some("mid"), None, None, Some("16.99"))
        .with_endpoints(&base, &mcp_url)
        .with_patch_48h_protection(false)
        .with_builds_dir(Some(&dir));
    let clash = resolve_recommended_build(&client, &clash_query, &cache, None)
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
    let swift_query = BuildQuery::new(103, crate::models::QUEUE_SWIFTPLAY)
        .with_filters(Some("mid"), None, None, Some("16.99"))
        .with_endpoints(&base, &mcp_url)
        .with_patch_48h_protection(false)
        .with_builds_dir(Some(&dir));
    let swift = resolve_recommended_build(&client, &swift_query, &cache, None)
        .await
        .unwrap();
    assert_eq!(swift.champion_id, 103);

    // Verify Swiftplay hits its own 480 disk cache on subsequent request with 0 extra network calls
    build_provider_caches().builds.clear_ram();
    let swift_cached = resolve_recommended_build(&client, &swift_query, &cache, None)
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

#[tokio::test]
async fn test_singleflight_coalesces_concurrent_builds_and_clear_all_ram_caches() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sf_ok/global/champions/ranked/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": ["16.97"]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/sf_ok/global/champions/aram/103/none"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "damage_type": "AP",
                "summary": { "average_stats": { "win_rate": 0.52, "pick_rate": 0.07, "play": 40000 } },
                "runes": [{
                    "primary_page_id": 8100,
                    "secondary_page_id": 8200,
                    "primary_rune_ids": [8112, 8126, 8138, 8135],
                    "secondary_rune_ids": [8226, 8210],
                    "stat_mod_ids": [5008, 5008, 5001],
                    "play": 20000,
                    "win": 10500
                }],
                "summoner_spells": [{ "ids": [4, 32], "play": 21000, "win": 11000 }],
                "starter_items": [{ "ids": [3802, 2003], "play": 19000, "win": 10000 }],
                "boots": [{ "ids": [3020], "play": 25000, "win": 13000 }],
                "core_items": [{ "ids": [6655, 4645, 3089], "play": 12000, "win": 6400 }],
                "last_items": [{ "ids": [3157], "play": 9000, "win": 4800 }]
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

    crate::build_provider::clear_all_ram_caches();

    let client = reqwest::Client::new();
    let cache = test_ddragon_cache();
    let mcp_url = format!("{}/mcp", mock_server.uri());
    let ok_base = format!("{}/sf_ok", mock_server.uri());
    let query = BuildQuery::new(103, crate::models::QUEUE_ARAM)
        .with_filters(None, None, None, Some("16.97"))
        .with_endpoints(&ok_base, &mcp_url)
        .with_patch_48h_protection(false);

    // 1. Concurrent cold requests coalesce into a single REST build fetch.
    let (r1, r2, r3) = tokio::join!(
        resolve_recommended_build(&client, &query, &cache, None),
        resolve_recommended_build(&client, &query, &cache, None),
        resolve_recommended_build(&client, &query, &cache, None),
    );
    assert_eq!(r1.unwrap().active_patch, "16.97");
    assert_eq!(r2.unwrap().active_patch, "16.97");
    assert_eq!(r3.unwrap().active_patch, "16.97");

    // 2. Clearing all RAM caches evicts L1 build entry, forcing a 2nd network fetch (Expect(2)).
    crate::build_provider::clear_all_ram_caches();
    let r4 = resolve_recommended_build(&client, &query, &cache, None)
        .await
        .unwrap();
    assert_eq!(r4.active_patch, "16.97");
}
