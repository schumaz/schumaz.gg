use super::cache::{get_augments_cache, rehydrate_aram_names};
use super::*;
use crate::models::{ArenaAugmentMetadata, ChampionMetadata};
use std::collections::HashMap;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_ddragon() -> DataDragonCache {
    let mut champions = HashMap::new();
    champions.insert(
        4,
        ChampionMetadata {
            champion_id: 4,
            key_name: "TwistedFate".to_string(),
            display_name: "Twisted Fate".to_string(),
            icon_url: "https://ddragon.example/TwistedFate.png".to_string(),
        },
    );
    champions.insert(
        50,
        ChampionMetadata {
            champion_id: 50,
            key_name: "Swain".to_string(),
            display_name: "Swain".to_string(),
            icon_url: "https://ddragon.example/Swain.png".to_string(),
        },
    );

    let mut arena_augments = HashMap::new();
    arena_augments.insert(
        1074,
        ArenaAugmentMetadata {
            id: 1074,
            name: "Raio Encolhedor".to_string(),
            desc: "Ao contato Reduz dano.".to_string(),
            icon_url: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/shrinkray_small.png".to_string(),
            rarity: 4,
        },
    );
    arena_augments.insert(
        2062,
        ArenaAugmentMetadata {
            id: 2062,
            name: "Conjuração Ecoante".to_string(),
            desc: "Conjura duas vezes.".to_string(),
            icon_url: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/echo_small.png".to_string(),
            rarity: 8,
        },
    );
    arena_augments.insert(
        3075,
        ArenaAugmentMetadata {
            id: 3075,
            name: "Coice".to_string(),
            desc: "? causa dano.".to_string(),
            icon_url: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/recoil_small.png".to_string(),
            rarity: 4,
        },
    );

    DataDragonCache {
        version: "16.22.1".to_string(),
        champions,
        runes: HashMap::new(),
        spells: HashMap::new(),
        arena_augments,
    }
}

fn test_ddragon_en() -> DataDragonCache {
    let mut dd = test_ddragon();
    dd.arena_augments.insert(
        3075,
        ArenaAugmentMetadata {
            id: 3075,
            name: "Recoil".to_string(),
            desc: "Extra damage.".to_string(),
            icon_url: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/recoil_small.png".to_string(),
            rarity: 4,
        },
    );
    dd
}

#[test]
fn test_aram_mayhem_augments_web_base_varies_by_locale() {
    assert_eq!(
        aram_mayhem_augments_web_base(AppLocale::PtBr),
        "https://op.gg/pt/lol/modes/aram-mayhem"
    );
    assert_eq!(
        aram_mayhem_augments_web_base(AppLocale::EnUs),
        "https://op.gg/lol/modes/aram-mayhem"
    );
}

#[test]
fn test_parse_classic_aram_champions_sorts_by_rank_and_normalizes_rates() {
    let json = serde_json::json!({
        "data": [
            { "id": 4, "average_stats": { "win_rate": 0.522, "pick_rate": 0.078, "tier": 2, "rank": 16 } },
            { "id": 50, "average_stats": { "win_rate": 0.633, "pick_rate": 0.091, "tier": 0, "rank": 1 } }
        ]
    });

    let list = parse_classic_aram_champions(&json, &test_ddragon()).unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, 50);
    assert_eq!(list[0].name, "Swain");
    assert_eq!(list[0].rank, 1);
    assert_eq!(list[0].tier, 0);
    assert_eq!(list[0].win_rate, Some(63.3));
    assert_eq!(list[1].id, 4);
    assert_eq!(list[1].key, "twistedfate");
}

#[test]
fn test_parse_aram_mayhem_champions_from_next_f_html_normalizes_tiers() {
    let inner = r#"1a:["$","$L32",null,{"champions":[{"key":"ahri","name":"Ahri","image_url":"https://img/Ahri.png","champion_id":103,"id":103,"tier":2,"rank":14},{"key":"twisted_fate_opgg","name":"Twisted Fate OPGG","image_url":"https://img/TF.png","champion_id":4,"id":4,"tier":1,"rank":1},{"key":"ashe","name":"Ashe","image_url":"https://img/Ashe.png","champion_id":22,"id":22,"tier":5,"rank":173}]}]"#;
    let escaped = serde_json::to_string(inner).unwrap();
    let html = format!(r#"<html><script>self.__next_f.push([1,{escaped}])</script></html>"#);

    let list = parse_aram_mayhem_champions_html(&html, &test_ddragon()).unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].id, 4);
    assert_eq!(list[0].key, "twistedfate");
    assert_eq!(list[0].name, "Twisted Fate");
    assert_eq!(list[0].icon_url, "https://ddragon.example/TwistedFate.png");
    assert_eq!(list[0].rank, 1);
    assert_eq!(list[0].tier, 0);
    // Champions absent from DataDragon must never inherit OP.GG image_url
    assert_eq!(list[1].id, 103);
    assert_eq!(list[1].icon_url, "");
    assert_eq!(list[1].rank, 14);
    assert_eq!(list[1].tier, 1);
    assert_eq!(list[2].id, 22);
    assert_eq!(list[2].icon_url, "");
    assert_eq!(list[2].tier, 4);
}

#[test]
fn test_rehydrate_aram_names_overwrites_and_clears_non_ddragon_icons() {
    let mut list = vec![
        AramChampionRank {
            id: 4,
            key: "twistedfate".to_string(),
            name: "Legacy Name".to_string(),
            icon_url: "https://legacy-cdn.example/champion/TwistedFate.png".to_string(),
            tier: 0,
            rank: 1,
            win_rate: None,
            pick_rate: None,
        },
        AramChampionRank {
            id: 999,
            key: "unknown".to_string(),
            name: "Unknown".to_string(),
            icon_url: "https://legacy-cdn.example/champion/Unknown.png".to_string(),
            tier: 3,
            rank: 99,
            win_rate: None,
            pick_rate: None,
        },
    ];

    rehydrate_aram_names(&mut list, &test_ddragon());

    assert_eq!(list[0].name, "Twisted Fate");
    assert_eq!(list[0].icon_url, "https://ddragon.example/TwistedFate.png");
    assert_eq!(list[1].icon_url, "");
}

#[test]
fn test_parse_aram_mayhem_augments_hydrates_from_ddragon_catalog_only() {
    let inner = r#"6c:["$","$L6d",null,{"data":[{"id":1032,"tier":5,"performance":97.14,"popular":0},{"id":1033,"name":"Uncatalogued","tier":5,"performance":50.0,"popular":0,"smallIcon":"https://legacy-cdn.example/x.png","desc":"ignored"},{"id":1074,"tier":3,"performance":76.82,"popular":0.23,"name":"OPGG Wrong Name","smallIcon":"https://legacy-cdn.example/shrink.png","rarity":1,"desc":"OPGG Wrong Desc"},{"id":3075,"tier":0,"performance":99.62,"popular":3.8,"name":"OPGG Recoil","smallIcon":"https://legacy-cdn.example/recoil.png","rarity":1,"desc":"OPGG Desc"},{"id":2062,"tier":0,"performance":114.24,"popular":3.2,"name":"OPGG Echo","smallIcon":"https://legacy-cdn.example/echo.png","rarity":1,"desc":"OPGG Echo Desc"}]}]"#;
    let escaped = serde_json::to_string(inner).unwrap();
    let html = format!(r#"<html><script>self.__next_f.push([1,{escaped}])</script></html>"#);

    let dd = test_ddragon();
    let augments = parse_aram_mayhem_augments_html(&html, &dd).unwrap();
    assert_eq!(augments.len(), 3);
    assert_eq!(augments[0].id, 3075);
    assert_eq!(augments[0].name, "Coice");
    assert_eq!(augments[0].tier, 0);
    assert_eq!(augments[0].rarity, Some(4));
    assert_eq!(augments[0].performance, 99.62);
    assert_eq!(augments[0].popular, 3.8);
    assert_eq!(augments[0].desc, "? causa dano.");
    assert!(
        augments[0]
            .icon_url
            .starts_with("https://raw.communitydragon.org/")
    );
    assert_eq!(augments[1].id, 2062);
    assert_eq!(augments[1].name, "Conjuração Ecoante");
    assert_eq!(augments[1].rarity, Some(8));
    assert!(
        augments[1]
            .icon_url
            .starts_with("https://raw.communitydragon.org/")
    );
    assert_eq!(augments[2].id, 1074);
    assert_eq!(augments[2].name, "Raio Encolhedor");
    assert_eq!(augments[2].desc, "Ao contato Reduz dano.");
    assert!(
        augments[2]
            .icon_url
            .starts_with("https://raw.communitydragon.org/")
    );
}

#[tokio::test]
async fn test_fetch_aram_mayhem_augments_with_mock_server_and_locale_cache_isolation() {
    let mock_server = MockServer::start().await;
    let inner =
        r#"6c:["$","$L6d",null,{"data":[{"id":3075,"tier":0,"performance":99.62,"popular":3.8}]}]"#;
    let html = format!(
        r#"<html><script>self.__next_f.push([1,{}])</script></html>"#,
        serde_json::to_string(inner).unwrap()
    );

    Mock::given(method("GET"))
        .and(path("/pt/twistedfate/augments"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html.clone()))
        .expect(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/en/twistedfate/augments"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let dd_pt = test_ddragon();
    let dd_en = test_ddragon_en();

    let pt_augments = fetch_aram_mayhem_augments_with_base_and_locale(
        &client,
        "Twisted Fate",
        &format!("{}/pt", mock_server.uri()),
        AppLocale::PtBr,
        &dd_pt,
    )
    .await
    .unwrap();

    let en_augments = fetch_aram_mayhem_augments_with_base_and_locale(
        &client,
        "Twisted Fate",
        &format!("{}/en", mock_server.uri()),
        AppLocale::EnUs,
        &dd_en,
    )
    .await
    .unwrap();

    assert_eq!(pt_augments[0].name, "Coice");
    assert_eq!(en_augments[0].name, "Recoil");
    assert!(
        pt_augments[0]
            .icon_url
            .starts_with("https://raw.communitydragon.org/")
    );
    assert!(
        en_augments[0]
            .icon_url
            .starts_with("https://raw.communitydragon.org/")
    );
}

#[tokio::test]
async fn test_aram_augments_disk_cache_and_stale_while_error() {
    let dir = std::env::temp_dir().join(format!(
        "schumazgg_aram_augments_disk_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let mock_server = MockServer::start().await;
    let inner_pt =
        r#"6c:["$","$L6d",null,{"data":[{"id":3075,"tier":0,"performance":99.62,"popular":3.8}]}]"#;
    let html_pt = format!(
        r#"<html><script>self.__next_f.push([1,{}])</script></html>"#,
        serde_json::to_string(inner_pt).unwrap()
    );

    Mock::given(method("GET"))
        .and(path("/pt/ahri/augments"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html_pt))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let dd = test_ddragon();
    let res1 = fetch_aram_mayhem_augments_with_base_locale_and_dir(
        &client,
        "Ahri",
        &format!("{}/pt", mock_server.uri()),
        AppLocale::PtBr,
        &dd,
        Some(&dir),
    )
    .await
    .unwrap();
    assert_eq!(res1[0].name, "Coice");
    let file_path = aram_augments_file_path(&dir, AppLocale::PtBr, "ahri");
    assert!(file_path.exists());

    // Simulate legacy .lz4 entry with stale OP.GG CDN URL and expired timestamp ->
    // Stale-While-Error must rescue on 503 AND rehydrate icon_url to CommunityDragon.
    if let Ok(mut g) = get_augments_cache().write() {
        g.clear();
    }
    let mut legacy = res1.clone();
    legacy[0].icon_url = "https://legacy-cdn.example/recoil.png".to_string();
    legacy[0].name = "Stale Legacy Name".to_string();
    crate::storage::write_lz4_envelope_at(&file_path, "16.25", &legacy, 1).unwrap();

    let offline = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/pt/ahri/augments"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&offline)
        .await;

    let rescued = fetch_aram_mayhem_augments_with_base_locale_and_dir(
        &client,
        "Ahri",
        &format!("{}/pt", offline.uri()),
        AppLocale::PtBr,
        &dd,
        Some(&dir),
    )
    .await
    .unwrap();
    assert_eq!(rescued[0].name, "Coice");
    assert!(
        rescued[0]
            .icon_url
            .starts_with("https://raw.communitydragon.org/")
    );

    let _ = std::fs::remove_dir_all(&dir);
}
