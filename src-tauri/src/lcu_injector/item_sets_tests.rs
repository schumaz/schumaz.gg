use super::*;
use crate::build_provider::{ItemBlock, ItemBlockKind};
use crate::lcu_connector::build_item_sets_endpoint;
use crate::lcu_injector::{TEST_SELECTIONS_LOCK, clear_champion_selections};
use serde_json::json;
use std::sync::Mutex;

fn sample_item_build(champ_id: i32, name: &str, items: Vec<i32>) -> ChampionBuild {
    ChampionBuild {
        champion_id: champ_id,
        champion_name: name.to_string(),
        item_blocks: vec![ItemBlock {
            kind: ItemBlockKind::Core,
            title: "Primeiros 3 Itens".to_string(),
            item_ids: items,
            ..Default::default()
        }],
        ..Default::default()
    }
}

struct MockItemSetsStore {
    get_result: Result<serde_json::Value, String>,
    get_calls: Mutex<usize>,
    put_payloads: Mutex<Vec<(u64, serde_json::Value)>>,
}

impl MockItemSetsStore {
    fn with_get(get_result: Result<serde_json::Value, String>) -> Self {
        Self {
            get_result,
            get_calls: Mutex::new(0),
            put_payloads: Mutex::new(Vec::new()),
        }
    }
}

impl ItemSetsStore for MockItemSetsStore {
    async fn get_item_sets(&self, _summoner_id: u64) -> Result<serde_json::Value, String> {
        *self.get_calls.lock().unwrap() += 1;
        self.get_result.clone()
    }

    async fn put_item_sets(
        &self,
        summoner_id: u64,
        payload: &serde_json::Value,
    ) -> Result<(), String> {
        self.put_payloads
            .lock()
            .unwrap()
            .push((summoner_id, payload.clone()));
        Ok(())
    }
}

#[test]
fn test_build_item_sets_endpoint_formats_summoner_id() {
    assert_eq!(
        build_item_sets_endpoint(12345),
        "/lol-item-sets/v1/item-sets/12345/sets"
    );
}

#[test]
fn test_merge_preserves_manual_sets_verbatim_and_replaces_managed_set_in_place() {
    let _guard = TEST_SELECTIONS_LOCK.blocking_lock();
    clear_champion_selections();

    let manual_before = json!({
        "uid": "user-uid-1",
        "title": "My Custom Ahri",
        "mode": "CLASSIC",
        "map": "11",
        "type": "custom",
        "sortrank": 0,
        "startedFrom": "blank",
        "associatedChampions": [103],
        "associatedMaps": [11],
        "blocks": [{
            "type": "Rush",
            "recMath": false,
            "items": [{ "id": "3118", "count": 1 }]
        }],
        "preferredItemSlots": [{ "id": "3157", "preferredItemSlot": 1 }]
    });
    let old_managed = json!({
        "uid": "schumaz-old-uid",
        "title": "Ahri | schumaz.gg",
        "associatedChampions": [103],
        "associatedMaps": [11, 12, 30],
        "blocks": [{ "type": "Old", "items": [{ "id": "1056", "count": 1 }] }],
        "mode": "any",
        "type": "custom"
    });
    let manual_after = json!({
        "uid": "user-uid-2",
        "title": "ARAM Tank Build",
        "associatedChampions": [],
        "associatedMaps": [12],
        "blocks": []
    });

    let existing = json!({
        "accountId": 987654,
        "timestamp": 1700000000123_i64,
        "itemSets": [manual_before.clone(), old_managed, manual_after.clone()]
    });

    let jinx = sample_item_build(222, "Jinx", vec![3031, 3085, 3036]);
    let (merged, changed) = merge_item_sets_payload(12345, &existing, &[&jinx]).unwrap();

    assert!(changed);
    assert_eq!(merged["accountId"], 987654);
    assert_eq!(merged["timestamp"], 1700000000123_i64);

    let sets = merged["itemSets"].as_array().unwrap();
    assert_eq!(sets.len(), 3);
    assert_eq!(sets[0], manual_before);
    assert_eq!(sets[1]["title"], "Jinx | schumaz.gg");
    assert_eq!(sets[1]["associatedChampions"], json!([222]));
    assert_eq!(sets[2], manual_after);
}

#[test]
fn test_merge_appends_multi_champion_sets_and_defaults_missing_account_id() {
    let _guard = TEST_SELECTIONS_LOCK.blocking_lock();
    clear_champion_selections();

    let manual = json!({ "uid": "u1", "title": "SoloQ Picks", "blocks": [] });
    let existing = json!({ "itemSets": [manual.clone()] });

    let ahri = sample_item_build(103, "Ahri", vec![3118, 4645, 3089]);
    let lee = sample_item_build(64, "LeeSin", vec![6692, 3071, 3053]);

    let (merged, changed) = merge_item_sets_payload(555001, &existing, &[&ahri, &lee]).unwrap();
    assert!(changed);
    assert_eq!(merged["accountId"], 555001);

    let sets = merged["itemSets"].as_array().unwrap();
    assert_eq!(sets.len(), 3);
    assert_eq!(sets[0], manual);
    assert_eq!(sets[1]["title"], "Ahri | schumaz.gg");
    assert_eq!(sets[2]["title"], "LeeSin | schumaz.gg");
}

#[test]
fn test_merge_skips_put_when_lcu_enriched_managed_set_is_semantically_identical() {
    let _guard = TEST_SELECTIONS_LOCK.blocking_lock();
    clear_champion_selections();

    let manual = json!({ "uid": "u1", "title": "Player Custom", "blocks": [] });
    let lcu_enriched_managed = json!({
        "uid": "server-uuid-999",
        "title": "Jinx | schumaz.gg",
        "mode": "any",
        "map": "any",
        "type": "custom",
        "sortrank": 1,
        "startedFrom": "blank",
        "associatedChampions": [222],
        "associatedMaps": [11, 12, 30],
        "preferredItemSlots": [],
        "blocks": [{
            "type": "Primeiros 3 Itens",
            "recMath": false,
            "minSummonerLevel": -1,
            "maxSummonerLevel": -1,
            "showIfSummonerSpell": "",
            "hideIfSummonerSpell": "",
            "appendAfterSection": "",
            "visibleWithAllOf": [],
            "hiddenWithAnyOf": [],
            "items": [
                { "id": "3031", "count": 1 },
                { "id": "3085", "count": 1 },
                { "id": "3036", "count": 1 }
            ]
        }]
    });

    let existing = json!({
        "accountId": 42,
        "itemSets": [manual, lcu_enriched_managed]
    });

    let same_jinx = sample_item_build(222, "Jinx", vec![3031, 3085, 3036]);
    let (_, changed_same) = merge_item_sets_payload(42, &existing, &[&same_jinx]).unwrap();
    assert!(!changed_same);

    let updated_jinx = sample_item_build(222, "Jinx", vec![3031, 3085, 3033]);
    let (merged_diff, changed_diff) =
        merge_item_sets_payload(42, &existing, &[&updated_jinx]).unwrap();
    assert!(changed_diff);
    assert_eq!(
        merged_diff["itemSets"][1]["blocks"][0]["items"][2]["id"],
        "3033"
    );

    // Non-contiguous managed sets must consolidate even if items match individually
    let ahri = sample_item_build(103, "Ahri", vec![3118]);
    let ahri_set = build_managed_item_sets(&[&ahri]).pop().unwrap();
    let jinx_set = build_managed_item_sets(&[&same_jinx]).pop().unwrap();
    let non_contiguous = json!({
        "accountId": 42,
        "itemSets": [ahri_set, { "title": "Middle Manual" }, jinx_set]
    });
    let (consolidated, changed_non_contig) =
        merge_item_sets_payload(42, &non_contiguous, &[&ahri, &same_jinx]).unwrap();
    assert!(changed_non_contig);
    assert_eq!(consolidated["itemSets"][0]["title"], "Ahri | schumaz.gg");
    assert_eq!(consolidated["itemSets"][1]["title"], "Jinx | schumaz.gg");
    assert_eq!(consolidated["itemSets"][2]["title"], "Middle Manual");
}

#[test]
fn test_merge_rejects_malformed_or_error_payloads_and_accepts_valid_empty_bundle() {
    let _guard = TEST_SELECTIONS_LOCK.blocking_lock();
    clear_champion_selections();

    let jinx = sample_item_build(222, "Jinx", vec![3031]);

    for bad in [
        json!({ "errorCode": "RESOURCE_NOT_FOUND", "httpStatus": 404 }),
        json!({ "httpStatus": 503, "message": "Unavailable" }),
        json!([]),
        json!({}),
        json!({ "accountId": 42, "itemSets": "corrupted" }),
    ] {
        assert!(
            merge_item_sets_payload(42, &bad, &[&jinx]).is_err(),
            "expected Err for payload {bad}"
        );
    }

    let valid_empty = json!({ "accountId": 42, "itemSets": [] });
    let (merged, changed) = merge_item_sets_payload(42, &valid_empty, &[&jinx]).unwrap();
    assert!(changed);
    assert_eq!(merged["itemSets"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_sync_item_sets_with_store_lifecycle_and_noop_skip() {
    let _guard = TEST_SELECTIONS_LOCK.lock().await;
    clear_champion_selections();

    let jinx = sample_item_build(222, "Jinx", vec![3031, 3085, 3036]);

    // 1. Empty builds skips both GET and PUT
    let empty_store = MockItemSetsStore::with_get(Ok(json!({ "accountId": 42, "itemSets": [] })));
    assert!(!sync_item_sets_with_store(&empty_store, 42, &[]).await.unwrap());
    assert_eq!(*empty_store.get_calls.lock().unwrap(), 0);
    assert!(empty_store.put_payloads.lock().unwrap().is_empty());

    // 2. GET failure never issues PUT
    let err_store = MockItemSetsStore::with_get(Err("404 NotFound".to_string()));
    assert!(sync_item_sets_with_store(&err_store, 42, &[&jinx]).await.is_err());
    assert_eq!(*err_store.get_calls.lock().unwrap(), 1);
    assert!(err_store.put_payloads.lock().unwrap().is_empty());

    // 3. Initial sync issues PUT preserving manual set
    let manual = json!({ "uid": "u1", "title": "My Custom", "blocks": [] });
    let update_store =
        MockItemSetsStore::with_get(Ok(json!({ "accountId": 42, "itemSets": [manual.clone()] })));
    assert!(sync_item_sets_with_store(&update_store, 42, &[&jinx]).await.unwrap());
    {
        let puts = update_store.put_payloads.lock().unwrap();
        assert_eq!(puts.len(), 1);
        assert_eq!(puts[0].0, 42);
        assert_eq!(puts[0].1["itemSets"].as_array().unwrap().len(), 2);
        assert_eq!(puts[0].1["itemSets"][0], manual);
    }

    // 4. Subsequent re-sync with LCU metadata skips PUT
    let mut enriched = build_managed_item_sets(&[&jinx]).pop().unwrap();
    enriched["uid"] = json!("lcu-assigned-uid");
    enriched["sortrank"] = json!(1);
    let noop_store = MockItemSetsStore::with_get(Ok(json!({
        "accountId": 42,
        "itemSets": [manual, enriched]
    })));
    assert!(!sync_item_sets_with_store(&noop_store, 42, &[&jinx]).await.unwrap());
    assert_eq!(*noop_store.get_calls.lock().unwrap(), 1);
    assert!(noop_store.put_payloads.lock().unwrap().is_empty());
}
