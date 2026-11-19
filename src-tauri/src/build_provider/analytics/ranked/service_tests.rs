use super::cache::get_ranked_cache;
use super::*;
use crate::build_provider::patch::parse_iso8601_to_unix;
use crate::models::ChampionMetadata;
use std::collections::HashMap;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
    DataDragonCache {
        version: "16.22.1".to_string(),
        champions,
        runes: HashMap::new(),
        spells: HashMap::new(),
        ..Default::default()
    }
}

#[tokio::test]
async fn test_fetch_ranked_tier_list_applies_48h_patch_protection_and_manual_override() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/ranked_48h/global/champions/ranked/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": ["16.22", "16.21"]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/ranked_48h/global/champions/ranked"))
        .and(query_param("tier", "emerald_plus"))
        .and(query_param("version", "16.22"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": { "version": "16.22", "match_count": 150 },
            "data": [
                {
                    "id": 103,
                    "positions": [
                        { "name": "MID", "stats": { "win_rate": 0.49, "pick_rate": 0.05, "tier_data": { "tier": 2, "rank": 5 } } }
                    ]
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let created_at = "2026-11-14T08:00:00Z";
    let now_unix = parse_iso8601_to_unix(created_at).unwrap() + 10 * 3600;
    Mock::given(method("GET"))
        .and(path("/ranked_48h/global/champions/ranked/103/mid"))
        .and(query_param("tier", "emerald_plus"))
        .and(query_param("version", "16.22"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "trends": {
                    "win": [{ "created_at": created_at }]
                }
            }
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/ranked_48h/global/champions/ranked"))
        .and(query_param("tier", "emerald_plus"))
        .and(query_param("version", "16.21"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": { "version": "16.21", "match_count": 500000 },
            "data": [
                {
                    "id": 103,
                    "positions": [
                        { "name": "MID", "stats": { "win_rate": 0.52, "pick_rate": 0.11, "tier_data": { "tier": 1, "rank": 1 } } }
                    ]
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let ddragon = test_ddragon();
    let base_uri = format!("{}/ranked_48h", mock_server.uri());

    let protected = fetch_ranked_tier_list_with_base(
        &client,
        &base_uri,
        Some("global"),
        Some("emerald_plus"),
        None,
        &ddragon,
        true,
        now_unix,
    )
    .await
    .unwrap();

    assert_eq!(protected.active_patch, "16.21");
    assert!(protected.patch_fallback_applied);
    assert_eq!(protected.patch_protection_remaining_hours, Some(38));
    assert_eq!(protected.champions[0].tier, 0);

    let manual = fetch_ranked_tier_list_with_base(
        &client,
        &base_uri,
        Some("global"),
        Some("emerald_plus"),
        Some("16.22"),
        &ddragon,
        true,
        now_unix,
    )
    .await
    .unwrap();

    assert_eq!(manual.active_patch, "16.22");
    assert!(!manual.patch_fallback_applied);
    assert_eq!(manual.patch_protection_remaining_hours, None);
    assert_eq!(manual.champions[0].tier, 1);
}

#[tokio::test]
async fn test_ranked_tier_list_disk_cache_ttl_and_stale_while_error() {
    let dir =
        std::env::temp_dir().join(format!("schumazgg_ranked_disk_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ranked_disk/global/champions/ranked/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": ["16.25"]
        })))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ranked_disk/global/champions/ranked"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": { "version": "16.25", "match_count": 50000 },
            "data": [{
                "id": 103,
                "positions": [{ "name": "MID", "stats": { "win_rate": 0.53, "pick_rate": 0.1, "tier_data": { "tier": 1, "rank": 1 } } }]
            }]
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let dd = test_ddragon();
    let now = crate::storage::now_unix_secs();
    let disk_base = format!("{}/ranked_disk", mock_server.uri());

    let first = fetch_ranked_tier_list_with_base_and_dir(
        &client,
        &disk_base,
        Some("global"),
        Some("emerald_plus"),
        None,
        &dd,
        false,
        now,
        AppLocale::PtBr,
        Some(&dir),
    )
    .await
    .unwrap();
    assert_eq!(first.champions.len(), 1);
    assert_eq!(first.active_patch, "16.25");

    // Clear RAM and verify disk hit with 0 network calls within 48h
    if let Ok(mut g) = get_ranked_cache().write() {
        g.clear();
    }
    let disk_hit = fetch_ranked_tier_list_with_base_and_dir(
        &client,
        &disk_base,
        Some("global"),
        Some("emerald_plus"),
        None,
        &dd,
        false,
        now + 26 * 3600,
        AppLocale::PtBr,
        Some(&dir),
    )
    .await
    .unwrap();
    assert_eq!(disk_hit.champions[0].id, 103);

    // Now test Stale-While-Error when >48h expired and network returns 503
    if let Ok(mut g) = get_ranked_cache().write() {
        g.clear();
    }
    let offline = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ranked_offline/global/champions/ranked"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&offline)
        .await;

    let offline_base = format!("{}/ranked_offline", offline.uri());
    let rescued = fetch_ranked_tier_list_with_base_and_dir(
        &client,
        &offline_base,
        Some("global"),
        Some("emerald_plus"),
        None,
        &dd,
        false,
        now + 49 * 3600,
        AppLocale::PtBr,
        Some(&dir),
    )
    .await
    .unwrap();
    assert_eq!(rescued.champions[0].id, 103);

    let _ = std::fs::remove_dir_all(&dir);
}
