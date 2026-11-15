use super::*;
use crate::build_provider::items::TEST_CLASSIFICATION_MUTEX;
use crate::build_provider::*;
use crate::models::AppLocale;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_ddragon_lz4_persistence_hydrates_items_and_supports_stale_while_error() {
    let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
    let dir = std::env::temp_dir().join(format!(
        "schumaz_dd_test_{}_{}",
        std::process::id(),
        crate::storage::now_unix_secs()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);

    if let Ok(mut g) = get_ddragon_cache_store().write() {
        g.remove(&AppLocale::EnUs);
    }

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/versions.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!(["16.30.1"])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/cdn/16.30.1/data/en_US/champion.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "Ahri": { "id": "Ahri", "key": "103", "name": "Ahri", "image": { "full": "Ahri.png" } }
            }
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/cdn/16.30.1/data/en_US/runesReforged.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/cdn/16.30.1/data/en_US/item.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "9950": { "name": "Base Disk Item", "gold": { "total": 2800 } },
                "9951": { "name": "Evolved Disk Item", "inStore": false, "specialRecipe": 9950 }
            }
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/cdn/16.30.1/data/en_US/summoner.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "data": {} })))
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let loaded =
        refresh_ddragon_if_outdated_with_base(&client, &server.uri(), Some(&dir), AppLocale::EnUs)
            .await
            .unwrap();
    assert_eq!(loaded.version, "16.30.1");
    assert!(ddragon_metadata_file_path(&dir, AppLocale::EnUs).exists());

    // Clear RAM caches and hydrate from disk (.lz4) -> should restore both DataDragonCache and DYNAMIC_ITEM_CLASSIFICATIONS
    if let Ok(mut g) = get_ddragon_cache_store().write() {
        g.remove(&AppLocale::EnUs);
    }
    set_dynamic_item_classifications(ItemClassificationCache::default());
    assert_eq!(normalize_tear_item_dynamic(9951), 9951);

    let hydrated = hydrate_ddragon_from_dir(&dir, AppLocale::EnUs).unwrap();
    assert_eq!(hydrated.version, "16.30.1");
    assert_eq!(hydrated.champions.get(&103).unwrap().display_name, "Ahri");
    assert_eq!(normalize_tear_item_dynamic(9951), 9950);

    // Offline server: refresh falls back to disk/RAM cache (*Stale-While-Error*)
    if let Ok(mut g) = get_ddragon_cache_store().write() {
        g.remove(&AppLocale::EnUs);
    }
    let offline = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/versions.json"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&offline)
        .await;

    let rescued =
        refresh_ddragon_if_outdated_with_base(&client, &offline.uri(), Some(&dir), AppLocale::EnUs)
            .await
            .unwrap();
    assert_eq!(rescued.version, "16.30.1");

    let _ = std::fs::remove_dir_all(&dir);
}
