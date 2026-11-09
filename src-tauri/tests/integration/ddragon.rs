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
            .expect("Loading ddragon cache should succeed");

        assert_eq!(cache.version, "14.23.1");
        assert!(cache.champions.contains_key(&103));
        assert_eq!(cache.champions.get(&103).unwrap().display_name, "Ahri");
        assert!(cache.runes.contains_key(&8100));
    }
