use super::*;
use crate::build_provider::analytics::aram::augments::fetch_aram_mayhem_augments_with_base_locale_and_dir;
use crate::build_provider::analytics::aram::cache::{
    aram_augments_file_path, get_augments_cache, has_valid_augment_descriptions,
    rehydrate_aram_augments,
};
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

#[test]
fn test_riot_inline_icon_tokens_flagged_and_cleaned_on_rehydration() {
    let mut dirty = vec![AramAugmentEntry {
        id: 1097,
        name: "Witchful Thinking".to_string(),
        tier: 0,
        rarity: Some(1),
        performance: 95.0,
        popular: 2.7,
        icon_url: "https://raw.communitydragon.org/latest/wt.png".to_string(),
        desc: "Gain %i:scaleAP% 60 Ability Power.".to_string(),
    }];
    assert!(!has_valid_augment_descriptions(&dirty));

    let empty_dd = DataDragonCache::default();
    rehydrate_aram_augments(&mut dirty, &empty_dd);
    assert_eq!(dirty[0].desc, "Gain 60 Ability Power.");
    assert!(has_valid_augment_descriptions(&dirty));
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

#[test]
fn test_tooltip_fallback_populates_blank_desc_augments_and_strips_stat_trackers() {
    use crate::build_provider::analytics::aram::clean_augment_tooltip_with_context;
    let meta = |id: i32, name: &str| ArenaAugmentMetadata {
        id,
        name: name.to_string(),
        desc: String::new(),
        icon_url: format!("https://raw.communitydragon.org/latest/{id}.png"),
        rarity: 4,
        ability_slot: None,
    };
    let dd = DataDragonCache {
        version: "16.23.1".to_string(),
        arena_augments: HashMap::from([
            (2096, meta(2096, "Lil' Extra Help")),
            (1071, meta(1071, "Scopier Weapons")),
            (3123, meta(3123, "Jeweled Bracelet")),
            (2054, meta(2054, "Tripleshot")),
            (3060, meta(3060, "One Hit Wonder")),
            (1415, meta(1415, "Twin Fire")),
            (1154, meta(1154, "Urf's Champion")),
        ]),
        ..Default::default()
    };
    let payload = concat!(
        r#"[{"id":2096,"tier":0,"performance":83.1,"popular":5.3,"desc":"","tooltip":"Gain ? Attack Range and ? <attackSpeed>Attack Speed</attackSpeed> during your <font color='#F0C200'>?</font> duration."},"#,
        r#"{"id":1071,"tier":0,"performance":82.8,"popular":4.8,"desc":"","tooltip":"Gain ? Attack Range."},"#,
        r#"{"id":3123,"tier":2,"performance":75.0,"popular":0.9,"desc":"","tooltip":"<font color='#F0C200'>?</font> can <crit>Critically Strike</crit> for 45% increased damage.<br><br>Gain <crit>25% Critical Strike Chance</crit>.<br><br><rules>Damage Dealt to Champions: ?</rules>"},"#,
        r#"{"id":2054,"tier":2,"performance":74.0,"popular":0.7,"desc":"","tooltip":"Your <font color='#F0C200'>?</font> targets 2 additional enemies.<br><br>Additional Targets Hit: ?<br><rules>Units take less damage from the same spell. </rules>"},"#,
        r#"{"id":3060,"tier":2,"performance":70.5,"popular":0.5,"desc":"","tooltip":"<font color='#F0C200'>?</font>'s damage is increased by 75%.<br><br><rules>Bonus Damage: ?%</rules>."},"#,
        r#"{"id":1415,"tier":2,"performance":77.3,"popular":0.4,"desc":"","tooltip":"On ability damage, fire 1 plus ? missiles that deal <magicDamage>? magic damage</magicDamage> each."},"#,
        r#"{"id":1154,"tier":2,"performance":85.1,"popular":0.3,"desc":"","tooltip":"<font color='#F0C200'>QUEST:</font> Score ? takedowns.<br><br>REWARD: Golden Spatula<br><br>Takedowns: ?"}]"#,
    );
    let html = format!(
        r#"<html><script>self.__next_f.push([1,{}])</script></html>"#,
        serde_json::to_string(&format!(r#"6c:["$","$L6d",null,{{"data":{payload}}}]"#)).unwrap()
    );
    let out = parse_aram_mayhem_augments_html_for_locale(&html, &dd, AppLocale::EnUs).unwrap();
    let by_id: HashMap<i32, String> = out.into_iter().map(|a| (a.id, a.desc)).collect();
    assert_eq!(by_id[&2096], "Gain bonus Attack Range and bonus Attack Speed during your Ability duration.");
    assert_eq!(by_id[&1071], "Gain bonus Attack Range.");
    assert_eq!(by_id[&3123], "An Ability can Critically Strike for 45% increased damage. Gain 25% Critical Strike Chance.");
    assert_eq!(by_id[&2054], "Your Ability targets 2 additional enemies. Units take less damage from the same spell.");
    assert_eq!(by_id[&3060], "An Ability's damage is increased by 75%.");
    assert_eq!(by_id[&1415], "On ability damage, fire additional missiles that deal magic damage each.");
    assert_eq!(by_id[&1154], "QUEST: Score takedowns. REWARD: Golden Spatula");
    assert_eq!(
        clean_augment_tooltip_with_context("Recebe ? de Alcance e ? de Velocidade durante <font color='#F0C200'>?</font>.<br><br><rules>Dano causado a Campeões: ?</rules>", AppLocale::PtBr, None),
        "Recebe bônus de Alcance e bônus de Velocidade durante uma habilidade."
    );
}
