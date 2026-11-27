use super::{
    arena_versions_file_path, fetch_arena_champion_build_with_base_and_dir,
    fetch_arena_tier_list_with_base_and_dir, fetch_arena_versions_with_cache_dir,
};
use crate::build_provider::models::{ChampionMetadata, DataDragonCache, ItemBlockKind};
use crate::build_provider::service::get_recommended_build_with_filters_and_endpoints_with_dir;
use crate::models::{AppLocale, QUEUE_ARENA};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

static TEST_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn make_temp_dir(tag: &str) -> PathBuf {
    let seq = TEST_DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "schumaz_arena_{}_{}_{}",
        tag,
        std::process::id(),
        seq
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp test dir");
    dir
}

fn make_test_dd() -> DataDragonCache {
    let mut champions = HashMap::new();
    champions.insert(
        350,
        ChampionMetadata {
            champion_id: 350,
            key_name: "Yuumi".to_string(),
            display_name: "Yuumi".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Yuumi.png"
                .to_string(),
        },
    );
    champions.insert(
        11,
        ChampionMetadata {
            champion_id: 11,
            key_name: "MasterYi".to_string(),
            display_name: "Master Yi".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/MasterYi.png"
                .to_string(),
        },
    );
    DataDragonCache {
        version: "16.22.1".to_string(),
        champions,
        ..Default::default()
    }
}

fn sample_arena_tier_json() -> serde_json::Value {
    json!({
        "meta": { "version": "16.22" },
        "data": [
            {
                "id": 350,
                "is_rip": false,
                "average_stats": {
                    "win": 5200,
                    "play": 10000,
                    "total_place": 33000,
                    "first_place": 1500,
                    "pick_rate": 0.07,
                    "ban_rate": 0.14,
                    "tier": 1,
                    "rank": 1,
                    "tier_data": { "tier": 1, "rank": 1 }
                }
            }
        ]
    })
}

fn sample_arena_champ_json() -> serde_json::Value {
    json!({
        "data": {
            "summary": {
                "average_stats": {
                    "win": 5200,
                    "play": 10000,
                    "first_place": 1500,
                    "total_place": 33000,
                    "pick_rate": 0.07
                }
            },
            "boots": [{ "ids": [223158], "play": 8000, "win": 4200, "pick_rate": 0.8 }],
            "prism_items": [{ "ids": [443090], "play": 3000, "win": 1800, "pick_rate": 0.3 }],
            "core_items": [{ "ids": [226617, 223504, 223107], "play": 2500, "win": 1350, "pick_rate": 0.25 }],
            "last_items": [{ "ids": [226620], "play": 2000, "win": 1050, "pick_rate": 0.2 }],
            "augment_group": [
                {
                    "rarity": 1,
                    "augments": [
                        { "id": 48, "play": 1000, "win": 580, "first_place": 200, "total_place": 3100, "pick_rate": 0.1 }
                    ]
                }
            ],
            "synergies": [
                { "champion_id": 11, "op_rank": 1, "play": 800, "win": 480, "first_place": 160, "total_place": 2400, "pick_rate": 0.08 }
            ]
        }
    })
}

#[tokio::test]
async fn test_arena_tier_list_fetches_caches_lz4_and_falls_back_stale_while_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/global/champions/arena/versions"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "data": ["16.22", "16.21"] })),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/global/champions/arena"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_arena_tier_json()))
        .mount(&server)
        .await;

    let tmp_dir = make_temp_dir("tier");
    let client = reqwest::Client::new();
    let dd = make_test_dd();
    let now = crate::storage::now_unix_secs();

    let resp = fetch_arena_tier_list_with_base_and_dir(
        &client,
        &server.uri(),
        None,
        &dd,
        true,
        now,
        AppLocale::PtBr,
        Some(tmp_dir.as_path()),
    )
    .await
    .expect("arena tier list fetched");

    assert_eq!(resp.active_patch, "16.22");
    assert_eq!(resp.champions.len(), 1);
    assert_eq!(resp.champions[0].id, 350);

    // Offline server with expired TTL must still succeed via Stale-While-Error from `.lz4`
    let dead_server = MockServer::start().await;
    let stale_resp = fetch_arena_tier_list_with_base_and_dir(
        &client,
        &dead_server.uri(),
        None,
        &dd,
        true,
        now + crate::storage::OPGG_CACHE_TTL_SECS + 3600,
        AppLocale::PtBr,
        Some(tmp_dir.as_path()),
    )
    .await
    .expect("stale-while-error fallback from disk");

    assert_eq!(stale_resp.champions.len(), 1);
    assert_eq!(stale_resp.champions[0].name, "Yuumi");
    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[tokio::test]
async fn test_arena_versions_uses_3h_disk_ttl() {
    let tmp_dir = make_temp_dir("versions_3h");
    let ver_file = arena_versions_file_path(&tmp_dir);
    let now = crate::storage::now_unix_secs();
    let client = reqwest::Client::new();

    // 1h-old envelope (< 3h) -> hits disk with 0 network requests
    crate::storage::write_lz4_envelope_at(
        &ver_file,
        "16.22",
        &vec!["16.22".to_string(), "16.21".to_string()],
        now.saturating_sub(3600),
    )
    .unwrap();

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/arena_v_fresh/global/champions/arena/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": ["16.23"] })))
        .expect(0)
        .mount(&server)
        .await;

    let fresh_base = format!("{}/arena_v_fresh", server.uri());
    let cached = fetch_arena_versions_with_cache_dir(&client, &fresh_base, Some(&tmp_dir)).await;
    assert_eq!(cached, vec!["16.22", "16.21"]);

    // 4h-old envelope (> 3h, < 48h) -> expires and refreshes from network
    crate::storage::write_lz4_envelope_at(
        &ver_file,
        "16.22",
        &vec!["16.22".to_string(), "16.21".to_string()],
        now.saturating_sub(4 * 3600),
    )
    .unwrap();

    Mock::given(method("GET"))
        .and(path("/arena_v_stale/global/champions/arena/versions"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "data": ["16.23", "16.22"] })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let stale_base = format!("{}/arena_v_stale", server.uri());
    let refreshed = fetch_arena_versions_with_cache_dir(&client, &stale_base, Some(&tmp_dir)).await;
    assert_eq!(refreshed, vec!["16.23", "16.22"]);

    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[tokio::test]
async fn test_service_routes_arena_queue_without_fast_spells_or_runes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/global/champions/arena/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": ["16.22"] })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/global/champions/arena/350"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_arena_champ_json()))
        .mount(&server)
        .await;

    let tmp_dir = make_temp_dir("service");
    let client = reqwest::Client::new();
    let dd = make_test_dd();
    let (tx, rx) = tokio::sync::oneshot::channel();

    let build = get_recommended_build_with_filters_and_endpoints_with_dir(
        &client,
        350,
        QUEUE_ARENA,
        Some("mid"),
        Some("emerald_plus"),
        Some("global"),
        None,
        &dd,
        &server.uri(),
        "http://127.0.0.1:9/mcp",
        Some(tx),
        true,
        Some(tmp_dir.as_path()),
    )
    .await
    .expect("arena service build");

    assert!(!build.supports_runes);
    assert!(!build.supports_spells);
    assert_eq!(build.spell1_id, 0);
    assert_eq!(build.spell2_id, 0);
    assert!(rx.await.is_err(), "Arena must NOT emit fast spells");
    assert_eq!(build.item_blocks[0].kind, ItemBlockKind::Prismatic);
    assert_eq!(build.arena_augments.len(), 1);
    assert_eq!(build.arena_synergies.len(), 1);

    // Verify direct fetcher hits `.lz4` cache even if network fails later
    let dead_server = MockServer::start().await;
    let cached = fetch_arena_champion_build_with_base_and_dir(
        &client,
        &dead_server.uri(),
        350,
        None,
        &dd,
        true,
        crate::storage::now_unix_secs() + crate::storage::OPGG_CACHE_TTL_SECS + 3600,
        crate::lcu_injector::get_active_locale(),
        Some(tmp_dir.as_path()),
    )
    .await
    .expect("stale-while-error arena champion build");
    assert_eq!(cached.champion_id, 350);
    assert_eq!(cached.item_blocks[0].kind, ItemBlockKind::Prismatic);
    let _ = std::fs::remove_dir_all(&tmp_dir);
}
