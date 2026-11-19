use super::super::items::*;
use super::super::models::{ChampionBuild, ItemStatEntry, SpellMetadata};
use super::super::runes::extract_all_mcp_runes;
use super::super::spells::extract_summoner_spell_choices_for_mode;
use super::mcp_extractors::{extract_all_core_items_from_text, extract_mcp_grouped_item_lists};
use super::mcp_stats::{
    extract_mcp_summoner_spell_entries, is_known_summoner_spell_id, parse_mcp_average_stats,
};
use std::collections::HashMap;

/// Parses OP.GG MCP text output into a ChampionBuild (requiring valid runes).
pub fn parse_opgg_mcp_text(
    text: &str,
    champion_id: i32,
    champion_name: &str,
) -> Result<ChampionBuild, String> {
    parse_opgg_mcp_text_with_options(text, champion_id, champion_name, true)
}

/// Parses OP.GG MCP text output into a ChampionBuild with configurable rune requirement.
pub fn parse_opgg_mcp_text_with_options(
    text: &str,
    champion_id: i32,
    champion_name: &str,
    supports_runes: bool,
) -> Result<ChampionBuild, String> {
    parse_opgg_mcp_text_for_mode(text, champion_id, champion_name, supports_runes, None, None)
}

/// Parses OP.GG MCP text output into a ChampionBuild with optional mode-based spell filtering.
pub fn parse_opgg_mcp_text_for_mode(
    text: &str,
    champion_id: i32,
    champion_name: &str,
    supports_runes: bool,
    spell_mode: Option<&str>,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
) -> Result<ChampionBuild, String> {
    // 1. Extract win_rate and pick_rate: AverageStats(...)
    let (win_rate, pick_rate) = parse_mcp_average_stats(text);

    // 2. Extract Runes (supports both legacy `Runes(8100,...)` and OP.GG MCP `Runes(id,8100,"Domination",...)`)
    let top_rune_pages = extract_all_mcp_runes(text);
    let (primary_style_id, sub_style_id, selected_perk_ids) =
        if let Some(first) = top_rune_pages.first() {
            (
                first.primary_style_id,
                first.sub_style_id,
                first.selected_perk_ids.clone(),
            )
        } else if supports_runes {
            if !text.contains("Runes(") {
                return Err("Missing Runes block in OP.GG text".to_string());
            }
            return Err("Invalid Runes format in OP.GG text".to_string());
        } else {
            (0, 0, Vec::new())
        };

    // 3. Extract all CoreItems entries with stats
    let all_entries = extract_all_core_items_from_text(text);

    let position = if text.contains("\"position\": \"jungle\"") || text.contains("\"jungle\"") {
        "jungle"
    } else if text.contains("\"position\": \"support\"") || text.contains("\"support\"") {
        "support"
    } else {
        ""
    };
    let is_jungle = position.eq_ignore_ascii_case("jungle");

    // 4. Extract summoner spells via shared extract_summoner_spell_choices_for_mode
    let spells_json = extract_mcp_summoner_spell_entries(text, &all_entries);
    let ((spell1_id, spell2_id), spell_choice_ids) = extract_summoner_spell_choices_for_mode(
        &spells_json,
        is_jungle,
        spell_mode,
        spells_catalog,
    );

    // Filter out spell arrays from items entries
    let item_candidate_entries: Vec<ItemStatEntry> = all_entries
        .into_iter()
        .filter(|entry| {
            !(entry.ids.len() == 2
                && is_known_summoner_spell_id(entry.ids[0])
                && is_known_summoner_spell_id(entry.ids[1]))
        })
        .collect();

    let is_ap = text.contains("\"AP\"")
        || (!text.contains("\"AD\"")
            && (champion_name.to_lowercase().contains("ahri")
                || champion_name.to_lowercase().contains("lux")
                || champion_name.to_lowercase().contains("syndra")
                || champion_name.to_lowercase().contains("veigar")));
    let damage_type = if is_ap { "AP" } else { "AD" };

    let starter_entries = if let Some(e) = item_candidate_entries.get(2) {
        vec![e.clone()]
    } else {
        Vec::new()
    };

    let grouped_lists = extract_mcp_grouped_item_lists(text);
    let mut fourth_entries = Vec::new();
    let mut fifth_entries = Vec::new();
    let mut sixth_entries = Vec::new();
    let mut last_entries = Vec::new();
    let mut boots_from_mcp = Vec::new();
    for (name, items) in grouped_lists {
        match name.as_str() {
            "boots" => boots_from_mcp = items,
            "fourth_items" => fourth_entries = items,
            "fifth_items" => fifth_entries = items,
            "sixth_items" => sixth_entries = items,
            "last_items" => last_entries = items,
            _ => last_entries.extend(items),
        }
    }

    let boot_entries = if !boots_from_mcp.is_empty() {
        boots_from_mcp
    } else {
        item_candidate_entries.get(1).cloned().into_iter().collect()
    };

    let mut candidate_entries = Vec::new();
    let core_ids = if let Some(first) = item_candidate_entries.first() {
        let mut core_entry = first.clone();
        if core_entry.play == 1 {
            core_entry.play = 10000;
        }
        let ids = core_entry.ids.clone();
        candidate_entries.push(core_entry);
        ids
    } else {
        Vec::new()
    };

    if !last_entries.is_empty() {
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
    } else if item_candidate_entries.len() > 3 {
        for entry in &item_candidate_entries[3..] {
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
    }

    let all_for_tear: Vec<ItemStatEntry> = candidate_entries
        .iter()
        .chain(fourth_entries.iter())
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .cloned()
        .collect();
    let tear_upgrade = resolve_tear_to_completed_item(&all_for_tear);

    let normalize_entries = |entries: &mut Vec<ItemStatEntry>| {
        for entry in entries.iter_mut() {
            entry.ids = resolve_tear_item_ids(&entry.ids, tear_upgrade);
        }
        entries.retain(|e| !e.ids.is_empty());
    };
    normalize_entries(&mut candidate_entries);
    normalize_entries(&mut fourth_entries);
    normalize_entries(&mut fifth_entries);
    normalize_entries(&mut sixth_entries);
    normalize_entries(&mut last_entries);

    let item_blocks = build_5_item_blocks_with_depth_lists(
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

    let build_options = build_unified_build_options(
        &candidate_entries,
        &fourth_entries,
        &fifth_entries,
        &sixth_entries,
        &candidate_entries,
        &[],
        &general_situational,
        damage_type,
        position,
    );

    let spell_choices = spell_choice_ids
        .into_iter()
        .map(|id| SpellMetadata {
            id,
            ..Default::default()
        })
        .collect();

    Ok(ChampionBuild {
        champion_id,
        champion_name: champion_name.to_string(),
        win_rate,
        pick_rate,
        primary_style_id,
        sub_style_id,
        selected_perk_ids,
        spell1_id,
        spell2_id,
        item_blocks,
        spell_choices,
        rune_pages: top_rune_pages,
        build_options,
        supports_runes,
        ..Default::default()
    })
}

#[cfg(test)]
#[path = "mcp_parser_tests.rs"]
mod tests;
