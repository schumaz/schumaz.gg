use super::super::items::{
    resolve_classifications_for_ddragon, resolve_tear_item_ids_in,
    resolve_tear_to_completed_item_in,
};
use super::super::items::{build_5_item_blocks_with_cls, build_unified_build_options_with_cls};
use super::super::models::{ChampionBuild, DataDragonCache, ItemStatEntry, SpellMetadata};
use super::super::runes::extract_all_mcp_runes_from_doc;
use super::super::skills::extract_mcp_skill_order_from_doc;
use super::super::spells::extract_summoner_spell_pairs_for_mode;
use super::mcp_dsl::McpDocument;
use super::mcp_extractors::extract_mcp_data_sections;
use super::mcp_stats::{
    extract_mcp_summoner_spell_entries_doc, is_known_summoner_spell_id,
    parse_mcp_average_stats_doc, parse_mcp_tier_doc,
};
use super::rest_extractors::infer_damage_type_with_cls;
use std::collections::HashMap;

/// Parses OP.GG MCP text output into a `ChampionBuild` (requiring valid runes).
pub fn parse_opgg_mcp_text(
    text: &str,
    champion_id: i32,
    champion_name: &str,
) -> Result<ChampionBuild, String> {
    parse_opgg_mcp_text_with_options(text, champion_id, champion_name, true)
}

/// Parses OP.GG MCP text output into a `ChampionBuild` with configurable rune requirement.
pub fn parse_opgg_mcp_text_with_options(
    text: &str,
    champion_id: i32,
    champion_name: &str,
    supports_runes: bool,
) -> Result<ChampionBuild, String> {
    parse_opgg_mcp_text_for_mode(text, champion_id, champion_name, supports_runes, None, None)
}

/// Parses OP.GG MCP text output into a `ChampionBuild` with optional mode-based spell filtering.
pub fn parse_opgg_mcp_text_for_mode(
    text: &str,
    champion_id: i32,
    champion_name: &str,
    supports_runes: bool,
    spell_mode: Option<&str>,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
) -> Result<ChampionBuild, String> {
    parse_opgg_mcp_text_for_mode_with_ddragon(
        text,
        champion_id,
        champion_name,
        supports_runes,
        spell_mode,
        None,
        spells_catalog,
    )
}

/// Parses OP.GG MCP text output into a `ChampionBuild` using a single `McpDocument` pass,
/// dynamic DataDragon champion/item catalogs, and zero hardcoded champion names or play inflation.
pub fn parse_opgg_mcp_text_for_mode_with_ddragon(
    text: &str,
    champion_id: i32,
    champion_name: &str,
    supports_runes: bool,
    spell_mode: Option<&str>,
    ddragon: Option<&DataDragonCache>,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
) -> Result<ChampionBuild, String> {
    let doc = McpDocument::parse(text);
    let cls = resolve_classifications_for_ddragon(ddragon);
    let effective_spells_catalog = spells_catalog.or_else(|| ddragon.map(|d| &d.spells));

    let (win_rate, pick_rate) = parse_mcp_average_stats_doc(&doc);
    let tier = parse_mcp_tier_doc(&doc);

    let top_rune_pages = extract_all_mcp_runes_from_doc(&doc);
    let (primary_style_id, sub_style_id, selected_perk_ids) =
        if let Some(first) = top_rune_pages.first() {
            (
                first.primary_style_id,
                first.sub_style_id,
                first.selected_perk_ids.clone(),
            )
        } else if supports_runes {
            if doc.find_tuples(&["Runes"]).is_empty() {
                return Err("Missing Runes block in OP.GG text".to_string());
            }
            return Err("Invalid Runes format in OP.GG text".to_string());
        } else {
            (0, 0, Vec::new())
        };

    let sections = extract_mcp_data_sections(&doc, &cls, effective_spells_catalog);
    let position = sections.position.as_deref().unwrap_or("");
    let is_jungle = position.eq_ignore_ascii_case("jungle");

    let spells_json = extract_mcp_summoner_spell_entries_doc(
        &doc,
        &sections.spell_fallback_entries,
        effective_spells_catalog,
    );
    let ((spell1_id, spell2_id), spell_options) = extract_summoner_spell_pairs_for_mode(
        &spells_json,
        is_jungle,
        spell_mode,
        effective_spells_catalog,
    );

    let is_spell_pair = |entry: &ItemStatEntry| -> bool {
        entry.ids.len() == 2
            && (effective_spells_catalog.is_some_and(|c| c.contains_key(&entry.ids[0]))
                || is_known_summoner_spell_id(entry.ids[0]))
            && (effective_spells_catalog.is_some_and(|c| c.contains_key(&entry.ids[1]))
                || is_known_summoner_spell_id(entry.ids[1]))
    };

    let mut core_entries: Vec<ItemStatEntry> = sections
        .core_entries
        .into_iter()
        .filter(|e| !is_spell_pair(e))
        .collect();
    let starter_entries: Vec<ItemStatEntry> = sections
        .starter_entries
        .into_iter()
        .filter(|e| !is_spell_pair(e))
        .collect();
    let boot_entries: Vec<ItemStatEntry> = sections
        .boot_entries
        .into_iter()
        .filter(|e| !is_spell_pair(e))
        .collect();
    let mut fourth_entries = sections.fourth_entries;
    let mut fifth_entries = sections.fifth_entries;
    let mut sixth_entries = sections.sixth_entries;
    let mut last_entries = sections.last_entries;

    if core_entries.is_empty()
        && last_entries.is_empty()
        && fourth_entries.is_empty()
        && fifth_entries.is_empty()
        && sixth_entries.is_empty()
    {
        return Err("Missing core_items and last_items in OP.GG MCP text".to_string());
    }

    if sections.core_has_explicit_play {
        core_entries.sort_by(|a, b| b.play.cmp(&a.play).then_with(|| b.win.cmp(&a.win)));
        if core_entries.iter().any(|e| e.play >= 2) {
            core_entries.retain(|e| e.play >= 2);
        }
    }

    let core_ids = core_entries
        .first()
        .map(|e| e.ids.clone())
        .unwrap_or_default();

    let mut candidate_entries = core_entries.clone();
    for entry in &last_entries {
        let filtered_ids: Vec<i32> = entry
            .ids
            .iter()
            .filter(|id| !core_ids.contains(id))
            .copied()
            .collect();
        if !filtered_ids.is_empty() {
            candidate_entries.push(ItemStatEntry {
                ids: filtered_ids,
                play: entry.play,
                win: entry.win,
                pick_rate: entry.pick_rate,
            });
        }
    }

    let damage_type_owned = infer_damage_type_with_cls(
        sections.damage_type.as_deref(),
        &core_entries,
        &last_entries,
        None,
        champion_id,
        ddragon,
        &cls,
    );
    let damage_type = damage_type_owned.as_str();

    let all_for_tear: Vec<ItemStatEntry> = candidate_entries
        .iter()
        .chain(fourth_entries.iter())
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .cloned()
        .collect();
    let tear_upgrade = resolve_tear_to_completed_item_in(&cls, &all_for_tear);

    let normalize_entries = |entries: &mut Vec<ItemStatEntry>| {
        for entry in entries.iter_mut() {
            entry.ids = resolve_tear_item_ids_in(&cls, &entry.ids, tear_upgrade);
        }
        entries.retain(|e| !e.ids.is_empty());
    };
    normalize_entries(&mut core_entries);
    normalize_entries(&mut candidate_entries);
    normalize_entries(&mut fourth_entries);
    normalize_entries(&mut fifth_entries);
    normalize_entries(&mut sixth_entries);
    normalize_entries(&mut last_entries);

    let item_blocks = build_5_item_blocks_with_cls(
        &cls,
        starter_entries,
        boot_entries,
        candidate_entries.clone(),
        fourth_entries.clone(),
        fifth_entries.clone(),
        sixth_entries.clone(),
        damage_type,
        position,
    );
    let general_situational = item_blocks
        .get(4)
        .map(|b| b.item_ids.clone())
        .unwrap_or_default();

    let build_options = build_unified_build_options_with_cls(
        &cls,
        &core_entries,
        &fourth_entries,
        &fifth_entries,
        &sixth_entries,
        &candidate_entries,
        &[],
        &general_situational,
        damage_type,
        position,
    );

    let skill_order = extract_mcp_skill_order_from_doc(&doc);

    Ok(ChampionBuild {
        champion_id,
        champion_name: champion_name.to_string(),
        win_rate,
        pick_rate,
        tier,
        primary_style_id,
        sub_style_id,
        selected_perk_ids,
        spell1_id,
        spell2_id,
        item_blocks,
        spell_options,
        rune_pages: top_rune_pages,
        build_options,
        supports_runes,
        skill_order,
        ..Default::default()
    })
}

#[cfg(test)]
#[path = "mcp_parser_tests.rs"]
mod tests;
