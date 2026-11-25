use schumazgg_lib::build_provider::{
    ChampionBuild, ChampionMetadata, DataDragonCache,
    enrich_build_with_ddragon_for_queue_with_flash_key_and_locale, extract_lane_champion_roster,
    get_recommended_build_with_filters_and_endpoints, parse_opgg_analysis_payload_for_mode_and_roster,
    parse_opgg_champion_positions_ranking,
};
use schumazgg_lib::lcu_injector::FlashKeyPreference;
use schumazgg_lib::models::{AppLocale, QUEUE_ARAM, QUEUE_SOLO_DUO};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const AHRI_103_MID_JSON: &str = include_str!("../fixtures/ranked/ahri_103_mid.json");
const AHRI_103_TOP_JSON: &str = include_str!("../fixtures/ranked/ahri_103_top.json");
const ANNIE_1_MID_JSON: &str = include_str!("../fixtures/ranked/annie_1_mid.json");
const RANKED_POSITIONS_JSON: &str = include_str!("../fixtures/ranked/ranked_positions.json");

fn make_matchup_ddragon_cache() -> DataDragonCache {
    let mut champions = HashMap::new();
    for (id, key, name) in [
        (103, "Ahri", "Ahri"),
        (1, "Annie", "Annie"),
        (800, "Mel", "Mel"),
        (105, "Fizz", "Fizz"),
        (142, "Zoe", "Zoe"),
        (893, "Aurora", "Aurora"),
        (268, "Azir", "Azir"),
        (3, "Galio", "Galio"),
        (777, "Yone", "Yone"),
        (39, "Irelia", "Irelia"),
        (34, "Anivia", "Anivia"),
        (131, "Diana", "Diana"),
        (245, "Ekko", "Ekko"),
        (101, "Xerath", "Xerath"),
        (163, "Taliyah", "Taliyah"),
        (13, "Ryze", "Ryze"),
        (157, "Yasuo", "Yasuo"),
        (6, "Urgot", "Urgot"),
        (904, "Zaahen", "Zaahen"),
        (86, "Garen", "Garen"),
        (36, "DrMundo", "Dr. Mundo"),
        (516, "Ornn", "Ornn"),
        (106, "Volibear", "Volibear"),
        (420, "Illaoi", "Illaoi"),
        (897, "KSante", "K'Sante"),
        (85, "Kennen", "Kennen"),
        (82, "Mordekaiser", "Mordekaiser"),
    ] {
        champions.insert(
            id,
            ChampionMetadata {
                champion_id: id,
                key_name: key.to_string(),
                display_name: name.to_string(),
                icon_url: format!(
                    "https://ddragon.leagueoflegends.com/cdn/16.23.1/img/champion/{key}.png"
                ),
            },
        );
    }
    DataDragonCache {
        version: "16.23.1".to_string(),
        champions,
        ..Default::default()
    }
}

fn parse_and_enrich_sr(
    raw_json: &str,
    champion_id: i32,
    champion_name: &str,
    lane: &str,
    roster: &HashSet<i32>,
    dd: &DataDragonCache,
) -> ChampionBuild {
    let mut payload: Value = serde_json::from_str(raw_json).expect("valid real OP.GG REST JSON");
    if let Some(data) = payload.get_mut("data").and_then(|v| v.as_object_mut()) {
        data.insert("position".to_string(), serde_json::json!(lane));
    }
    let mut build = parse_opgg_analysis_payload_for_mode_and_roster(
        &payload,
        champion_id,
        champion_name,
        true,
        Some("CLASSIC"),
        None,
        Some(roster),
    )
    .expect("SR REST payload should parse cleanly");
    build.active_position = lane.to_string();
    enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
        &mut build,
        dd,
        FlashKeyPreference::D,
        AppLocale::PtBr,
        Some(QUEUE_SOLO_DUO),
    );
    build
}

#[test]
fn test_real_ranked_ahri_mid_and_top_and_annie_mid_filter_to_same_lane_roster() {
    let dd = make_matchup_ddragon_cache();
    let pos_json: Value = serde_json::from_str(RANKED_POSITIONS_JSON).expect("valid positions JSON");
    let pos_map = parse_opgg_champion_positions_ranking(&pos_json);
    let mid_roster = extract_lane_champion_roster(&pos_map, "mid");
    let top_roster = extract_lane_champion_roster(&pos_map, "top");

    // 1. Ahri Mid: only Mid laners (Naafiri/Smolder/Pantheon/Malphite/Sion/Ezreal excluded)
    let ahri_mid = parse_and_enrich_sr(AHRI_103_MID_JSON, 103, "Ahri", "mid", &mid_roster, &dd);
    let ahri_mid_m = ahri_mid.lane_matchups.expect("Ahri Mid matchups");
    let ahri_hard: Vec<i32> = ahri_mid_m.hard.iter().map(|m| m.champion_id).collect();
    let ahri_fav: Vec<i32> = ahri_mid_m.favorable.iter().map(|m| m.champion_id).collect();
    assert_eq!(ahri_hard, vec![1, 800, 105, 142, 893]);
    assert_eq!(ahri_fav, vec![268, 3, 777, 39, 34]);
    assert_eq!(ahri_mid_m.hard.iter().map(|m| m.win_rate).collect::<Vec<_>>(), vec![47.4, 48.0, 48.3, 48.6, 48.9]);
    assert_eq!(ahri_mid_m.favorable.iter().map(|m| m.win_rate).collect::<Vec<_>>(), vec![55.3, 55.0, 54.7, 54.1, 53.7]);
    for off_role in [950, 901, 80, 54, 14, 81] {
        assert!(!ahri_hard.contains(&off_role) && !ahri_fav.contains(&off_role));
    }

    // 2. Annie Mid: only Mid laners
    let annie_mid = parse_and_enrich_sr(ANNIE_1_MID_JSON, 1, "Annie", "mid", &mid_roster, &dd);
    let annie_m = annie_mid.lane_matchups.expect("Annie Mid matchups");
    assert_eq!(annie_m.hard.iter().map(|m| m.champion_id).collect::<Vec<_>>(), vec![131, 245, 101, 3, 34]);
    assert_eq!(annie_m.favorable.iter().map(|m| m.champion_id).collect::<Vec<_>>(), vec![39, 777, 163, 13, 157]);

    // 3. Ahri Top: only Top laners
    let ahri_top = parse_and_enrich_sr(AHRI_103_TOP_JSON, 103, "Ahri", "top", &top_roster, &dd);
    let ahri_top_m = ahri_top.lane_matchups.expect("Ahri Top matchups");
    let top_hard: Vec<i32> = ahri_top_m.hard.iter().map(|m| m.champion_id).collect();
    let top_fav: Vec<i32> = ahri_top_m.favorable.iter().map(|m| m.champion_id).collect();
    assert_eq!(top_hard, vec![6, 904, 86, 36, 516]);
    assert_eq!(top_fav, vec![106, 420, 897, 85, 82]);
    assert!(top_hard.iter().chain(top_fav.iter()).all(|id| top_roster.contains(id)));

    // 4. ARAM omits lane_matchups even with counters present
    let ahri_json: Value = serde_json::from_str(AHRI_103_MID_JSON).expect("valid JSON");
    let mut aram_build = parse_opgg_analysis_payload_for_mode_and_roster(
        &ahri_json, 103, "Ahri", true, Some("ARAM"), None, Some(&mid_roster),
    )
    .expect("ARAM parse");
    enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
        &mut aram_build, &dd, FlashKeyPreference::D, AppLocale::PtBr, Some(QUEUE_ARAM),
    );
    assert!(aram_build.lane_matchups.is_none());
}

#[tokio::test]
async fn test_end_to_end_service_filters_ahri_mid_and_top_matchups_via_wiremock() {
    let mock_server = MockServer::start().await;
    let pos_json: Value = serde_json::from_str(RANKED_POSITIONS_JSON).expect("positions JSON");
    let ahri_mid_json: Value = serde_json::from_str(AHRI_103_MID_JSON).expect("ahri mid JSON");
    let ahri_top_json: Value = serde_json::from_str(AHRI_103_TOP_JSON).expect("ahri top JSON");

    Mock::given(method("GET"))
        .and(path("/e2e/global/champions/ranked/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "data": ["16.23"] })))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/e2e/global/champions/ranked"))
        .respond_with(ResponseTemplate::new(200).set_body_json(pos_json))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/e2e/global/champions/ranked/103/mid"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ahri_mid_json))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/e2e/global/champions/ranked/103/top"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ahri_top_json))
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let dd = make_matchup_ddragon_cache();
    let rest_base = format!("{}/e2e", mock_server.uri());
    let mcp_url = format!("{}/mcp", mock_server.uri());

    let mid_build = get_recommended_build_with_filters_and_endpoints(
        &client, 103, QUEUE_SOLO_DUO, Some("mid"), Some("emerald_plus"), Some("global"), Some("16.23"),
        &dd, &rest_base, &mcp_url,
    )
    .await
    .expect("Ahri Mid e2e build");
    let mid_m = mid_build.lane_matchups.expect("Ahri Mid matchups");
    assert_eq!(mid_m.hard.iter().map(|m| m.champion_id).collect::<Vec<_>>(), vec![1, 800, 105, 142, 893]);
    assert_eq!(mid_m.favorable.iter().map(|m| m.champion_id).collect::<Vec<_>>(), vec![268, 3, 777, 39, 34]);

    let top_build = get_recommended_build_with_filters_and_endpoints(
        &client, 103, QUEUE_SOLO_DUO, Some("top"), Some("emerald_plus"), Some("global"), Some("16.23"),
        &dd, &rest_base, &mcp_url,
    )
    .await
    .expect("Ahri Top e2e build");
    let top_m = top_build.lane_matchups.expect("Ahri Top matchups");
    assert_eq!(top_m.hard.iter().map(|m| m.champion_id).collect::<Vec<_>>(), vec![6, 904, 86, 36, 516]);
    assert_eq!(top_m.favorable.iter().map(|m| m.champion_id).collect::<Vec<_>>(), vec![106, 420, 897, 85, 82]);
}
