use super::*;
use crate::build_provider::items::TEST_CLASSIFICATION_MUTEX;
use crate::build_provider::*;
use crate::models::AppLocale;
use std::sync::Arc;
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_ddragon_lz4_persistence_hydrates_items_tooltips_and_supports_stale_while_error() {
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
        .and(path("/cdn/16.30.1/data/en_US/championFull.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "Ahri": {
                    "id": "Ahri",
                    "key": "103",
                    "name": "Ahri",
                    "image": { "full": "Ahri.png" },
                    "spells": [
                        { "name": "Orb of Deception", "description": "Throws an orb.", "cooldownBurn": "7", "maxrank": 5, "image": { "full": "AhriQ.png" } },
                        { "name": "Fox-Fire", "description": "Releases fox-fires.", "cooldownBurn": "9/8/7/6/5", "maxrank": 5, "image": { "full": "AhriW.png" } },
                        { "name": "Charm", "description": "Charms an enemy.", "cooldownBurn": "12", "maxrank": 5, "image": { "full": "AhriE.png" } },
                        { "name": "Spirit Rush", "description": "Dashes forward.", "cooldownBurn": "130/115/100", "maxrank": 3, "image": { "full": "AhriR.png" } }
                    ]
                }
            }
        })))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": 8100,
                "key": "Domination",
                "name": "Domination",
                "icon": "perk-images/Styles/7200_Domination.png",
                "slots": [
                    {
                        "runes": [
                            {
                                "id": 8112,
                                "key": "Electrocute",
                                "name": "Electrocute",
                                "icon": "perk-images/Styles/Domination/Electrocute/Electrocute.png",
                                "shortDesc": "Hitting a champion with 3 separate attacks deals <b>bonus adaptive damage</b>."
                            }
                        ]
                    }
                ]
            }
        ])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/cdn/16.30.1/data/en_US/item.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "9950": {
                    "name": "Base Disk Item",
                    "gold": { "total": 2800 },
                    "plaintext": "Increases Ability Power",
                    "description": "<mainText><stats><attention>80</attention> Ability Power</stats></mainText>"
                },
                "9951": { "name": "Evolved Disk Item", "inStore": false, "specialRecipe": 9950 }
            }
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/cdn/16.30.1/data/en_US/summoner.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "SummonerFlash": {
                    "key": "4",
                    "name": "Flash",
                    "description": "Teleports your champion a short distance.",
                    "cooldown": [300],
                    "image": { "full": "SummonerFlash.png" },
                    "modes": ["CLASSIC"]
                }
            }
        })))
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let loaded =
        refresh_ddragon_if_outdated_with_base(&client, &server.uri(), Some(&dir), AppLocale::EnUs)
            .await
            .unwrap();
    assert_eq!(loaded.version, "16.30.1");
    assert!(ddragon_metadata_file_path(&dir, AppLocale::EnUs).exists());

    // Clear RAM caches and hydrate from disk (.lz4) -> restores tooltip metadata and classifications
    if let Ok(mut g) = get_ddragon_cache_store().write() {
        g.remove(&AppLocale::EnUs);
    }
    set_dynamic_item_classifications(ItemClassificationCache::default());
    assert_eq!(normalize_tear_item_dynamic(9951), 9951);

    let hydrated = hydrate_ddragon_from_dir(&dir, AppLocale::EnUs).unwrap();
    assert_eq!(hydrated.version, "16.30.1");
    assert_eq!(hydrated.champions.get(&103).unwrap().display_name, "Ahri");
    assert_eq!(normalize_tear_item_dynamic(9951), 9950);
    let item = hydrated.items.get(&9950).expect("Item hydrated from .lz4");
    assert_eq!(item.gold_total, 2800);
    assert_eq!(item.stats, vec!["80 Ability Power"]);
    assert_eq!(item.plaintext, "Increases Ability Power");
    assert_eq!(
        hydrated.runes.get(&8112).unwrap().short_desc,
        "Hitting a champion with 3 separate attacks deals bonus adaptive damage."
    );
    assert_eq!(hydrated.spells.get(&4).unwrap().cooldown_secs, Some(300));
    let ahri_spells = hydrated
        .champion_abilities
        .get(&103)
        .expect("Champion abilities hydrated from .lz4");
    assert_eq!(ahri_spells.len(), 4);
    assert_eq!(ahri_spells[0].name, "Orb of Deception");

    // Legacy disk bundle missing `items` and `champion_abilities` on same version must backfill
    let legacy_bundle = DataDragonDiskBundle {
        ddragon: DataDragonCache {
            version: "16.30.1".to_string(),
            champions: hydrated.champions.clone(),
            ..Default::default()
        },
        item_classifications: None,
    };
    save_ddragon_bundle_to_dir(&dir, AppLocale::EnUs, &legacy_bundle).unwrap();
    if let Ok(mut g) = get_ddragon_cache_store().write() {
        g.remove(&AppLocale::EnUs);
    }
    let backfilled =
        refresh_ddragon_if_outdated_with_base(&client, &server.uri(), Some(&dir), AppLocale::EnUs)
            .await
            .unwrap();
    assert!(backfilled.items.contains_key(&9950));
    assert_eq!(backfilled.champion_abilities.get(&103).map(Vec::len), Some(4));

    // Patch rollout (16.30.1 -> 16.31.1): refreshes active locale and evicts stale-patch locales from RAM
    if let Ok(mut g) = get_ddragon_cache_store().write() {
        g.insert(
            AppLocale::PtBr,
            Arc::new(DataDragonCache {
                version: "16.30.1".to_string(),
                ..Default::default()
            }),
        );
    }
    let next_patch_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/versions.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!(["16.31.1"])))
        .mount(&next_patch_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/cdn/16.31.1/data/en_US/champion.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "Ahri": { "id": "Ahri", "key": "103", "name": "Ahri", "image": { "full": "Ahri.png" } }
            }
        })))
        .mount(&next_patch_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/cdn/16.31.1/data/en_US/runesReforged.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&next_patch_server)
        .await;
    let rolled = refresh_ddragon_if_outdated_with_base(
        &client,
        &next_patch_server.uri(),
        Some(&dir),
        AppLocale::EnUs,
    )
    .await
    .unwrap();
    assert_eq!(rolled.version, "16.31.1");
    assert!(
        get_ddragon_cache_store()
            .read()
            .unwrap()
            .get(&AppLocale::PtBr)
            .is_none(),
        "stale-patch locale must be evicted from RAM when patch advances"
    );

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
    assert_eq!(rescued.version, "16.31.1");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_ddragon_patch_expiration_helpers() {
    assert!(super::patch_state::is_ddragon_version_current(
        "16.30.1", None
    ));
    assert!(super::patch_state::is_ddragon_version_current(
        "16.30.1",
        Some("16.30.1")
    ));
    assert!(!super::patch_state::is_ddragon_version_current(
        "16.30.1",
        Some("16.31.1")
    ));
    assert!(!super::patch_state::is_ddragon_version_current(
        "16.30.1",
        Some("16.30.2")
    ));

    // Same patch within recheck window -> no unnecessary recheck.
    assert!(!super::patch_state::should_recheck_current_patch(
        "16.30.1",
        Some(Duration::from_secs(60)),
        Some("16.30")
    ));
    // Newer OP.GG patch major.minor -> triggers immediate patch recheck even within window.
    assert!(super::patch_state::should_recheck_current_patch(
        "16.30.1",
        Some(Duration::from_secs(60)),
        Some("16.31")
    ));
    // Recheck interval elapsed or never checked -> triggers recheck for long-running tray sessions.
    assert!(super::patch_state::should_recheck_current_patch(
        "16.30.1",
        None,
        Some("16.30")
    ));
    assert!(super::patch_state::should_recheck_current_patch(
        "16.30.1",
        Some(Duration::from_secs(crate::storage::VERSIONS_DISK_TTL_SECS)),
        Some("16.30")
    ));
}

#[test]
fn test_latest_cached_ddragon_version_from_dir_picks_newest_disk_envelope() {
    let dir = std::env::temp_dir().join(format!(
        "schumaz_dd_ver_test_{}_{}",
        std::process::id(),
        crate::storage::now_unix_secs()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);

    assert_eq!(latest_cached_ddragon_version_from_dir(&dir), None);

    // Non-DataDragon or corrupt files should be ignored safely.
    std::fs::write(dir.join("metadata_corrupt.lz4"), b"not-an-envelope").unwrap();
    crate::storage::write_lz4_envelope(&dir.join("other_cache.lz4"), "99.9.9", &42_i32).unwrap();
    assert_eq!(latest_cached_ddragon_version_from_dir(&dir), None);

    // Write an older skins envelope then a newer metadata envelope; newest created_at wins.
    crate::storage::write_lz4_envelope(&dir.join("skins_pt_BR.lz4"), "16.20.1", &"s1").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1050));
    crate::storage::write_lz4_envelope(&dir.join("metadata_en_US.lz4"), "16.24.1", &"m1").unwrap();

    assert_eq!(
        latest_cached_ddragon_version_from_dir(&dir).as_deref(),
        Some("16.24.1")
    );

    let _ = std::fs::remove_dir_all(&dir);
}
