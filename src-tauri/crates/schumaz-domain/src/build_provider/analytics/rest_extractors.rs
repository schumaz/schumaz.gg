use super::super::items::{get_item_tags_dynamic, is_offensive_ad_tag, is_offensive_ap_tag};
use super::super::models::{ItemStatEntry, RunePageOption};
use super::super::runes::extract_top_rune_pages;
use super::positions::normalize_opgg_lane_name;

/// Normalizes OP.GG's 1-based meta tier (`1` = OP, `2` = Tier 1, `3` = Tier 2, `4` = Tier 3,
/// `5` = Tier 4) into our canonical 0-based scale (`0..=4`). Returns `None` for non-positive values.
pub(crate) fn normalize_opgg_tier(raw: i32) -> Option<i32> {
    if raw <= 0 {
        return None;
    }
    Some((raw - 1).clamp(0, 4))
}

/// Extracts and normalizes the champion's canonical tier (`0 = OP`, `1..=4 = Tier 1..4`) for the
/// active lane/mode from an OP.GG REST analysis payload.
pub fn extract_rest_tier(analysis: &serde_json::Value, position: &str) -> Option<i32> {
    if analysis
        .pointer("/summary/is_rip")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return None;
    }

    let normalized_pos = normalize_opgg_lane_name(position.trim());
    if let Some(target_lane) = normalized_pos
        && let Some(positions) = analysis
            .pointer("/summary/positions")
            .and_then(|v| v.as_array())
        && !positions.is_empty()
    {
        let matched = positions.iter().find(|entry| {
            entry
                .get("name")
                .and_then(|v| v.as_str())
                .and_then(normalize_opgg_lane_name)
                == Some(target_lane)
        });
        let raw = matched.and_then(|entry| {
            entry
                .pointer("/stats/tier_data/tier")
                .or_else(|| entry.pointer("/stats/tier"))
                .and_then(|v| v.as_i64())
                .map(|n| n as i32)
        })?;
        return normalize_opgg_tier(raw);
    }

    let raw = analysis
        .pointer("/summary/average_stats/tier_data/tier")
        .or_else(|| analysis.pointer("/summary/average_stats/tier"))
        .or_else(|| analysis.pointer("/average_stats/tier_data/tier"))
        .or_else(|| analysis.pointer("/average_stats/tier"))
        .or_else(|| analysis.pointer("/stats/tier_data/tier"))
        .or_else(|| analysis.pointer("/stats/tier"))
        .or_else(|| analysis.get("tier"))
        .and_then(|v| v.as_i64())
        .map(|n| n as i32)?;

    normalize_opgg_tier(raw)
}

pub fn extract_rest_win_and_pick_rates(analysis: &serde_json::Value) -> (f64, f64) {
    let raw_win_rate = analysis
        .get("win_rate")
        .or_else(|| analysis.pointer("/stats/win_rate"))
        .or_else(|| analysis.pointer("/summary/average_stats/win_rate"))
        .or_else(|| analysis.pointer("/average_stats/win_rate"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let win_rate = if raw_win_rate > 0.0 && raw_win_rate <= 1.0 {
        (raw_win_rate * 1000.0).round() / 10.0
    } else {
        raw_win_rate
    };

    let raw_pick_rate = analysis
        .get("pick_rate")
        .or_else(|| analysis.pointer("/stats/pick_rate"))
        .or_else(|| analysis.pointer("/summary/average_stats/pick_rate"))
        .or_else(|| analysis.pointer("/average_stats/pick_rate"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let pick_rate = if raw_pick_rate > 0.0 && raw_pick_rate <= 1.0 {
        (raw_pick_rate * 1000.0).round() / 10.0
    } else {
        raw_pick_rate
    };

    (win_rate, pick_rate)
}

pub fn extract_rest_runes_and_pages(
    analysis: &serde_json::Value,
    effective_supports_runes: bool,
) -> Result<(i32, i32, Vec<i32>, Vec<RunePageOption>), String> {
    let primary_style_opt = analysis
        .pointer("/runes/primary_style_id")
        .or_else(|| analysis.pointer("/runes/primary_page_id"))
        .or_else(|| analysis.pointer("/runes/0/primary_page_id"))
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    let sub_style_opt = analysis
        .pointer("/runes/sub_style_id")
        .or_else(|| analysis.pointer("/runes/secondary_page_id"))
        .or_else(|| analysis.pointer("/runes/0/secondary_page_id"))
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    let mut extracted_perk_ids = Vec::with_capacity(9);

    if let Some(primary_perks) = analysis
        .pointer("/runes/primary_perk_ids")
        .or_else(|| analysis.pointer("/runes/0/primary_rune_ids"))
        .and_then(|v| v.as_array())
    {
        for perk in primary_perks {
            if let Some(id) = perk.as_i64() {
                extracted_perk_ids.push(id as i32);
            }
        }
    }

    if let Some(sub_perks) = analysis
        .pointer("/runes/sub_perk_ids")
        .or_else(|| analysis.pointer("/runes/0/secondary_rune_ids"))
        .and_then(|v| v.as_array())
    {
        for perk in sub_perks {
            if let Some(id) = perk.as_i64() {
                extracted_perk_ids.push(id as i32);
            }
        }
    }

    if let Some(stat_shards) = analysis
        .pointer("/runes/stat_shards")
        .or_else(|| analysis.pointer("/runes/0/stat_mod_ids"))
        .and_then(|v| v.as_array())
    {
        for shard in stat_shards {
            if let Some(id) = shard.as_i64() {
                extracted_perk_ids.push(id as i32);
            }
        }
    }

    let (mut primary_style_id, mut sub_style_id, mut selected_perk_ids) =
        if effective_supports_runes {
            let p_id = primary_style_opt
                .ok_or_else(|| "Missing primary_style_id in OP.GG runes".to_string())?;
            let s_id =
                sub_style_opt.ok_or_else(|| "Missing sub_style_id in OP.GG runes".to_string())?;
            if extracted_perk_ids.len() != 9 {
                return Err(format!(
                    "Invalid perk count in OP.GG payload: expected 9 perks, found {}",
                    extracted_perk_ids.len()
                ));
            }
            (p_id, s_id, extracted_perk_ids)
        } else if let (Some(p_id), Some(s_id)) = (primary_style_opt, sub_style_opt)
            && extracted_perk_ids.len() == 9
        {
            (p_id, s_id, extracted_perk_ids)
        } else {
            (0, 0, Vec::new())
        };

    let mut top_rune_pages = extract_top_rune_pages(analysis);
    if top_rune_pages.is_empty() {
        if effective_supports_runes
            || (primary_style_id > 0 && sub_style_id > 0 && selected_perk_ids.len() == 9)
        {
            top_rune_pages.push(RunePageOption {
                primary_style_id,
                sub_style_id,
                selected_perk_ids: selected_perk_ids.clone(),
                play: 1,
                win: 1,
                pick_rate: 1.0,
                ..Default::default()
            });
        }
    } else {
        let first = &top_rune_pages[0];
        primary_style_id = first.primary_style_id;
        sub_style_id = first.sub_style_id;
        selected_perk_ids = first.selected_perk_ids.clone();
    }

    Ok((
        primary_style_id,
        sub_style_id,
        selected_perk_ids,
        top_rune_pages,
    ))
}

fn extract_primary_champion_role(analysis: &serde_json::Value) -> Option<&str> {
    let roles = analysis
        .pointer("/summary/roles")
        .and_then(|v| v.as_array())?;
    roles
        .iter()
        .max_by_key(|r| {
            r.pointer("/stats/play")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
        })
        .or_else(|| roles.first())
        .and_then(|r| r.get("name"))
        .and_then(|v| v.as_str())
}

/// Infers `"AD"` or `"AP"` dynamically from explicit payload metadata, weighted DataDragon
/// item tags aggregated across `core_items` + `last_items`, and champion role classification.
pub fn infer_rest_damage_type(
    analysis: &serde_json::Value,
    core_entries: &[ItemStatEntry],
    last_entries: &[ItemStatEntry],
) -> String {
    if let Some(dt) = analysis
        .get("damage_type")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return dt.to_string();
    }

    let mut ad_weight: i64 = 0;
    let mut ap_weight: i64 = 0;
    let mut has_spell_damage_tag = false;

    for entry in core_entries.iter().chain(last_entries.iter()) {
        let weight = entry.play.max(1);
        for &id in &entry.ids {
            let tags = get_item_tags_dynamic(id);
            if tags.is_empty() {
                continue;
            }
            if tags.iter().any(|t| t == "SpellDamage") {
                has_spell_damage_tag = true;
            }
            if tags.iter().any(|t| is_offensive_ad_tag(t)) {
                ad_weight = ad_weight.saturating_add(weight);
            }
            if tags.iter().any(|t| is_offensive_ap_tag(t)) {
                ap_weight = ap_weight.saturating_add(weight);
            }
        }
    }

    if ad_weight > ap_weight {
        return "AD".to_string();
    }
    if ap_weight > ad_weight {
        return "AP".to_string();
    }

    if !has_spell_damage_tag
        && extract_primary_champion_role(analysis).is_some_and(|role| {
            role.eq_ignore_ascii_case("FIGHTER")
                || role.eq_ignore_ascii_case("ASSASSIN")
                || role.eq_ignore_ascii_case("MARKSMAN")
        })
    {
        return "AD".to_string();
    }

    "AP".to_string()
}
