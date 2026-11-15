use super::super::items::extract_core_archetype_tags;
use super::super::models::{ItemStatEntry, RunePageOption};
use super::super::runes::extract_top_rune_pages;

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

pub fn infer_rest_damage_type(
    analysis: &serde_json::Value,
    core_entries: &[ItemStatEntry],
) -> String {
    if let Some(dt) = analysis.get("damage_type").and_then(|v| v.as_str()) {
        return dt.to_string();
    }

    let core_is_ad = core_entries
        .first()
        .map(|c| extract_core_archetype_tags(&c.ids).3)
        .unwrap_or(false);
    let core_is_ap = core_entries
        .first()
        .map(|c| extract_core_archetype_tags(&c.ids).4)
        .unwrap_or(false);

    if core_is_ad {
        "AD".to_string()
    } else if core_is_ap {
        "AP".to_string()
    } else if analysis
        .pointer("/summary/roles/0/name")
        .and_then(|v| v.as_str())
        .is_some_and(|r| r.eq_ignore_ascii_case("MARKSMAN"))
    {
        "AD".to_string()
    } else {
        "AP".to_string()
    }
}
