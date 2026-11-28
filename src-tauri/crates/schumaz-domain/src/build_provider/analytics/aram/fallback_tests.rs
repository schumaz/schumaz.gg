use super::augments::{AramAugmentsQuery, fetch_aram_mayhem_augments_internal};
use super::cache::aram_augments_file_path;
use super::{AramTierQuery, fetch_aram_tier_list_internal};
use crate::build_provider::models::DataDragonCache;
use crate::models::{
    AppLocale, AramAugmentEntry, AramMode, ArenaAugmentMetadata, ChampionMetadata,
};
use std::collections::HashMap;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CDRAGON_PREFIX: &str = "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/";

fn aug_meta(id: i32, name: &str, desc: &str, icon: &str, rarity: i32) -> ArenaAugmentMetadata {
    ArenaAugmentMetadata {
        id,
        name: name.to_string(),
        desc: desc.to_string(),
        icon_url: format!("{CDRAGON_PREFIX}{icon}"),
        rarity,
        ability_slot: None,
    }
}

fn make_dd() -> DataDragonCache {
    DataDragonCache {
        version: "16.25.1".to_string(),
        champions: HashMap::from([(
            4,
            ChampionMetadata {
                champion_id: 4,
                key_name: "TwistedFate".to_string(),
                display_name: "Twisted Fate".to_string(),
                icon_url:
                    "https://ddragon.leagueoflegends.com/cdn/16.25.1/img/champion/TwistedFate.png"
                        .to_string(),
            },
        )]),
        arena_augments: HashMap::from([
            (
                1074,
                aug_meta(1074, "Raio Encolhedor", "", "shrinkray_small.png", 4),
            ),
            (
                2062,
                aug_meta(
                    2062,
                    "Conjuração Ecoante",
                    "Conjura duas vezes.",
                    "echo_small.png",
                    8,
                ),
            ),
            (
                3075,
                aug_meta(3075, "Coice", "Causa dano bônus.", "recoil_small.png", 4),
            ),
        ]),
        ..Default::default()
    }
}

fn mcp_text_response(text: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": { "content": [{ "type": "text", "text": text }] }
    })
}

fn html_augments_response(raw_json_array: &str) -> String {
    let inner = format!(r#"6c:["$","$L6d",null,{{"data":{raw_json_array}}}]"#);
    format!(
        r#"<html><script>self.__next_f.push([1,{}])</script></html>"#,
        serde_json::to_string(&inner).unwrap()
    )
}

#[tokio::test]
async fn test_augments_mcp_primary_merges_html_top_tiers_and_survives_html_failure() {
    let dd = make_dd();
    let client = reqwest::Client::new();
    let server = MockServer::start().await;

    let mcp_dsl = "class Augment: id,desc,tier,performance,popular\nLolListAramAugments(Data([Augment(1074,\"MCP desc.\",3,79.75,0.22)]))";
    Mock::given(method("POST"))
        .and(path("/mcp_merge"))
        .and(body_string_contains("lol_list_aram_augments"))
        .and(body_string_contains("\"champion_id\":4"))
        .and(body_string_contains("\"lang\":\"pt_BR\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(mcp_text_response(mcp_dsl)))
        .expect(1)
        .mount(&server)
        .await;

    let html_body = html_augments_response(
        r#"[{"id":3075,"tier":0,"performance":99.6,"popular":3.8},{"id":1074,"tier":3,"performance":11.1,"popular":0.01}]"#,
    );
    Mock::given(method("GET"))
        .and(path("/html_merge/twistedfate/augments"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html_body))
        .expect(1)
        .mount(&server)
        .await;

    let mcp_merged_url = format!("{}/mcp_merge", server.uri());
    let html_merged_url = format!("{}/html_merge", server.uri());
    let merged = fetch_aram_mayhem_augments_internal(
        &client,
        &AramAugmentsQuery {
            champion_key: "Twisted Fate",
            mcp_endpoint: &mcp_merged_url,
            web_base: &html_merged_url,
            locale: AppLocale::PtBr,
            aram_dir: None,
            known_latest: Some("16.25"),
            html_fallback_enabled: true,
        },
        &dd,
    )
    .await
    .unwrap();
    assert_eq!(merged.len(), 2);
    assert_eq!((merged[0].id, merged[0].tier), (3075, 0));
    assert_eq!(
        (merged[1].id, merged[1].tier, merged[1].performance),
        (1074, 3, 79.75)
    );

    // When HTML top-tier supplement returns 503, MCP primary still succeeds cleanly.
    Mock::given(method("POST"))
        .and(path("/mcp_only_survive"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mcp_text_response(mcp_dsl)))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/html_down/twistedfate/augments"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;

    let mcp_survive_url = format!("{}/mcp_only_survive", server.uri());
    let html_down_url = format!("{}/html_down", server.uri());
    let mcp_only = fetch_aram_mayhem_augments_internal(
        &client,
        &AramAugmentsQuery {
            champion_key: "Twisted Fate",
            mcp_endpoint: &mcp_survive_url,
            web_base: &html_down_url,
            locale: AppLocale::PtBr,
            aram_dir: None,
            known_latest: Some("16.25"),
            html_fallback_enabled: true,
        },
        &dd,
    )
    .await
    .unwrap();
    assert_eq!((mcp_only.len(), mcp_only[0].id), (1, 1074));
}

#[tokio::test]
async fn test_augments_html_fallback_switch_and_stale_before_html() {
    let dd = make_dd();
    let client = reqwest::Client::new();
    let server = MockServer::start().await;

    let mcp_dsl = "class Augment: id,desc,tier,performance,popular\nLolListAramAugments(Data([Augment(1074,\"MCP desc.\",3,79.75,0.22)]))";
    Mock::given(method("POST"))
        .and(path("/mcp_no_html"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mcp_text_response(mcp_dsl)))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/html_forbidden/twistedfate/augments"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let mcp_no_html = format!("{}/mcp_no_html", server.uri());
    let html_forbidden = format!("{}/html_forbidden", server.uri());
    let pure_mcp = fetch_aram_mayhem_augments_internal(
        &client,
        &AramAugmentsQuery {
            champion_key: "Twisted Fate", mcp_endpoint: &mcp_no_html, web_base: &html_forbidden,
            locale: AppLocale::PtBr, aram_dir: None, known_latest: Some("16.25"),
            html_fallback_enabled: false,
        },
        &dd,
    )
    .await
    .unwrap();
    assert_eq!((pure_mcp.len(), pure_mcp[0].id), (1, 1074));

    // Clean Stale-While-Error `.lz4` rescues BEFORE HTML fallback (0 HTML requests).
    let dir = std::env::temp_dir().join(format!(
        "schumazgg_mcp_stale_before_html_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let stale_entries = vec![AramAugmentEntry {
        id: 2062,
        name: "Conjuração Ecoante".to_string(),
        tier: 0,
        rarity: Some(8),
        performance: 110.0,
        popular: 3.1,
        icon_url: format!("{CDRAGON_PREFIX}echo_small.png"),
        desc: "Conjura duas vezes.".to_string(),
    }];
    let aug_path = aram_augments_file_path(&dir, AppLocale::PtBr, "twistedfate");
    crate::storage::write_lz4_envelope_at(&aug_path, "16.24", &stale_entries, 1).unwrap();

    Mock::given(method("POST"))
        .and(path("/mcp_down_stale"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;

    let mcp_down_stale = format!("{}/mcp_down_stale", server.uri());
    let rescued = fetch_aram_mayhem_augments_internal(
        &client,
        &AramAugmentsQuery {
            champion_key: "Twisted Fate", mcp_endpoint: &mcp_down_stale, web_base: &html_forbidden,
            locale: AppLocale::PtBr, aram_dir: Some(&dir), known_latest: Some("16.25"),
            html_fallback_enabled: true,
        },
        &dd,
    )
    .await
    .unwrap();
    assert_eq!((rescued.len(), rescued[0].id), (1, 2062));
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn test_mayhem_tier_list_uses_rest_primary_and_isolates_html_fallback() {
    let dd = make_dd();
    let client = reqwest::Client::new();
    let server = MockServer::start().await;

    let rest_json = serde_json::json!({
        "data": [{ "id": 4, "average_stats": { "win_rate": 0.54, "pick_rate": 0.08, "tier": 1, "rank": 3 } }]
    });
    Mock::given(method("GET"))
        .and(path("/aram_rest_primary"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rest_json))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/mayhem_html_unused"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let rest_primary = format!("{}/aram_rest_primary", server.uri());
    let html_unused = format!("{}/mayhem_html_unused", server.uri());
    let primary = fetch_aram_tier_list_internal(
        &client,
        &AramTierQuery {
            mode: AramMode::AramMayhem, aram_rest_url: &rest_primary, mayhem_web_url: &html_unused,
            locale: AppLocale::PtBr, aram_dir: None, known_latest: Some("16.25"),
            html_fallback_enabled: true,
        },
        &dd,
    )
    .await
    .unwrap();
    assert_eq!((primary.len(), primary[0].id, primary[0].key.as_str()), (1, 4, "twistedfate"));

    Mock::given(method("GET"))
        .and(path("/aram_rest_down_off"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;
    let rest_down_off = format!("{}/aram_rest_down_off", server.uri());
    assert!(
        fetch_aram_tier_list_internal(
            &client,
            &AramTierQuery {
                mode: AramMode::AramMayhem, aram_rest_url: &rest_down_off, mayhem_web_url: &html_unused,
                locale: AppLocale::PtBr, aram_dir: None, known_latest: Some("16.25"),
                html_fallback_enabled: false,
            },
            &dd,
        )
        .await
        .is_err()
    );

    let html_tier = format!(
        r#"<html><script>self.__next_f.push([1,{}])</script></html>"#,
        serde_json::to_string(
            r#"1a:["$","$L32",null,{"champions":[{"key":"twisted_fate","name":"Twisted Fate","id":4,"tier":1,"rank":1}]}]"#
        )
        .unwrap()
    );
    Mock::given(method("GET"))
        .and(path("/aram_rest_down_on"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/mayhem_html_fallback"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html_tier))
        .expect(1)
        .mount(&server)
        .await;

    let rest_down_on = format!("{}/aram_rest_down_on", server.uri());
    let html_fb = format!("{}/mayhem_html_fallback", server.uri());
    let fallback = fetch_aram_tier_list_internal(
        &client,
        &AramTierQuery {
            mode: AramMode::AramMayhem, aram_rest_url: &rest_down_on, mayhem_web_url: &html_fb,
            locale: AppLocale::PtBr, aram_dir: None, known_latest: Some("16.25"),
            html_fallback_enabled: true,
        },
        &dd,
    )
    .await
    .unwrap();
    assert_eq!((fallback.len(), fallback[0].id, fallback[0].tier), (1, 4, 0));
}
