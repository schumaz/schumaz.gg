use schumazgg_lib::build_provider::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_load_ddragon_cache_with_wiremock() {
    let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
    let mock_server = MockServer::start().await;

    let versions_json = serde_json::json!(["14.23.1"]);
    let champion_json = serde_json::json!({
        "data": {
            "Ahri": {
                "id": "Ahri",
                "key": "103",
                "name": "Ahri",
                "image": { "full": "Ahri.png" }
            }
        }
    });
    let runes_json = serde_json::json!([
        {
            "id": 8100,
            "key": "Domination",
            "icon": "Dom.png",
            "name": "Domination",
            "slots": []
        }
    ]);

    Mock::given(method("GET"))
        .and(path("/api/versions.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(versions_json))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/cdn/14.23.1/data/pt_BR/champion.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(champion_json))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/cdn/14.23.1/data/pt_BR/runesReforged.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(runes_json))
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let cache = load_ddragon_cache_from_base_url(&client, &mock_server.uri())
        .await
        .expect("Loading ddragon cache should succeed even if optional summoner.json/item.json are absent");

    assert_eq!(cache.version, "14.23.1");
    assert!(cache.champions.contains_key(&103));
    assert_eq!(cache.champions.get(&103).unwrap().display_name, "Ahri");
    assert!(cache.runes.contains_key(&8100));
    assert!(cache.spells.is_empty());
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_load_ddragon_cache_loads_pt_br_summoner_spells() {
    let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
    let mock_server = MockServer::start().await;

    let versions_json = serde_json::json!(["16.22.1"]);
    let champion_json = serde_json::json!({
        "data": {
            "Tristana": {
                "id": "Tristana",
                "key": "18",
                "name": "Tristana",
                "image": { "full": "Tristana.png" }
            }
        }
    });
    let runes_json = serde_json::json!([]);
    let summoner_json = serde_json::json!({
        "data": {
            "SummonerFlash": {
                "id": "SummonerFlash",
                "name": "Flash",
                "key": "4",
                "image": { "full": "SummonerFlash.png" }
            },
            "SummonerHeal": {
                "id": "SummonerHeal",
                "name": "Curar",
                "key": "7",
                "image": { "full": "SummonerHeal.png" }
            },
            "SummonerBarrier": {
                "id": "SummonerBarrier",
                "name": "Barreira",
                "key": "21",
                "image": { "full": "SummonerBarrier.png" }
            },
            "SummonerMana": {
                "id": "SummonerMana",
                "name": "Clareza",
                "key": "13",
                "image": { "full": "SummonerMana.png" }
            },
            "SummonerSnowball": {
                "id": "SummonerSnowball",
                "name": "Marca",
                "key": "32",
                "image": { "full": "SummonerSnowball.png" }
            },
            "SummonerTeleport": {
                "id": "SummonerTeleport",
                "name": "Teleporte",
                "key": "12",
                "image": { "full": "SummonerTeleport.png" }
            }
        }
    });

    Mock::given(method("GET"))
        .and(path("/api/versions.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(versions_json))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/cdn/16.22.1/data/pt_BR/champion.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(champion_json))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/cdn/16.22.1/data/pt_BR/runesReforged.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(runes_json))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/cdn/16.22.1/data/pt_BR/summoner.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(summoner_json))
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let cache = load_ddragon_cache_from_base_url(&client, &mock_server.uri())
        .await
        .expect("Loading ddragon cache with pt_BR summoner.json should succeed");

    assert_eq!(cache.spells.get(&32).unwrap().name, "Marca");
    assert_eq!(cache.spells.get(&13).unwrap().name, "Clareza");
    assert_eq!(cache.spells.get(&12).unwrap().name, "Teleporte");
    assert_eq!(cache.spells.get(&7).unwrap().name, "Curar");
    assert_eq!(cache.spells.get(&21).unwrap().name, "Barreira");
    assert!(
        cache
            .spells
            .get(&32)
            .unwrap()
            .icon_url
            .ends_with("/cdn/16.22.1/img/spell/SummonerSnowball.png")
    );
}
