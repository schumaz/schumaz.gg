use super::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_fetch_opgg_rest_uses_none_for_aram_ignoring_position() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/global/champions/aram/103/none"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "ok": true }
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let base = mock_server.uri();
    let res = fetch_opgg_rest_champion_analysis_req(
        &client,
        &RestAnalysisRequest {
            rest_base: &base,
            region: "global",
            mode: "aram",
            champion_id: 103,
            position: Some("top"),
            tier: "emerald_plus",
            version: Some("16.22"),
        },
    )
    .await;

    assert!(res.is_ok(), "ARAM REST call should hit /aram/103/none");
    assert_eq!(res.unwrap()["data"]["ok"], serde_json::json!(true));
}
