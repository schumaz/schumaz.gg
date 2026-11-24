use super::tooltip_parser::{
    extract_item_plaintext, extract_item_stats, parse_ddragon_items, sanitize_ddragon_html,
};
use super::{parse_ddragon_runes, parse_ddragon_summoners};
use serde_json::json;

#[test]
fn test_sanitize_ddragon_html_strips_tags_preserves_breaks_and_decodes_entities() {
    let raw = "Gain bonuses:<br>Level 5: +5 <lol-uikit-tooltipped-keyword key='LinkTooltip_Description_CDR'>Ability Haste</lol-uikit-tooltipped-keyword> <br><br>Level 11: &amp; <b>20%</b> refund&nbsp;.";
    let cleaned = sanitize_ddragon_html(raw);
    assert_eq!(
        cleaned,
        "Gain bonuses:\nLevel 5: +5 Ability Haste\nLevel 11: & 20% refund."
    );
    assert!(!cleaned.contains('<'));
    assert!(!cleaned.contains('>'));
}

#[test]
fn test_parse_ddragon_items_extracts_gold_stats_plaintext_and_fallbacks() {
    let items_json = json!({
        "data": {
            "3089": {
                "name": "Capuz da Morte de Rabadon",
                "plaintext": "Aumenta drasticamente o Poder de Habilidade",
                "gold": { "total": 3500 },
                "description": "<mainText><stats><attention>130</attention> de Poder de Habilidade</stats><br><br><passive>Apogeu Mágico</passive><br>Aumenta o <scaleAP>Poder de Habilidade total em 30%</scaleAP>.</mainText>"
            },
            "2503": {
                "name": "Tocha de Chamas Negras",
                "plaintext": "",
                "gold": { "total": 2800 },
                "description": "<mainText><stats><attention>80</attention> de Poder de Habilidade<br><attention>600</attention> de Mana<br><attention>20</attention> de Aceleração de Habilidade</stats><br><br><passive>Labareda Nefasta</passive><br>Habilidades de dano causam <magicDamage>Dano Mágico</magicDamage> adicional por 3s.<br><br><rules>Regra auxiliar ocultada.</rules></mainText>"
            },
            "3865": {
                "name": "Atlas Mundial",
                "plaintext": "",
                "gold": { "total": 400 },
                "description": ""
            },
            "2051": {
                "name": "Guardian's Horn",
                "plaintext": "",
                "gold": { "total": 950 },
                "description": "<mainText><stats><attention>150</attention> Health</stats><br><br><li><passive>Recovery:</passive> Restores <healing>20 Health</healing> every 5 seconds.<flavorText>Lore text.</flavorText></mainText>"
            }
        }
    });

    let items = parse_ddragon_items(&items_json);
    assert_eq!(items.len(), 4);

    let rabadon = items.get(&3089).expect("Rabadon parsed");
    assert_eq!(rabadon.name, "Capuz da Morte de Rabadon");
    assert_eq!(rabadon.gold_total, 3500);
    assert_eq!(rabadon.stats, vec!["130 de Poder de Habilidade"]);
    assert_eq!(
        rabadon.plaintext,
        "Aumenta drasticamente o Poder de Habilidade"
    );

    let blackfire = items.get(&2503).expect("Blackfire parsed");
    assert_eq!(blackfire.gold_total, 2800);
    assert_eq!(
        blackfire.stats,
        vec![
            "80 de Poder de Habilidade",
            "600 de Mana",
            "20 de Aceleração de Habilidade"
        ]
    );
    assert!(
        blackfire.plaintext.starts_with(
            "Labareda Nefasta: Habilidades de dano causam Dano Mágico adicional por 3s."
        )
    );
    assert!(!blackfire.plaintext.contains("Regra auxiliar"));

    let atlas = items.get(&3865).expect("World Atlas parsed");
    assert_eq!(atlas.gold_total, 400);
    assert!(atlas.stats.is_empty());
    assert!(atlas.plaintext.is_empty());

    let horn = items.get(&2051).expect("Guardian's Horn parsed");
    assert_eq!(horn.stats, vec!["150 Health"]);
    assert_eq!(
        horn.plaintext,
        "Recovery: Restores 20 Health every 5 seconds."
    );
    assert!(!horn.plaintext.contains("Lore text"));
}

#[test]
fn test_parse_ddragon_runes_and_summoners_extract_tooltip_fields() {
    let runes_json = json!([
        {
            "id": 8100,
            "key": "Domination",
            "name": "Dominação",
            "icon": "perk-images/Styles/7200_Domination.png",
            "slots": [
                {
                    "runes": [
                        {
                            "id": 8126,
                            "key": "CheapShot",
                            "name": "Golpe Desleal",
                            "icon": "perk-images/Styles/Domination/CheapShot/CheapShot.png",
                            "shortDesc": "Causa Dano Verdadeiro adicional a Campeões inimigos com <lol-uikit-tooltipped-keyword key='LinkTooltip_Description_ImpairAct'>mobilidade ou ações debilitadas</lol-uikit-tooltipped-keyword>."
                        }
                    ]
                }
            ]
        }
    ]);
    let runes = parse_ddragon_runes(&runes_json).expect("runes parsed");
    let tree = runes.get(&8100).expect("tree parsed");
    assert!(tree.short_desc.is_empty());
    let cheap_shot = runes.get(&8126).expect("rune parsed");
    assert_eq!(
        cheap_shot.short_desc,
        "Causa Dano Verdadeiro adicional a Campeões inimigos com mobilidade ou ações debilitadas."
    );

    let spells_json = json!({
        "data": {
            "SummonerFlash": {
                "key": "4",
                "name": "Flash",
                "description": "Teleporta você por uma curta distância em direção ao seu cursor.",
                "cooldown": [300],
                "cooldownBurn": "300",
                "image": { "full": "SummonerFlash.png" },
                "modes": ["CLASSIC", "ARAM"]
            },
            "SummonerArenaFlash": {
                "key": "2202",
                "name": "Flash",
                "description": "Teleporta seu Campeão.",
                "cooldown": [0.25],
                "cooldownBurn": "0.25",
                "image": { "full": "SummonerFlash.png" },
                "modes": ["CHERRY"]
            }
        }
    });
    let spells = parse_ddragon_summoners(&spells_json, "16.22.1").expect("spells parsed");
    let flash = spells.get(&4).expect("Flash parsed");
    assert_eq!(
        flash.description,
        "Teleporta você por uma curta distância em direção ao seu cursor."
    );
    assert_eq!(flash.cooldown_secs, Some(300));
    let arena_flash = spells.get(&2202).expect("Arena flash parsed");
    assert_eq!(arena_flash.cooldown_secs, None);
}

#[test]
fn test_extract_item_helpers_handle_empty_and_truncation() {
    assert!(extract_item_stats("<mainText><stats></stats></mainText>").is_empty());
    let long_desc = format!(
        "<mainText><stats></stats><passive>Long</passive><br>{}</mainText>",
        "word ".repeat(80)
    );
    let summary = extract_item_plaintext("", &long_desc);
    assert!(summary.ends_with('…'));
    assert!(summary.chars().count() <= 225);
}
