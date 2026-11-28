use super::super::models::{ChampionBuild, SpellMetadata};
use super::super::spells::extract_summoner_spell_pairs_for_mode;
use super::mcp_parser::parse_opgg_mcp_text_for_mode_with_ddragon;
use super::rest_extractors::{extract_rest_runes_and_pages, extract_rest_tier, extract_rest_win_and_pick_rates};
pub use super::rest_parser_items::{McpDepthItems, RestAnalysisParseOptions};
use super::rest_parser_items::assemble_analysis_items;
use std::collections::{HashMap, HashSet};

pub fn parse_opgg_analysis_payload(
    opgg_json: &serde_json::Value,
    champion_id: i32,
    champion_name: &str,
) -> Result<ChampionBuild, String> {
    parse_opgg_analysis_payload_with_options(opgg_json, champion_id, champion_name, true)
}

pub fn parse_opgg_analysis_payload_with_options(
    opgg_json: &serde_json::Value,
    champion_id: i32,
    champion_name: &str,
    supports_runes: bool,
) -> Result<ChampionBuild, String> {
    parse_opgg_analysis_payload_for_mode(
        opgg_json,
        champion_id,
        champion_name,
        supports_runes,
        None,
        None,
    )
}

/// Parses an OP.GG REST or wrapped MCP payload into a `ChampionBuild`, filtering summoner spells
/// against DataDragon's `modes` for `spell_mode` when provided.
pub fn parse_opgg_analysis_payload_for_mode(
    opgg_json: &serde_json::Value,
    champion_id: i32,
    champion_name: &str,
    supports_runes: bool,
    spell_mode: Option<&str>,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
) -> Result<ChampionBuild, String> {
    parse_opgg_analysis_payload_for_mode_and_roster(
        opgg_json,
        champion_id,
        champion_name,
        supports_runes,
        spell_mode,
        spells_catalog,
        None,
    )
}

/// Parses an OP.GG REST or wrapped MCP payload into a `ChampionBuild`, optionally filtering
/// lane matchups to opponents belonging to `allowed_lane_champions`.
pub fn parse_opgg_analysis_payload_for_mode_and_roster(
    opgg_json: &serde_json::Value,
    champion_id: i32,
    champion_name: &str,
    supports_runes: bool,
    spell_mode: Option<&str>,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
    allowed_lane_champions: Option<&HashSet<i32>>,
) -> Result<ChampionBuild, String> {
    parse_opgg_analysis_with_options(
        opgg_json,
        RestAnalysisParseOptions {
            champion_id,
            champion_name,
            supports_runes,
            spell_mode,
            ddragon: None,
            spells_catalog,
            allowed_lane_champions,
            position_override: None,
            mcp_depth_override: None,
        },
    )
}

/// Parses an OP.GG REST or wrapped MCP payload using typed `RestAnalysisParseOptions` without
/// mutating or cloning `serde_json::Value`.
pub fn parse_opgg_analysis_with_options(
    opgg_json: &serde_json::Value,
    opts: RestAnalysisParseOptions<'_>,
) -> Result<ChampionBuild, String> {
    let json_supports_runes = opgg_json
        .get("supports_runes")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let effective_spells_catalog = opts
        .spells_catalog
        .or_else(|| opts.ddragon.map(|d| &d.spells));

    if let Some(text) = opgg_json
        .pointer("/result/content/0/text")
        .and_then(|v| v.as_str())
    {
        return parse_opgg_mcp_text_for_mode_with_ddragon(
            text,
            opts.champion_id,
            opts.champion_name,
            opts.supports_runes && json_supports_runes,
            opts.spell_mode,
            opts.ddragon,
            effective_spells_catalog,
        );
    }

    let analysis = opgg_json
        .get("data")
        .or_else(|| opgg_json.pointer("/result/data"))
        .unwrap_or(opgg_json);

    let effective_supports_runes = opts.supports_runes
        && json_supports_runes
        && analysis
            .get("supports_runes")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

    let (win_rate, pick_rate) = extract_rest_win_and_pick_rates(analysis);
    let (primary_style_id, sub_style_id, selected_perk_ids, top_rune_pages) =
        extract_rest_runes_and_pages(analysis, effective_supports_runes)?;

    let position = opts
        .position_override
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            analysis
                .get("position")
                .or_else(|| analysis.pointer("/meta/position"))
                .or_else(|| analysis.pointer("/summary/position"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or("");
    let tier = extract_rest_tier(analysis, position);
    let is_jungle = position.eq_ignore_ascii_case("jungle");

    let ((spell1_id, spell2_id), spell_options) = extract_summoner_spell_pairs_for_mode(
        analysis,
        is_jungle,
        opts.spell_mode,
        effective_spells_catalog,
    );

    let (item_blocks, build_options) = assemble_analysis_items(
        analysis,
        opts.champion_id,
        position,
        opts.ddragon,
        opts.mcp_depth_override,
    )?;

    let skill_order = super::super::skills::extract_rest_skill_order(analysis);
    let is_non_sr_mode = opts.spell_mode.is_some_and(|m| {
        m.eq_ignore_ascii_case("ARAM")
            || m.eq_ignore_ascii_case("KIWI")
            || m.eq_ignore_ascii_case("CHERRY")
    });
    let lane_matchups = if is_non_sr_mode {
        None
    } else {
        super::matchups::extract_rest_lane_matchups_for_roster(
            analysis,
            opts.champion_id,
            position,
            opts.allowed_lane_champions,
        )
    };

    Ok(ChampionBuild {
        champion_id: opts.champion_id,
        champion_name: opts.champion_name.to_string(),
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
        supports_runes: effective_supports_runes,
        skill_order,
        lane_matchups,
        ..Default::default()
    })
}

#[cfg(test)]
#[path = "rest_parser_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "rest_parser_aram_tests.rs"]
mod aram_tests;
