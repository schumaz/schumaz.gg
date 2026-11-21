use super::*;
use crate::build_provider::analytics::aram::fetch_aram_mayhem_augments_with_base_locale_and_dir;
use crate::build_provider::analytics::client::fetch_opgg_rest_champion_analysis_from_base;
use crate::build_provider::models::DataDragonCache;
use crate::models::{AppLocale, AramAugmentEntry, ArenaAugmentMetadata};
use std::collections::HashMap;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn test_failure_cooldown_store_activation_isolation_expiry_and_eviction() {
    let store = FailureCooldownStore::with_capacity(Duration::from_secs(45), 2);
    let t0 = Instant::now();

    assert!(is_cooldown_status(429));
    assert!(is_cooldown_status(500));
    assert!(is_cooldown_status(503));
    assert!(!is_cooldown_status(404));
    assert!(!is_cooldown_status(400));
    assert!(!is_cooldown_status(200));

    // Activation & per-key isolation
    store.record_failure_at("key-a", "http_429", t0);
    let active = store.check_at("key-a", t0 + Duration::from_secs(10));
    assert!(active.is_some());
    assert_eq!(active.unwrap().1, "http_429");
    assert!(
        store
            .check_at("key-b", t0 + Duration::from_secs(10))
            .is_none()
    );

    // Expiration after TTL
    assert!(
        store
            .check_at("key-a", t0 + Duration::from_secs(45))
            .is_none()
    );
    assert!(
        store
            .check_at("key-a", t0 + Duration::from_secs(46))
            .is_none()
    );

    // Explicit clear removes active cooldown
    store.record_failure_at("key-a", "network_error", t0);
    store.clear("key-a");
    assert!(
        store
            .check_at("key-a", t0 + Duration::from_secs(1))
            .is_none()
    );

    // Bounded capacity evicts expired or oldest entries
    store.record_failure_at("k1", "http_500", t0);
    store.record_failure_at("k2", "http_502", t0 + Duration::from_secs(5));
    assert_eq!(store.len(), 2);
    store.record_failure_at("k3", "http_503", t0 + Duration::from_secs(10));
    assert_eq!(store.len(), 2);
    assert!(store.check_at("k1", t0 + Duration::from_secs(11)).is_none());
    assert!(store.check_at("k2", t0 + Duration::from_secs(11)).is_some());
    assert!(store.check_at("k3", t0 + Duration::from_secs(11)).is_some());
}

#[tokio::test]
async fn test_analytics_cooldown_prevents_burst_retries_on_429_and_allows_stale_fallback() {
    let mock_server = MockServer::start().await;

    // 1. REST endpoint returning 429: two rapid calls must hit the server ONLY ONCE
    Mock::given(method("GET"))
        .and(path("/cooldown_rest/global/champions/ranked/103/mid"))
        .respond_with(ResponseTemplate::new(429))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let rest_base = format!("{}/cooldown_rest", mock_server.uri());
    let first = fetch_opgg_rest_champion_analysis_from_base(
        &client,
        &rest_base,
        "global",
        "ranked",
        103,
        Some("mid"),
        "emerald_plus",
        Some("16.25"),
    )
    .await;
    assert!(first.is_err());

    let second = fetch_opgg_rest_champion_analysis_from_base(
        &client,
        &rest_base,
        "global",
        "ranked",
        103,
        Some("mid"),
        "emerald_plus",
        Some("16.25"),
    )
    .await;
    assert!(second.is_err());
    assert!(
        second.unwrap_err().contains("cooldown"),
        "second immediate call should short-circuit via cooldown"
    );

    // 2. ARAM Mayhem augments returning 503 with stale `.lz4`: two rapid calls hit network ONCE
    //    and both gracefully serve Stale-While-Error from disk.
    Mock::given(method("GET"))
        .and(path("/cooldown_mayhem/pt/ahri/augments"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&mock_server)
        .await;

    let dir = std::env::temp_dir().join(format!("schumazgg_cooldown_stale_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let mut arena_augments = HashMap::new();
    arena_augments.insert(
        3075,
        ArenaAugmentMetadata {
            id: 3075,
            name: "Coice".to_string(),
            desc: "Dano extra.".to_string(),
            icon_url: "https://raw.communitydragon.org/latest/recoil.png".to_string(),
            rarity: 4,
            ability_slot: None,
        },
    );
    let dd = DataDragonCache {
        version: "16.25.1".to_string(),
        arena_augments,
        ..Default::default()
    };

    let aug_path = crate::build_provider::analytics::aram::aram_augments_file_path(
        &dir,
        AppLocale::PtBr,
        "ahri",
    );
    let stale_payload = vec![AramAugmentEntry {
        id: 3075,
        name: "Coice".to_string(),
        tier: 0,
        rarity: Some(4),
        performance: 99.0,
        popular: 3.5,
        icon_url: "https://raw.communitydragon.org/latest/recoil.png".to_string(),
        desc: "Dano extra.".to_string(),
    }];
    crate::storage::write_lz4_envelope_at(&aug_path, "16.25", &stale_payload, 1).unwrap();

    let web_base = format!("{}/cooldown_mayhem/pt", mock_server.uri());
    let r1 = fetch_aram_mayhem_augments_with_base_locale_and_dir(
        &client,
        "Ahri",
        &web_base,
        AppLocale::PtBr,
        &dd,
        Some(&dir),
    )
    .await
    .unwrap();
    let r2 = fetch_aram_mayhem_augments_with_base_locale_and_dir(
        &client,
        "Ahri",
        &web_base,
        AppLocale::PtBr,
        &dd,
        Some(&dir),
    )
    .await
    .unwrap();

    assert_eq!(r1[0].id, 3075);
    assert_eq!(r2[0].id, 3075);
    let _ = std::fs::remove_dir_all(&dir);
}
