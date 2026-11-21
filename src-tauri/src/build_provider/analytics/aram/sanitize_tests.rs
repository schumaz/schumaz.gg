use super::*;
use crate::build_provider::analytics::aram::augments::fetch_aram_mayhem_augments_with_base_locale_and_dir;
use crate::build_provider::analytics::aram::cache::{aram_augments_file_path, get_augments_cache};
use crate::build_provider::analytics::aram::parsers::parse_aram_mayhem_augments_html_for_locale;
use crate::build_provider::models::DataDragonCache;
use crate::models::{AppLocale, AramAugmentEntry, ArenaAugmentMetadata};
use std::collections::HashMap;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn test_clean_augment_description_resolves_ability_slots_and_placeholders_en_us() {
    // Mercy's Strike (dynamic ability slot)
    assert_eq!(
        clean_augment_description_with_context(
            "After healing or shielding your ally with <font color='#F0C200'>?</font>, your next basic attack gains attack range.",
            AppLocale::EnUs,
            None,
        ),
        "After healing or shielding your ally with an Ability, your next basic attack gains attack range."
    );

    // THRASH / DOOM with known slot E / R
    assert_eq!(
        clean_augment_description_with_context(
            "<evolve>Empower</evolve> <font color='#F0C200'>?</font>!<br><br><evolve>Empowered</evolve> <font color='#F0C200'>?</font> gains <scaleAbilityHaste>20 Ability Haste</scaleAbilityHaste>.",
            AppLocale::EnUs,
            Some("E"),
        ),
        "Empower E! Empowered E gains 20 Ability Haste."
    );

    // Echo Cast / Spell Split ("your ?")
    assert_eq!(
        clean_augment_description_with_context(
            "Casting your <font color='#F0C200'>?</font> sends a clone toward your mouse position.",
            AppLocale::EnUs,
            None,
        ),
        "Casting your Ability sends a clone toward your mouse position."
    );
    assert_eq!(
        clean_augment_description_with_context(
            "Your <font color='#F0C200'>?</font> missile splits into two.",
            AppLocale::EnUs,
            None,
        ),
        "Your Ability missile splits into two."
    );

    // Trusty Weapon (sentence-start slot)
    assert_eq!(
        clean_augment_description_with_context(
            "Striking enemies with <font color='#F0C200'>?</font> builds Friendship.<br><br><font color='#F0C200'>?</font> gains bonus damage.",
            AppLocale::EnUs,
            None,
        ),
        "Striking enemies with an Ability builds Friendship. An Ability gains bonus damage."
    );

    // Numeric placeholders: Scoped Weapons, Zealot, Ok Boomerang, Glass Cannon, Final City Transit, Urf's Champion
    assert_eq!(
        clean_augment_description_with_context("Gain ? Attack Range.", AppLocale::EnUs, None),
        "Gain bonus Attack Range."
    );
    assert_eq!(
        clean_augment_description_with_context(
            "Gain <attention>? Critical Strike Chance</attention> and <attackSpeed>? Attack Speed</attackSpeed>.",
            AppLocale::EnUs,
            None,
        ),
        "Gain bonus Critical Strike Chance and bonus Attack Speed."
    );
    assert_eq!(
        clean_augment_description_with_context(
            "<keywordMajor>Autocast</keywordMajor> throw a boomerang every ?s.",
            AppLocale::EnUs,
            None,
        ),
        "Autocast throw a boomerang periodically."
    );
    assert_eq!(
        clean_augment_description_with_context(
            "Reduce max Health by 30%. Deal an extra ? damage as <trueDamage>true damage</trueDamage>.",
            AppLocale::EnUs,
            None,
        ),
        "Reduce max Health by 30%. Deal bonus damage as true damage."
    );
    assert_eq!(
        clean_augment_description_with_context(
            "Launch a train dealing <physicalDamage>? physical damage</physicalDamage> to enemies.",
            AppLocale::EnUs,
            None,
        ),
        "Launch a train dealing physical damage to enemies."
    );
    assert_eq!(
        clean_augment_description_with_context(
            "<font color='#F0C200'>QUEST:</font> Score ? takedowns.<br><br>REWARD: Golden Spatula",
            AppLocale::EnUs,
            None,
        ),
        "QUEST: Score takedowns. REWARD: Golden Spatula"
    );
}

#[test]
fn test_clean_augment_description_resolves_ability_slots_and_placeholders_pt_br() {
    assert_eq!(
        clean_augment_description_with_context(
            "<evolve>Fortalece</evolve> <font color='#F0C200'>?</font>!<br><br><evolve>Fortalecer</evolve> <font color='#F0C200'>?</font> concede 20 de Aceleração.",
            AppLocale::PtBr,
            Some("R"),
        ),
        "Fortalece R! Fortalecer R concede 20 de Aceleração."
    );
    assert_eq!(
        clean_augment_description_with_context(
            "Conjurar <font color='#F0C200'>?</font> envia um clone.",
            AppLocale::PtBr,
            None,
        ),
        "Conjurar uma habilidade envia um clone."
    );
    assert_eq!(
        clean_augment_description_with_context(
            "Recebe <attention>? de Chance de Acerto Crítico</attention> e <attackSpeed>? de Velocidade de Ataque</attackSpeed>.",
            AppLocale::PtBr,
            None,
        ),
        "Recebe bônus de Chance de Acerto Crítico e bônus de Velocidade de Ataque."
    );
    assert_eq!(
        clean_augment_description_with_context(
            "Arremessa um bumerangue a cada ?s.",
            AppLocale::PtBr,
            None,
        ),
        "Arremessa um bumerangue periodicamente."
    );
    assert_eq!(
        clean_augment_description_with_context(
            "Causa ? de dano adicional como Dano Verdadeiro.",
            AppLocale::PtBr,
            None,
        ),
        "Causa dano adicional como Dano Verdadeiro."
    );
    assert_eq!(
        clean_augment_description_with_context(
            "Causando <physicalDamage>? de Dano Físico</physicalDamage> aos inimigos.",
            AppLocale::PtBr,
            None,
        ),
        "Causando Dano Físico aos inimigos."
    );
}

#[test]
fn test_placeholder_detection_preserves_legitimate_question_marks_and_triple_q_name() {
    assert!(!is_internal_augment_key("???"));
    assert!(!is_internal_augment_key("Mercy's Strike"));
    assert!(is_internal_augment_key("GoH_PentakillAug_Generic"));

    assert!(has_unresolved_augment_placeholder(
        "After healing your ally with ?, your next attack..."
    ));
    assert!(has_unresolved_augment_placeholder(
        "Empower ?! Empowered ? gains 20."
    ));
    assert!(has_unresolved_augment_placeholder("Gain ? Attack Range."));
    assert!(has_unresolved_augment_placeholder("every ?s."));
    assert!(!has_unresolved_augment_placeholder("Ready for battle?"));
    assert!(!has_unresolved_augment_placeholder(
        "Your Missing Pings launch a missile."
    ));
}

#[tokio::test]
async fn test_parser_skips_internal_key_and_cache_auto_refreshes_placeholder_entries() {
    let dir = std::env::temp_dir().join(format!(
        "schumazgg_aram_placeholder_refresh_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let mut arena_augments = HashMap::new();
    arena_augments.insert(
        3172,
        ArenaAugmentMetadata {
            id: 3172,
            name: "THRASH".to_string(),
            desc: String::new(),
            icon_url: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/thrash_small.png".to_string(),
            rarity: 4,
            ability_slot: Some("E".to_string()),
        },
    );
    arena_augments.insert(
        3176,
        ArenaAugmentMetadata {
            id: 3176,
            name: "GoH_PentakillAug_Generic".to_string(),
            desc: String::new(),
            icon_url: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/generic_small.png".to_string(),
            rarity: 4,
            ability_slot: None,
        },
    );
    let dd = DataDragonCache {
        version: "16.22.1".to_string(),
        arena_augments,
        ..Default::default()
    };

    let inner = r#"6c:["$","$L6d",null,{"data":[{"id":3172,"tier":0,"performance":99.0,"popular":3.5,"desc":"<evolve>Empower</evolve> <font color='#F0C200'>?</font>!<br><br><evolve>Empowered</evolve> <font color='#F0C200'>?</font> gains 20 Ability Haste."},{"id":3176,"tier":0,"performance":95.0,"popular":2.5,"desc":"Empower ?"}]}]"#;
    let html = format!(
        r#"<html><script>self.__next_f.push([1,{}])</script></html>"#,
        serde_json::to_string(inner).unwrap()
    );

    let parsed = parse_aram_mayhem_augments_html_for_locale(&html, &dd, AppLocale::EnUs).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].name, "THRASH");
    assert_eq!(
        parsed[0].desc,
        "Empower E! Empowered E gains 20 Ability Haste."
    );

    // Seed a fresh disk cache containing unresolved `?` -> must trigger network refresh
    let file_path = aram_augments_file_path(&dir, AppLocale::EnUs, "sona");
    let dirty_cache = vec![AramAugmentEntry {
        id: 3172,
        name: "THRASH".to_string(),
        tier: 0,
        rarity: Some(4),
        performance: 99.0,
        popular: 3.5,
        icon_url: dd.arena_augments[&3172].icon_url.clone(),
        desc: "Empower ?! Empowered ? gains 20 Ability Haste.".to_string(),
    }];
    crate::storage::write_lz4_envelope(&file_path, "", &dirty_cache).unwrap();
    if let Ok(mut g) = get_augments_cache().write() {
        g.clear();
    }

    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/en/sona/augments"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let refreshed = fetch_aram_mayhem_augments_with_base_locale_and_dir(
        &client,
        "Sona",
        &format!("{}/en", mock_server.uri()),
        AppLocale::EnUs,
        &dd,
        Some(&dir),
    )
    .await
    .unwrap();
    assert_eq!(refreshed.len(), 1);
    assert_eq!(
        refreshed[0].desc,
        "Empower E! Empowered E gains 20 Ability Haste."
    );

    let _ = std::fs::remove_dir_all(&dir);
}
