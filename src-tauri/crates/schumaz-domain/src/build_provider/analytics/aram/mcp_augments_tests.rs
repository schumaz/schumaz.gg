use super::super::augments::fetch_aram_mayhem_augments_with_endpoints_locale_and_dir;
use super::super::parsers::merge_mcp_with_top_tier_supplement;
use super::*;
use crate::models::{ArenaAugmentMetadata, ChampionMetadata};
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
            (3075, aug_meta(3075, "Coice", "", "recoil_small.png", 4)),
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

#[test]
fn test_resolve_champion_id_for_aram_handles_key_display_and_numeric() {
    let dd = make_dd();
    assert_eq!(resolve_champion_id_for_aram("Twisted Fate", &dd), Some(4));
    assert_eq!(resolve_champion_id_for_aram("TwistedFate", &dd), Some(4));
    assert_eq!(resolve_champion_id_for_aram("twistedfate", &dd), Some(4));
    assert_eq!(resolve_champion_id_for_aram("4", &dd), Some(4));
    assert_eq!(resolve_champion_id_for_aram("unknownchamp", &dd), None);
    assert_eq!(resolve_champion_id_for_aram("", &dd), None);
}

#[test]
fn test_parse_mcp_augments_dsl_handles_commas_parens_quotes_and_cdragon_hydration() {
    let dsl = r#"class LolListAramAugments: data
class Data: augments
class Augment: id,name,desc,tier,performance,popular

LolListAramAugments(Data([Augment(1074,"OPGG Ignored Name","Ataques reduzem dano (15%), e aplicam \"efeito\" <OnHit>ao contato</OnHit>.",3,79.754,0.224),Augment(2062,"Ignored","OPGG desc ignored because CDragon has desc",4,91.2,0.15),Augment(9999,"Uncatalogued","ignored",3,50.0,0.1)]))"#;

    let res = parse_aram_mayhem_augments_mcp_for_locale(
        &mcp_text_response(dsl),
        &make_dd(),
        AppLocale::PtBr,
    )
    .unwrap();

    assert_eq!(res.len(), 2);
    assert_eq!((res[0].id, res[0].tier, res[0].rarity), (1074, 3, Some(4)));
    assert_eq!(res[0].name, "Raio Encolhedor");
    assert_eq!((res[0].performance, res[0].popular), (79.75, 0.22));
    assert_eq!(
        res[0].desc,
        "Ataques reduzem dano (15%), e aplicam \"efeito\" ao contato."
    );
    assert!(res[0].icon_url.starts_with(CDRAGON_PREFIX));

    assert_eq!((res[1].id, res[1].rarity), (2062, Some(8)));
    assert_eq!(res[1].name, "Conjuração Ecoante");
    assert_eq!(res[1].desc, "Conjura duas vezes.");
}

#[test]
fn test_parse_mcp_augments_supports_json_payload_compact_dsl_and_rejects_errors() {
    let compact_dsl = "class Augment: id,tier,performance,popular\nLolListAramAugments(Data([Augment(2062,4,95.5,0.4)]))";
    let parsed_dsl = parse_aram_mayhem_augments_mcp_for_locale(
        &mcp_text_response(compact_dsl),
        &make_dd(),
        AppLocale::PtBr,
    )
    .unwrap();
    assert_eq!(
        (parsed_dsl.len(), parsed_dsl[0].id, parsed_dsl[0].tier),
        (1, 2062, 4)
    );

    let json_payload = serde_json::json!({
        "data": { "augments": [{ "id": 3075, "tier": 3, "performance": 88.4, "popular": 1.25 }] }
    });
    let parsed =
        parse_aram_mayhem_augments_mcp_for_locale(&json_payload, &make_dd(), AppLocale::PtBr)
            .unwrap();
    assert_eq!((parsed.len(), parsed[0].id), (1, 3075));

    let rpc_err = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "error": { "code": -32600, "message": "Invalid Request" }
    });
    assert!(
        parse_aram_mayhem_augments_mcp_for_locale(&rpc_err, &make_dd(), AppLocale::PtBr).is_err()
    );

    let tool_err = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "result": { "isError": true, "content": [{ "type": "text", "text": "boom" }] }
    });
    assert!(
        parse_aram_mayhem_augments_mcp_for_locale(&tool_err, &make_dd(), AppLocale::PtBr).is_err()
    );
}

#[test]
fn test_merge_mcp_with_top_tier_supplement_keeps_only_tier_under_3_and_preserves_mcp() {
    let mcp = vec![AramAugmentEntry {
        id: 1074,
        name: "Raio Encolhedor".to_string(),
        tier: 3,
        rarity: Some(4),
        performance: 79.75,
        popular: 0.22,
        icon_url: String::new(),
        desc: "MCP".to_string(),
    }];
    let html = vec![
        AramAugmentEntry {
            id: 3075,
            name: "Coice".to_string(),
            tier: 0,
            rarity: Some(4),
            performance: 99.6,
            popular: 3.8,
            icon_url: String::new(),
            desc: "Top".to_string(),
        },
        AramAugmentEntry {
            id: 1074,
            name: "Raio Encolhedor".to_string(),
            tier: 3,
            rarity: Some(4),
            performance: 10.0,
            popular: 0.01,
            icon_url: String::new(),
            desc: "HTML".to_string(),
        },
        AramAugmentEntry {
            id: 2062,
            name: "Conjuração Ecoante".to_string(),
            tier: 4,
            rarity: Some(8),
            performance: 80.0,
            popular: 0.1,
            icon_url: String::new(),
            desc: "Low".to_string(),
        },
    ];
    let merged = merge_mcp_with_top_tier_supplement(mcp, html);
    assert_eq!(merged.len(), 2);
    assert_eq!((merged[0].id, merged[0].tier), (3075, 0));
    assert_eq!(
        (merged[1].id, merged[1].tier, merged[1].performance),
        (1074, 3, 79.75)
    );
}

#[tokio::test]
async fn test_fetch_aram_mayhem_augments_mcp_locale_cache_isolation() {
    let mock_server = MockServer::start().await;
    let pt_dsl = "class Augment: id,desc,tier,performance,popular\nLolListAramAugments(Data([Augment(3075,\"Dano extra.\",0,99.6,3.8)]))";
    let en_dsl = "class Augment: id,desc,tier,performance,popular\nLolListAramAugments(Data([Augment(3075,\"Extra damage.\",0,99.6,3.8)]))";

    Mock::given(method("POST"))
        .and(path("/mcp_iso"))
        .and(body_string_contains("\"lang\":\"pt_BR\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(mcp_text_response(pt_dsl)))
        .expect(1)
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(path("/mcp_iso"))
        .and(body_string_contains("\"lang\":\"en_US\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(mcp_text_response(en_dsl)))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let mcp_url = format!("{}/mcp_iso", mock_server.uri());
    let mut dd_en = make_dd();
    dd_en
        .arena_augments
        .insert(3075, aug_meta(3075, "Recoil", "", "recoil_small.png", 4));

    let pt = fetch_aram_mayhem_augments_with_endpoints_locale_and_dir(
        &client,
        "Twisted Fate",
        &mcp_url,
        &format!("{}/pt", mock_server.uri()),
        AppLocale::PtBr,
        &make_dd(),
        None,
    )
    .await
    .unwrap();
    let en = fetch_aram_mayhem_augments_with_endpoints_locale_and_dir(
        &client,
        "Twisted Fate",
        &mcp_url,
        &format!("{}/en", mock_server.uri()),
        AppLocale::EnUs,
        &dd_en,
        None,
    )
    .await
    .unwrap();

    assert_eq!(
        (pt[0].name.as_str(), pt[0].desc.as_str()),
        ("Coice", "Dano extra.")
    );
    assert_eq!(
        (en[0].name.as_str(), en[0].desc.as_str()),
        ("Recoil", "Extra damage.")
    );
}

#[test]
fn test_parse_mcp_augments_dsl_handles_unquoted_multibyte_utf8_safely() {
    let dsl = "Descrição em pt_BR — çãé 증강 🔥 \
        çAugment(9999,\"Identificador inválido\",3,50.0,0.1) \
        NãoAugment(9998,\"Prefixo alfanumérico\",3,50.0,0.1) \
        —Augment(1074,\"Dano çãé em área\",3,79.754,0.224) \
        🔥Augment(2062,\"증강 설명 (15%)\",4,91.2,0.15) \
        Texto final çãé — Augment(3075,\"Incompleto çãé";

    let parsed = parse_mcp_augments_dsl(dsl);
    assert_eq!(parsed.len(), 2);
    assert_eq!(
        (parsed[0].id, parsed[0].desc.as_deref()),
        (1074, Some("Dano çãé em área"))
    );
    assert_eq!(
        (parsed[1].id, parsed[1].desc.as_deref()),
        (2062, Some("증강 설명 (15%)"))
    );
}
