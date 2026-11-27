mod archetype;
mod catalog;

pub use archetype::{ChampionArchetype, infer_champion_archetype, infer_damage_type_from_profile};

use super::items::resolve_classifications_for_ddragon;
use super::models::{ChampionBuild, DataDragonCache, ItemBlock, ItemBlockKind};
use crate::models::{
    QUEUE_ARAM_MAYHEM, QUEUE_QUICKPLAY, QUEUE_SWIFTPLAY, is_arena_queue, queue_supports_runes,
    queue_supports_spells,
};
use archetype::derive_fallback_runes;
use catalog::derive_fallback_items;

/// Maps a queue ID into an analytics REST/MCP mode slug (`"aram"`, `"arena"`, or `"ranked"`).
///
/// OP.GG REST & MCP do not currently expose a `swiftplay`/`quickplay` mode (HTTP 422), so
/// Swiftplay (`480`) and Quickplay (`490`) use `"ranked"` analytics upstream while keeping their
/// own isolated `queue_id = 480` cache partition and resolver policy.
pub fn analytics_rest_mode_for_queue(queue_id: i32) -> &'static str {
    match queue_id {
        450 | QUEUE_ARAM_MAYHEM => "aram",
        q if is_arena_queue(q) => "arena",
        QUEUE_SWIFTPLAY | QUEUE_QUICKPLAY => "ranked",
        _ => "ranked",
    }
}

/// Maps a queue ID into an OP.GG queue argument string ("aram", "arena", or "ranked").
pub fn opgg_queue_param_from_id(queue_id: i32) -> &'static str {
    analytics_rest_mode_for_queue(queue_id)
}

/// Returns sensible default summoner spells adapted to the queue type.
/// ARAM & ARAM Mayhem: Flash (4) + Snowball/Mark (32); Summoner's Rift: Flash (4) + Ignite (14).
pub fn default_spells_for_queue(queue_id: i32) -> (i32, i32) {
    if queue_id == 450 || queue_id == QUEUE_ARAM_MAYHEM {
        (4, 32)
    } else if is_arena_queue(queue_id) {
        (0, 0)
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

fn empty_block(kind: ItemBlockKind, title: &str, item_ids: Vec<i32>) -> ItemBlock {
    ItemBlock {
        kind,
        title: title.to_string(),
        item_ids,
        item_icon_urls: vec![],
    }
}

/// Constructs a dynamic fallback build derived exclusively from `ddragon` catalogs for the
/// target champion and map/queue (zero hardcoded champion, rune, or item build data).
pub fn build_fallback_champion_build_with_ddragon(
    champion_id: i32,
    champion_name: &str,
    queue_id: i32,
    ddragon: Option<&DataDragonCache>,
) -> ChampionBuild {
    if is_arena_queue(queue_id) {
        return ChampionBuild {
            champion_id,
            champion_name: champion_name.to_string(),
            win_rate: 50.0,
            pick_rate: 0.0,
            item_blocks: vec![
                empty_block(ItemBlockKind::Prismatic, "Itens Prismáticos", vec![]),
                empty_block(ItemBlockKind::Core, "Primeiros 3 Itens", vec![]),
                empty_block(ItemBlockKind::Boots, "3 Botas Mais Feitas", vec![]),
                empty_block(ItemBlockKind::Final, "Build Final", vec![]),
                empty_block(ItemBlockKind::Situational, "Itens Situacionais / Úteis", vec![]),
            ],
            supports_runes: false,
            supports_spells: false,
            ..Default::default()
        };
    }

    let (spell1_id, spell2_id) = default_spells_for_queue(queue_id);
    let profile = ddragon.and_then(|dd| dd.champion_profiles.get(&champion_id));
    let archetype = profile
        .map(infer_champion_archetype)
        .unwrap_or(ChampionArchetype::Fighter);
    let uses_mana = profile
        .map(|p| p.partype.is_empty() || p.partype.eq_ignore_ascii_case("Mana"))
        .unwrap_or(true);

    let supports_runes = queue_supports_runes(queue_id);
    let (primary_style_id, sub_style_id, selected_perk_ids) = if supports_runes {
        derive_fallback_runes(ddragon, archetype)
    } else {
        (0, 0, Vec::new())
    };

    let items = if let Some(dd) = ddragon {
        let cls = resolve_classifications_for_ddragon(Some(dd));
        derive_fallback_items(&cls, queue_id, archetype, uses_mana)
    } else {
        catalog::FallbackItemBlocks::default()
    };

    ChampionBuild {
        champion_id,
        champion_name: champion_name.to_string(),
        win_rate: 50.0,
        pick_rate: 5.0,
        primary_style_id,
        sub_style_id,
        selected_perk_ids,
        spell1_id,
        spell2_id,
        item_blocks: vec![
            empty_block(ItemBlockKind::Starter, "Itens Iniciais", items.starter),
            empty_block(ItemBlockKind::Core, "Primeiros 3 Itens", items.core),
            empty_block(ItemBlockKind::Boots, "3 Botas Mais Feitas", items.boots),
            empty_block(ItemBlockKind::Final, "Build Final", items.final_build),
            empty_block(ItemBlockKind::Situational, "Itens Situacionais / Úteis", items.situational),
        ],
        supports_runes,
        supports_spells: queue_supports_spells(queue_id),
        ..Default::default()
    }
}

/// Convenience wrapper without an explicit `DataDragonCache` reference.
pub fn build_fallback_champion_build(
    champion_id: i32,
    champion_name: &str,
    queue_id: i32,
) -> ChampionBuild {
    build_fallback_champion_build_with_ddragon(champion_id, champion_name, queue_id, None)
}

#[cfg(test)]
#[path = "fallback_tests.rs"]
mod tests;
