use super::models::{ChampionBuild, ItemBlock};
use crate::ws_listener::QUEUE_ARAM_MAYHEM;

/// Maps a queue ID into an OP.GG queue argument string ("aram" or "ranked").
pub fn opgg_queue_param_from_id(queue_id: i32) -> &'static str {
    match queue_id {
        450 | QUEUE_ARAM_MAYHEM => "aram",
        _ => "ranked",
    }
}

/// Returns sensible default summoner spells adapted to the queue type.
/// ARAM & ARAM Mayhem: Flash (4) + Snowball/Mark (32); Summoner's Rift: Flash (4) + Ignite (14).
pub fn default_spells_for_queue(queue_id: i32) -> (i32, i32) {
    if queue_id == 450 || queue_id == QUEUE_ARAM_MAYHEM {
        (4, 32)
    } else {
        (4, 14)
    }
}

/// Fallback default lane when offline or when OP.GG ranking is unavailable.
pub fn champion_default_lane(_champion_name: &str) -> &'static str {
    "mid"
}

/// Normalizes DataDragon champion key into OP.GG's required champion name enum format.
pub fn champion_key_to_opgg_name(key: &str) -> String {
    match key {
        "MonkeyKing" => "MONKEY_KING".to_string(),
        _ => key.to_uppercase(),
    }
}

/// Constructs a safe, universal fallback build in case OP.GG API is down or times out.
pub fn build_fallback_champion_build(
    champion_id: i32,
    champion_name: &str,
    queue_id: i32,
) -> ChampionBuild {
    let (spell1_id, spell2_id) = default_spells_for_queue(queue_id);

    // Universal standard rune page: Precision (8000) + Domination (8100)
    let selected_perk_ids = vec![
        8005, // Press the Attack
        9111, // Triumph
        9104, // Legend: Alacrity
        8014, // Coup de Grace
        8139, // Taste of Blood
        8135, // Treasure Hunter
        5008, // Adaptive Force
        5008, // Adaptive Force
        5001, // Health Scaling
    ];

    let starter_items = if queue_id == 450 || queue_id == QUEUE_ARAM_MAYHEM {
        vec![2051, 2003] // Guardian's Horn/Blade + Potion
    } else {
        vec![1055, 2003] // Doran's Blade + Health Potion
    };

    // Core 3 items must strictly exclude any boots (e.g. Infinity Edge, Rapid Firecannon, Lord Dominik's)
    let core_items = vec![3031, 3094, 3036];
    let boots_items = vec![3006, 3047, 3111];
    // Final build: core + late-game items (e.g. Bloodthirster, Guardian Angel)
    let final_items = vec![3031, 3094, 3036, 3072, 3026];
    // Useful / situational items: high pickrate items that are NOT in core nor in final build
    let situational_items = vec![3046, 3091, 3139]; // Phantom Dancer, Wit's End, Mercurial

    ChampionBuild {
        champion_id,
        champion_name: champion_name.to_string(),
        win_rate: 50.0,
        pick_rate: 5.0,
        primary_style_id: 8000,
        sub_style_id: 8100,
        selected_perk_ids,
        spell1_id,
        spell2_id,
        item_blocks: vec![
            ItemBlock {
                title: "Itens Iniciais".to_string(),
                item_ids: starter_items,
                item_icon_urls: vec![],
            },
            ItemBlock {
                title: "Primeiros 3 Itens".to_string(),
                item_ids: core_items,
                item_icon_urls: vec![],
            },
            ItemBlock {
                title: "3 Botas Mais Feitas".to_string(),
                item_ids: boots_items,
                item_icon_urls: vec![],
            },
            ItemBlock {
                title: "Build Final".to_string(),
                item_ids: final_items,
                item_icon_urls: vec![],
            },
            ItemBlock {
                title: "Itens Situacionais / Úteis".to_string(),
                item_ids: situational_items,
                item_icon_urls: vec![],
            },
        ],
        champion_icon_url: String::new(),
        primary_style_name: String::new(),
        primary_style_icon_url: String::new(),
        sub_style_name: String::new(),
        sub_style_icon_url: String::new(),
        perks: vec![],
        spells: vec![],
        supports_runes: queue_id != QUEUE_ARAM_MAYHEM,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opgg_queue_param_and_default_spells_adaptation() {
        assert_eq!(opgg_queue_param_from_id(450), "aram");
        assert_eq!(default_spells_for_queue(450), (4, 32)); // Flash + Snowball

        assert_eq!(opgg_queue_param_from_id(QUEUE_ARAM_MAYHEM), "aram");
        assert_eq!(default_spells_for_queue(QUEUE_ARAM_MAYHEM), (4, 32));

        assert_eq!(opgg_queue_param_from_id(420), "ranked");
        assert_eq!(default_spells_for_queue(420), (4, 14)); // Flash + Ignite

        assert_eq!(opgg_queue_param_from_id(440), "ranked");
        assert_eq!(opgg_queue_param_from_id(400), "ranked");
        assert_eq!(opgg_queue_param_from_id(-1), "ranked");
        assert_eq!(default_spells_for_queue(-1), (4, 14));
    }

    #[test]
    fn test_fallback_build_has_valid_9_perks_and_queue_spells() {
        let aram_fallback = build_fallback_champion_build(103, "Ahri", 450);
        assert_eq!(aram_fallback.selected_perk_ids.len(), 9);
        assert_eq!(aram_fallback.spell1_id, 4);
        assert_eq!(aram_fallback.spell2_id, 32);
        assert!(aram_fallback.supports_runes);
        assert_eq!(aram_fallback.item_blocks.len(), 5);
        assert_eq!(aram_fallback.item_blocks[0].title, "Itens Iniciais");
        assert_eq!(aram_fallback.item_blocks[1].title, "Primeiros 3 Itens");
        assert_eq!(aram_fallback.item_blocks[2].title, "3 Botas Mais Feitas");
        assert_eq!(aram_fallback.item_blocks[3].title, "Build Final");
        assert_eq!(
            aram_fallback.item_blocks[4].title,
            "Itens Situacionais / Úteis"
        );

        let mayhem_fallback = build_fallback_champion_build(103, "Ahri", QUEUE_ARAM_MAYHEM);
        assert_eq!(mayhem_fallback.spell1_id, 4);
        assert_eq!(mayhem_fallback.spell2_id, 32);
        assert_eq!(mayhem_fallback.item_blocks[0].item_ids, vec![2051, 2003]);
        assert!(!mayhem_fallback.supports_runes);

        let ranked_fallback = build_fallback_champion_build(103, "Ahri", 420);
        assert_eq!(ranked_fallback.selected_perk_ids.len(), 9);
        assert_eq!(ranked_fallback.spell1_id, 4);
        assert_eq!(ranked_fallback.spell2_id, 14);
        assert!(ranked_fallback.supports_runes);

        let practice_fallback = build_fallback_champion_build(103, "Ahri", -1);
        assert_eq!(practice_fallback.selected_perk_ids.len(), 9);
        assert_eq!(practice_fallback.spell1_id, 4);
        assert_eq!(practice_fallback.spell2_id, 14);
        assert_eq!(practice_fallback.item_blocks[0].item_ids, vec![1055, 2003]);
        assert!(practice_fallback.supports_runes);
    }
}
