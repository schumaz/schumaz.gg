use super::cache::get_augments_cache;
use super::*;
use crate::models::ChampionMetadata;
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
    DataDragonCache {
        version: "16.22.1".to_string(),
        champions,
        runes: HashMap::new(),
        spells: HashMap::new(),
    }
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
    let inner = r#"1a:["$","$L32",null,{"champions":[{"key":"ahri","name":"Ahri","image_url":"https://img/Ahri.png","champion_id":103,"id":103,"tier":2,"rank":14},{"key":"twistedfate","name":"Twisted Fate","image_url":"https://img/TF.png","champion_id":4,"id":4,"tier":1,"rank":1},{"key":"ashe","name":"Ashe","image_url":"https://img/Ashe.png","champion_id":22,"id":22,"tier":5,"rank":173}]}]"#;
    let escaped = serde_json::to_string(inner).unwrap();
    let html = format!(r#"<html><script>self.__next_f.push([1,{escaped}])</script></html>"#);

    let list = parse_aram_mayhem_champions_html(&html, &test_ddragon()).unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].id, 4);
    assert_eq!(list[0].key, "twistedfate");
    assert_eq!(list[0].rank, 1);
    assert_eq!(list[0].tier, 0);
    assert_eq!(list[1].id, 103);
    assert_eq!(list[1].rank, 14);
    assert_eq!(list[1].tier, 1);
    assert_eq!(list[2].id, 22);
    assert_eq!(list[2].tier, 4);
}

#[test]
fn test_parse_aram_mayhem_augments_skips_entries_missing_name() {
    let inner = r#"6c:["$","$L6d",null,{"data":[{"id":1032,"tier":5,"performance":97.14,"popular":0},{"id":1033,"name":null,"tier":5,"performance":50.0,"popular":0,"desc":null},{"id":1074,"tier":3,"performance":76.82,"popular":0.23,"name":"Raio Encolhedor","smallIcon":"https://img/shrink.png","rarity":4,"desc":"<OnHit>Ao contato</OnHit><br />Reduz dano."},{"id":3075,"tier":0,"performance":99.62,"popular":3.8,"name":"Coice","smallIcon":"https://img/recoil.png","rarity":4,"desc":"<font color='#F0C200'>?</font> causa dano."},{"id":2062,"tier":0,"performance":114.24,"popular":3.2,"name":"Conjuração Ecoante","smallIcon":"https://img/echo.png","rarity":8,"desc":"Conjura duas vezes."}]}]"#;
    let escaped = serde_json::to_string(inner).unwrap();
    let html = format!(r#"<html><script>self.__next_f.push([1,{escaped}])</script></html>"#);

    let augments = parse_aram_mayhem_augments_html(&html).unwrap();
    assert_eq!(augments.len(), 3);
    assert_eq!(augments[0].name, "Coice");
    assert_eq!(augments[0].tier, 0);
    assert_eq!(augments[0].rarity, Some(4));
    assert_eq!(augments[0].desc, "? causa dano.");
    assert_eq!(augments[1].name, "Conjuração Ecoante");
    assert_eq!(augments[1].rarity, Some(8));
    assert_eq!(augments[2].name, "Raio Encolhedor");
    assert_eq!(augments[2].desc, "Ao contato Reduz dano.");
}

#[tokio::test]
async fn test_fetch_aram_mayhem_augments_with_mock_server_and_locale_cache_isolation() {
    let mock_server = MockServer::start().await;
    let inner_pt = r#"6c:["$","$L6d",null,{"data":[{"id":3075,"tier":0,"performance":99.62,"popular":3.8,"name":"Coice","smallIcon":"https://img/recoil.png","rarity":4,"desc":"Dano extra."}]}]"#;
    let html_pt = format!(
        r#"<html><script>self.__next_f.push([1,{}])</script></html>"#,
        serde_json::to_string(inner_pt).unwrap()
    );

    let inner_en = r#"6c:["$","$L6d",null,{"data":[{"id":3075,"tier":0,"performance":99.62,"popular":3.8,"name":"Recoil","smallIcon":"https://img/recoil.png","rarity":4,"desc":"Extra damage."}]}]"#;
    let html_en = format!(
        r#"<html><script>self.__next_f.push([1,{}])</script></html>"#,
        serde_json::to_string(inner_en).unwrap()
    );

    Mock::given(method("GET"))
        .and(path("/pt/twistedfate/augments"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html_pt))
        .expect(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/en/twistedfate/augments"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html_en))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let pt_augments = fetch_aram_mayhem_augments_with_base_and_locale(
        &client,
        "Twisted Fate",
        &format!("{}/pt", mock_server.uri()),
        AppLocale::PtBr,
    )
    .await
    .unwrap();

    let en_augments = fetch_aram_mayhem_augments_with_base_and_locale(
        &client,
        "Twisted Fate",
        &format!("{}/en", mock_server.uri()),
        AppLocale::EnUs,
    )
    .await
    .unwrap();

    assert_eq!(pt_augments[0].name, "Coice");
    assert_eq!(en_augments[0].name, "Recoil");
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
    let inner_pt = r#"6c:["$","$L6d",null,{"data":[{"id":3075,"tier":0,"performance":99.62,"popular":3.8,"name":"Coice","smallIcon":"https://img/recoil.png","rarity":4,"desc":"Dano extra."}]}]"#;
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
    let res1 = fetch_aram_mayhem_augments_with_base_locale_and_dir(
        &client,
        "Ahri",
        &format!("{}/pt", mock_server.uri()),
        AppLocale::PtBr,
        Some(&dir),
    )
    .await
    .unwrap();
    assert_eq!(res1[0].name, "Coice");
    let file_path = aram_augments_file_path(&dir, AppLocale::PtBr, "ahri");
    assert!(file_path.exists());

    // Clear RAM and expire disk timestamp -> Stale-While-Error rescues on 503
    if let Ok(mut g) = get_augments_cache().write() {
        g.clear();
    }
    crate::storage::write_lz4_envelope_at(&file_path, "16.25", &res1, 1).unwrap();

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
        Some(&dir),
    )
    .await
    .unwrap();
    assert_eq!(rescued[0].name, "Coice");

    let _ = std::fs::remove_dir_all(&dir);
}
