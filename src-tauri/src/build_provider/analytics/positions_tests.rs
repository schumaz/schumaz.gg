use super::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn test_parse_opgg_champion_positions_ranking_and_select_most_played_lane() {
    let json = serde_json::json!({
        "data": [
            {
                "id": 157, // Yasuo
                "positions": [
                    { "name": "MID", "stats": { "play": 57307, "role_rate": 0.60 } },
                    { "name": "TOP", "stats": { "play": 22898, "role_rate": 0.24 } },
                    { "name": "ADC", "stats": { "play": 14609, "role_rate": 0.15 } }
                ]
            },
            {
                "id": 79, // Gragas
                "positions": [
                    { "name": "MID", "stats": { "play": 3491, "role_rate": 0.12 } },
                    { "name": "TOP", "stats": { "play": 16785, "role_rate": 0.60 } },
                    { "name": "JUNGLE", "stats": { "play": 5731, "role_rate": 0.20 } }
                ]
            },
            {
                "id": 147, // Seraphine
                "positions": [
                    { "name": "SUPPORT", "stats": { "play": 44180, "role_rate": 0.91 } }
                ]
            }
        ]
    });

    let map = parse_opgg_champion_positions_ranking(&json);
    assert_eq!(map.len(), 3);

    let yasuo_pos = map.get(&157).unwrap();
    assert_eq!(select_most_played_lane(yasuo_pos), Some("mid"));

    let gragas_pos = map.get(&79).unwrap();
    assert_eq!(select_most_played_lane(gragas_pos), Some("top"));

    let sera_pos = map.get(&147).unwrap();
    assert_eq!(select_most_played_lane(sera_pos), Some("support"));
}

#[tokio::test]
async fn test_resolve_champion_most_played_lane_cached() {
    let champ_id = 99999;
    let positions = vec![
        OpggPositionInfo {
            name: "TOP".to_string(),
            play: 100,
            role_rate: 0.1,
        },
        OpggPositionInfo {
            name: "ADC".to_string(),
            play: 9500,
            role_rate: 0.9,
        },
    ];

    let mut inner_map = HashMap::new();
    inner_map.insert(champ_id, positions);

    let cache_key = "global:emerald_plus:test_v1".to_string();
    if let Ok(mut guard) = CHAMPION_POSITIONS_CACHE.write() {
        guard.insert(cache_key, inner_map);
    }

    let dummy_client = reqwest::Client::new();
    let resolved = resolve_champion_most_played_lane(
        &dummy_client,
        champ_id,
        "global",
        "emerald_plus",
        Some("test_v1"),
    )
    .await;

    assert_eq!(resolved, Ok("adc".to_string()));
}

#[tokio::test]
async fn test_positions_disk_cache_and_stale_while_error() {
    let dir =
        std::env::temp_dir().join(format!("schumazgg_positions_cache_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/global/champions/ranked"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                { "id": 103, "positions": [{ "name": "MID", "stats": { "play": 50000, "role_rate": 0.9 } }] }
            ]
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let lane = resolve_champion_most_played_lane_with_base_and_dir(
        &client,
        &mock_server.uri(),
        103,
        "global",
        "emerald_plus",
        Some("16.25"),
        Some(&dir),
    )
    .await
    .unwrap();
    assert_eq!(lane, "mid");
    assert!(positions_file_path(&dir, "global", "emerald_plus", Some("16.25")).exists());

    // Clear RAM and test expired disk envelope with offline server -> Stale-While-Error rescues!
    if let Ok(mut g) = CHAMPION_POSITIONS_CACHE.write() {
        g.clear();
    }
    let disk_file = positions_file_path(&dir, "global", "emerald_plus", Some("16.25"));
    let env = crate::storage::read_lz4_envelope::<ChampionPositionsMap>(&disk_file).unwrap();
    crate::storage::write_lz4_envelope_at(&disk_file, "16.25", &env.payload, 1).unwrap();

    let offline = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/global/champions/ranked"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&offline)
        .await;

    let rescued = resolve_champion_most_played_lane_with_base_and_dir(
        &client,
        &offline.uri(),
        103,
        "global",
        "emerald_plus",
        Some("16.25"),
        Some(&dir),
    )
    .await
    .unwrap();
    assert_eq!(rescued, "mid");

    let _ = std::fs::remove_dir_all(&dir);
}
