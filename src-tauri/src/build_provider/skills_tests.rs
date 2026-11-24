use super::*;
use crate::build_provider::analytics::{parse_opgg_analysis_payload, parse_opgg_mcp_text};
use crate::build_provider::ddragon::enrich_build_with_ddragon_with_flash_key_and_locale;
use crate::lcu_injector::FlashKeyPreference;
use crate::models::{AppLocale, ChampionAbilityMetadata};
use std::collections::HashMap;

fn make_ahri_abilities(lang: AppLocale) -> Vec<ChampionAbilityMetadata> {
    let (q, w, e, r) = match lang {
        AppLocale::PtBr => (
            "Orbe da Ilusão",
            "Fogo de Raposa",
            "Encanto",
            "Ímpeto Espiritual",
        ),
        AppLocale::EnUs => ("Orb of Deception", "Fox-Fire", "Charm", "Spirit Rush"),
    };
    [("Q", q, 5), ("W", w, 5), ("E", e, 5), ("R", r, 3)]
        .into_iter()
        .map(|(key, name, max_rank)| ChampionAbilityMetadata {
            key: key.to_string(),
            name: name.to_string(),
            icon_url: format!("https://ddragon.leagueoflegends.com/cdn/16.22.1/img/spell/Ahri{key}.png"),
            description: format!("{name} description"),
            cooldown_burn: "7".to_string(),
            max_rank,
        })
        .collect()
}

#[test]
fn test_extract_rest_skill_order_prefers_highest_play_and_expands_15_to_18() {
    let payload = serde_json::json!({
        "skill_masteries": [
            {
                "ids": ["Q", "W", "E"],
                "play": 11871,
                "win": 6894,
                "pick_rate": 0.1743,
                "builds": [
                    { "order": ["Q","W","E","Q","W","R","W","Q","W","R","W","Q","Q","E","E"], "play": 3917, "win": 2280, "pick_rate": 0.33 }
                ]
            },
            {
                "ids": ["Q", "E", "W"],
                "play": 53930,
                "win": 32799,
                "pick_rate": 0.7917,
                "builds": [
                    { "order": ["Q","E","W","Q","Q","R","Q","Q","E","R","E","E","E","W","W"], "play": 2949, "win": 1777, "pick_rate": 0.0547 },
                    { "order": ["Q","W","E","Q","Q","R","Q","Q","E","R","E","E","E","W","W"], "play": 40197, "win": 24404, "pick_rate": 0.7454 }
                ]
            }
        ],
        "skills": [
            { "order": ["Q","W","E","Q","Q","R","Q","Q","E","R","E","E","E","W","W"], "play": 40197, "win": 24404, "pick_rate": 0.5901 }
        ]
    });

    let rec = extract_rest_skill_order(&payload).expect("Should extract REST skill_masteries");
    assert_eq!(rec.priority, vec!["Q", "E", "W"]);
    assert_eq!(rec.play, 53930);
    assert_eq!(rec.win, 32799);
    assert_eq!(rec.pick_rate, 79.2);
    assert_eq!(rec.win_rate, 60.8);
    assert_eq!(rec.order.len(), 18);
    assert_eq!(
        &rec.order[..15],
        &["Q", "W", "E", "Q", "Q", "R", "Q", "Q", "E", "R", "E", "E", "E", "W", "W"]
    );
    assert_eq!(&rec.order[15..18], &["R", "W", "W"]);
}

#[test]
fn test_extract_rest_skill_order_derives_priority_from_skills_fallback() {
    let payload = serde_json::json!({
        "skills": [
            {
                "order": ["Q","W","E","Q","Q","R","Q","Q","W","R","W","W","W","E","E"],
                "play": 12000,
                "win": 6480,
                "pick_rate": 0.625
            }
        ]
    });

    let rec = extract_rest_skill_order(&payload).expect("Should fall back to skills array");
    assert_eq!(rec.priority, vec!["Q", "W", "E"]);
    assert_eq!(rec.pick_rate, 62.5);
    assert_eq!(rec.win_rate, 54.0);
    assert_eq!(rec.order.len(), 18);
    assert_eq!(&rec.order[15..18], &["R", "E", "E"]);
}

#[test]
fn test_special_kits_yuumi_6rank_jayce_no_r_aphelios_rejected_and_bounds() {
    // 1. Yuumi-style 6-rank Q reaches 18 cleanly using DataDragon max_rank (Q=6, W=5, E=5, R=3)
    let mut yuumi_order: Vec<String> = [
        "E", "Q", "E", "E", "Q", "R", "E", "E", "Q", "R", "Q", "Q", "Q", "W", "W",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    let yuumi_abilities = vec![
        ChampionAbilityMetadata { key: "Q".into(), name: "Q".into(), max_rank: 6, ..Default::default() },
        ChampionAbilityMetadata { key: "W".into(), name: "W".into(), max_rank: 5, ..Default::default() },
        ChampionAbilityMetadata { key: "E".into(), name: "E".into(), max_rank: 5, ..Default::default() },
        ChampionAbilityMetadata { key: "R".into(), name: "R".into(), max_rank: 3, ..Default::default() },
    ];
    expand_skill_order_to_18(&mut yuumi_order, &["Q".into(), "E".into(), "W".into()], Some(&yuumi_abilities));
    assert_eq!(yuumi_order.len(), 18);
    assert_eq!(&yuumi_order[15..18], &["R", "W", "W"]);

    // 2. Jayce-style 0-rank ultimate must NEVER insert R at level 16
    let jayce_payload = serde_json::json!({
        "skill_masteries": [{
            "ids": ["Q", "E", "W"],
            "play": 10000,
            "win": 5200,
            "pick_rate": 0.75,
            "builds": [{
                "order": ["Q","E","W","Q","Q","E","Q","E","Q","E","Q","E","E","W","W"],
                "play": 6000,
                "win": 3100,
                "pick_rate": 0.6
            }]
        }]
    });
    let jayce_rec = extract_rest_skill_order(&jayce_payload).expect("Jayce skill order should parse");
    assert_eq!(jayce_rec.order.len(), 18);
    assert!(!jayce_rec.order.iter().any(|k| k == "R"));
    assert_eq!(jayce_rec.order.iter().filter(|k| *k == "Q").count(), 6);

    // 3. Aphelios stat upgrade tokens ("AD", "AS", "ARPEN") are rejected cleanly -> None
    let aphelios_payload = serde_json::json!({
        "skill_masteries": [{ "ids": ["AD", "AS", "ARPEN"], "play": 5000, "win": 2500, "pick_rate": 0.5 }],
        "skills": [{ "order": ["AD", "AD", "AS"], "play": 5000, "win": 2500, "pick_rate": 0.5 }]
    });
    assert!(extract_rest_skill_order(&aphelios_payload).is_none());

    // 4. Short (<15) sequence is NOT fabricated to 18; already-18 sequence is preserved
    let mut short_seq = vec!["Q".into(), "W".into(), "E".into()];
    expand_skill_order_to_18(&mut short_seq, &["Q".into(), "W".into(), "E".into()], None);
    assert_eq!(short_seq.len(), 3);

    let mut full_18: Vec<String> = ["Q","W","E","Q","Q","R","Q","E","Q","E","R","E","E","W","W","R","W","W"]
        .into_iter().map(str::to_string).collect();
    let expected_18 = full_18.clone();
    expand_skill_order_to_18(&mut full_18, &["Q".into(), "E".into(), "W".into()], None);
    assert_eq!(full_18, expected_18);
}

#[test]
fn test_extract_mcp_skill_order_with_and_without_headers_and_skills_only() {
    let mcp_with_headers = "class LolGetChampionAnalysis: data\nclass Data: skill_masteries,skills\nclass SkillMasteries: ids,play,win,pick_rate,builds\nclass Builds: order,play,win,pick_rate\nclass Skills: order,play,win,pick_rate\n\nLolGetChampionAnalysis(Data([SkillMasteries([\"Q\",\"E\",\"W\"],42000,22100,0.764,[Builds([\"Q\",\"W\",\"E\",\"Q\",\"Q\",\"R\",\"Q\",\"Q\",\"E\",\"R\",\"E\",\"E\",\"E\",\"W\",\"W\"],31000,16400,0.738)])],[Skills([\"Q\",\"W\",\"E\",\"Q\",\"Q\",\"R\",\"Q\",\"Q\",\"E\",\"R\",\"E\",\"E\",\"E\",\"W\",\"W\"],31000,16400,0.56)]))";
    let rec = extract_mcp_skill_order(mcp_with_headers).expect("MCP headers should parse");
    assert_eq!(rec.priority, vec!["Q", "E", "W"]);
    assert_eq!(rec.play, 42000);
    assert_eq!(rec.pick_rate, 76.4);
    assert_eq!(rec.win_rate, 52.6);
    assert_eq!(rec.order.len(), 18);

    // Structural fallback without class headers
    let mcp_no_headers = "LolGetChampionAnalysis(Data([SkillMastery([\"W\",\"Q\",\"E\"],15000,7800,0.55,[Build([\"W\",\"Q\",\"E\",\"W\",\"W\",\"R\",\"W\",\"W\",\"Q\",\"R\",\"Q\",\"Q\",\"Q\",\"E\",\"E\"],9000,4700,0.6)])]))";
    let rec_struct = extract_mcp_skill_order(mcp_no_headers).expect("Structural MCP should parse");
    assert_eq!(rec_struct.priority, vec!["W", "Q", "E"]);
    assert_eq!(rec_struct.order.len(), 18);
    assert_eq!(&rec_struct.order[15..18], &["R", "E", "E"]);

    // Skills-only MCP fallback
    let mcp_skills_only = "class Skills: order,play,win,pick_rate\nLolGetChampionAnalysis(Data([Skills([\"E\",\"Q\",\"W\",\"E\",\"E\",\"R\",\"E\",\"E\",\"Q\",\"R\",\"Q\",\"Q\",\"Q\",\"W\",\"W\"],10000,5300,0.48)]))";
    let rec_skills = extract_mcp_skill_order(mcp_skills_only).expect("Skills-only MCP should parse");
    assert_eq!(rec_skills.priority, vec!["E", "Q", "W"]);
    assert_eq!(rec_skills.order.len(), 18);
}

#[test]
fn test_parsers_and_ddragon_enrichment_populate_and_localize_skill_order() {
    let rest_json = serde_json::json!({
        "data": {
            "runes": { "primary_style_id": 8100, "sub_style_id": 8200, "primary_perk_ids": [8112,8126,8138,8135], "sub_perk_ids": [8226,8210], "stat_shards": [5008,5008,5001] },
            "spells": [4, 14],
            "items": { "starters": [1056, 2003], "core": [3118, 4645, 3157], "boots": [3020] },
            "skill_masteries": [{
                "ids": ["Q", "W", "E"], "play": 50000, "win": 26500, "pick_rate": 0.812,
                "builds": [{ "order": ["Q","W","E","Q","Q","R","Q","Q","W","R","W","W","W","E","E"], "play": 36000, "win": 19200, "pick_rate": 0.72 }]
            }]
        }
    });
    let mut rest_build = parse_opgg_analysis_payload(&rest_json, 103, "Ahri").unwrap();
    assert_eq!(rest_build.skill_order.as_ref().unwrap().priority, vec!["Q", "W", "E"]);

    let mut pt_abilities = HashMap::new();
    pt_abilities.insert(103, make_ahri_abilities(AppLocale::PtBr));
    let pt_cache = DataDragonCache { version: "16.22.1".into(), champion_abilities: pt_abilities, ..Default::default() };
    enrich_build_with_ddragon_with_flash_key_and_locale(&mut rest_build, &pt_cache, FlashKeyPreference::D, AppLocale::PtBr);
    assert_eq!(rest_build.skill_order.as_ref().unwrap().abilities[0].name, "Orbe da Ilusão");

    let mut en_abilities = HashMap::new();
    en_abilities.insert(103, make_ahri_abilities(AppLocale::EnUs));
    let en_cache = DataDragonCache { version: "16.22.1".into(), champion_abilities: en_abilities, ..Default::default() };
    enrich_build_with_ddragon_with_flash_key_and_locale(&mut rest_build, &en_cache, FlashKeyPreference::D, AppLocale::EnUs);
    assert_eq!(rest_build.skill_order.as_ref().unwrap().abilities[0].name, "Orb of Deception");

    let mcp_text = "class LolGetChampionAnalysis: data\nclass SkillMasteries: ids,play,win,pick_rate,builds\nclass Builds: order,play,win,pick_rate\nLolGetChampionAnalysis(Data(Summary(AverageStats(0.51,0.12)),CoreItems([3118,4645,3157]),CoreItems([3020]),CoreItems([1056,2003]),CoreItems([4,14]),Runes(8100,[8112,8139,8140,8106],8200,[8210,8226],[5005,5008,5001]),[SkillMasteries([\"Q\",\"W\",\"E\"],40000,21000,0.75,[Builds([\"Q\",\"W\",\"E\",\"Q\",\"Q\",\"R\",\"Q\",\"Q\",\"W\",\"R\",\"W\",\"W\",\"W\",\"E\",\"E\"],30000,15800,0.75)])]))";
    let mcp_build = parse_opgg_mcp_text(mcp_text, 103, "Ahri").unwrap();
    assert_eq!(mcp_build.skill_order.as_ref().unwrap().priority, vec!["Q", "W", "E"]);
    assert_eq!(mcp_build.skill_order.as_ref().unwrap().order.len(), 18);
}
