use super::cache::{get_augments_cache, rehydrate_aram_augments, rehydrate_aram_names};
use super::*;
use crate::models::{AramAugmentEntry, ArenaAugmentMetadata, ChampionMetadata};
use std::collections::HashMap;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CDRAGON_AUG_PREFIX: &str = "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/";

fn aug_meta(id: i32, name: &str, desc: &str, icon: &str, rarity: i32) -> ArenaAugmentMetadata {
    ArenaAugmentMetadata {
        id,
        name: name.to_string(),
        desc: desc.to_string(),
        icon_url: format!("{CDRAGON_AUG_PREFIX}{icon}"),
        rarity,
    }
}

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
        aug_meta(1074, "Raio Encolhedor", "", "shrinkray_small.png", 4),
    );
    arena_augments.insert(
        2062,
        aug_meta(
            2062,
            "Conjuração Ecoante",
            "Conjura duas vezes.",
            "echo_small.png",
            8,
        ),
    );
    arena_augments.insert(3075, aug_meta(3075, "Coice", "", "recoil_small.png", 4));

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
    dd.arena_augments
        .insert(3075, aug_meta(3075, "Recoil", "", "recoil_small.png", 4));
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
    assert_eq!(list[1].id, 103);
    assert_eq!(list[1].icon_url, "");
    assert_eq!(list[1].rank, 14);
    assert_eq!(list[1].tier, 1);
    assert_eq!(list[2].id, 22);
    assert_eq!(list[2].icon_url, "");
    assert_eq!(list[2].tier, 4);
}

#[test]
fn test_rehydrate_aram_names_and_augments_preserve_desc_and_clean_icons() {
    let dd = test_ddragon();
    let mut champs = vec![
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
    rehydrate_aram_names(&mut champs, &dd);
    assert_eq!(champs[0].name, "Twisted Fate");
    assert_eq!(
        champs[0].icon_url,
        "https://ddragon.example/TwistedFate.png"
    );
    assert_eq!(champs[1].icon_url, "");

    let mut augs = vec![AramAugmentEntry {
        id: 3075,
        name: "Legacy".to_string(),
        tier: 0,
        rarity: Some(1),
        performance: 99.0,
        popular: 3.8,
        icon_url: "https://legacy-cdn.example/recoil.png".to_string(),
        desc: "Descrição preservada.".to_string(),
    }];
    rehydrate_aram_augments(&mut augs, &dd);
    assert_eq!(augs[0].name, "Coice");
    assert_eq!(augs[0].rarity, Some(4));
    assert!(
        augs[0]
            .icon_url
            .starts_with("https://raw.communitydragon.org/")
    );
    assert_eq!(augs[0].desc, "Descrição preservada.");
}

#[test]
fn test_parse_aram_mayhem_augments_hydrates_assets_from_cdragon_and_falls_back_desc() {
    let inner = r#"6c:["$","$L6d",null,{"data":[{"id":1032,"tier":5,"performance":97.14,"popular":0},{"id":1033,"name":"Uncatalogued","tier":5,"performance":50.0,"popular":0,"smallIcon":"https://legacy-cdn.example/x.png","desc":"ignored"},{"id":1074,"tier":3,"performance":76.82,"popular":0.23,"name":"OPGG Wrong","smallIcon":"https://legacy-cdn.example/shrink.png","rarity":1,"desc":"<OnHit>Ao contato</OnHit><br />Reduz dano."},{"id":3075,"tier":0,"performance":99.62,"popular":3.8,"name":"OPGG Recoil","smallIcon":"https://legacy-cdn.example/recoil.png","rarity":1,"desc":"<font color='#F0C200'>?</font> causa dano."},{"id":2062,"tier":0,"performance":114.24,"popular":3.2,"name":"OPGG Echo","smallIcon":"https://legacy-cdn.example/echo.png","rarity":1,"desc":"OPGG Echo Desc"}]}]"#;
    let escaped = serde_json::to_string(inner).unwrap();
    let html = format!(r#"<html><script>self.__next_f.push([1,{escaped}])</script></html>"#);

    let augments = parse_aram_mayhem_augments_html(&html, &test_ddragon()).unwrap();
    assert_eq!(augments.len(), 3);
    // meta.desc empty -> falls back to sanitized item.desc, while icon/name/rarity come from CDragon
    assert_eq!(augments[0].id, 3075);
    assert_eq!(augments[0].name, "Coice");
    assert_eq!(augments[0].rarity, Some(4));
    assert_eq!(augments[0].desc, "? causa dano.");
    assert!(
        augments[0]
            .icon_url
            .starts_with("https://raw.communitydragon.org/")
    );
    // meta.desc non-empty -> prefers CDragon meta.desc over item.desc
    assert_eq!(augments[1].id, 2062);
    assert_eq!(augments[1].name, "Conjuração Ecoante");
    assert_eq!(augments[1].rarity, Some(8));
    assert_eq!(augments[1].desc, "Conjura duas vezes.");
    // meta.desc empty -> sanitized HTML tags stripped cleanly
    assert_eq!(augments[2].id, 1074);
    assert_eq!(augments[2].name, "Raio Encolhedor");
    assert_eq!(augments[2].desc, "Ao contato Reduz dano.");
}

#[tokio::test]
async fn test_fetch_aram_mayhem_augments_with_mock_server_and_locale_cache_isolation() {
    let mock_server = MockServer::start().await;
    let html_for = |desc: &str| {
        let inner = format!(
            r#"6c:["$","$L6d",null,{{"data":[{{"id":3075,"tier":0,"performance":99.62,"popular":3.8,"desc":"{desc}"}}]}}]"#
        );
        format!(
            r#"<html><script>self.__next_f.push([1,{}])</script></html>"#,
            serde_json::to_string(&inner).unwrap()
        )
    };

    Mock::given(method("GET"))
        .and(path("/pt/twistedfate/augments"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html_for("Dano extra.")))
        .expect(1)
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/en/twistedfate/augments"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html_for("Extra damage.")))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let pt_augments = fetch_aram_mayhem_augments_with_base_and_locale(
        &client,
        "Twisted Fate",
        &format!("{}/pt", mock_server.uri()),
        AppLocale::PtBr,
        &test_ddragon(),
    )
    .await
    .unwrap();
    let en_augments = fetch_aram_mayhem_augments_with_base_and_locale(
        &client,
        "Twisted Fate",
        &format!("{}/en", mock_server.uri()),
        AppLocale::EnUs,
        &test_ddragon_en(),
    )
    .await
    .unwrap();

    assert_eq!(pt_augments[0].name, "Coice");
    assert_eq!(pt_augments[0].desc, "Dano extra.");
    assert_eq!(en_augments[0].name, "Recoil");
    assert_eq!(en_augments[0].desc, "Extra damage.");
}

#[tokio::test]
async fn test_aram_augments_disk_cache_backfills_blank_desc_and_stale_while_error() {
    let dir = std::env::temp_dir().join(format!(
        "schumazgg_aram_augments_disk_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let mock_server = MockServer::start().await;
    let inner_pt = r#"6c:["$","$L6d",null,{"data":[{"id":3075,"tier":0,"performance":99.62,"popular":3.8,"desc":"Dano extra."}]}]"#;
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
    let file_path = aram_augments_file_path(&dir, AppLocale::PtBr, "ahri");

    // Seed a fresh .lz4 file with empty descriptions -> must auto-refresh from network to backfill
    let blank_cache = vec![AramAugmentEntry {
        id: 3075,
        name: "Coice".to_string(),
        tier: 0,
        rarity: Some(4),
        performance: 99.62,
        popular: 3.8,
        icon_url: format!("{CDRAGON_AUG_PREFIX}recoil_small.png"),
        desc: String::new(),
    }];
    crate::storage::write_lz4_envelope(&file_path, "", &blank_cache).unwrap();

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
    assert_eq!(res1[0].desc, "Dano extra.");

    // Clear RAM, expire disk envelope with legacy icon -> Stale-While-Error rescues & rehydrates icon
    if let Ok(mut g) = get_augments_cache().write() {
        g.clear();
    }
    let mut legacy = res1.clone();
    legacy[0].icon_url = "https://legacy-cdn.example/recoil.png".to_string();
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
    assert_eq!(rescued[0].desc, "Dano extra.");
    assert!(
        rescued[0]
            .icon_url
            .starts_with("https://raw.communitydragon.org/")
    );

    let _ = std::fs::remove_dir_all(&dir);
}
