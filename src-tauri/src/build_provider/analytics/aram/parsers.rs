use super::html_extract::{
    clean_augment_description_with_context, decode_next_f_stream, extract_balanced_json_array,
    is_internal_augment_key, normalize_opgg_champion_key, round_pct,
};
use crate::build_provider::models::DataDragonCache;
use crate::models::{AppLocale, AramAugmentEntry, AramChampionRank};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RestAramChampionResponse {
    data: Vec<RestAramChampionItem>,
}

#[derive(Debug, Deserialize)]
struct RestAramChampionItem {
    id: i32,
    #[serde(default)]
    average_stats: Option<RestAramAverageStats>,
}

#[derive(Debug, Deserialize)]
struct RestAramAverageStats {
    #[serde(default)]
    win_rate: Option<f64>,
    #[serde(default)]
    pick_rate: Option<f64>,
    #[serde(default)]
    tier: Option<i32>,
    #[serde(default)]
    rank: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct WebMayhemChampionItem {
    id: i32,
    #[serde(default)]
    key: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    tier: i32,
    #[serde(default)]
    rank: i32,
}

#[derive(Debug, Deserialize)]
struct WebMayhemAugmentItem {
    id: i32,
    #[serde(default)]
    tier: i32,
    #[serde(default)]
    performance: f64,
    #[serde(default)]
    popular: f64,
    #[serde(default)]
    desc: Option<String>,
}

pub fn parse_classic_aram_champions(
    json: &serde_json::Value,
    ddragon: &DataDragonCache,
) -> Result<Vec<AramChampionRank>, String> {
    let parsed: RestAramChampionResponse = serde_json::from_value(json.clone())
        .map_err(|e| format!("Failed to parse classic ARAM response: {}", e))?;

    let mut champions: Vec<AramChampionRank> = parsed
        .data
        .into_iter()
        .filter_map(|item| {
            let stats = item.average_stats?;
            let meta = ddragon.champions.get(&item.id);
            let name = meta
                .map(|m| m.display_name.clone())
                .unwrap_or_else(|| format!("Champion {}", item.id));
            let key = meta
                .map(|m| normalize_opgg_champion_key(&m.key_name))
                .unwrap_or_else(|| item.id.to_string());
            let icon_url = meta.map(|m| m.icon_url.clone()).unwrap_or_default();

            Some(AramChampionRank {
                id: item.id,
                key,
                name,
                icon_url,
                tier: stats.tier.unwrap_or(5),
                rank: stats.rank.unwrap_or(999),
                win_rate: stats.win_rate.map(round_pct),
                pick_rate: stats.pick_rate.map(round_pct),
            })
        })
        .collect();

    champions.sort_by(|a, b| {
        let a_valid = if a.rank > 0 { 0 } else { 1 };
        let b_valid = if b.rank > 0 { 0 } else { 1 };
        a_valid
            .cmp(&b_valid)
            .then_with(|| a.rank.cmp(&b.rank))
            .then_with(|| a.tier.cmp(&b.tier))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(champions)
}

pub fn parse_aram_mayhem_champions_html(
    html: &str,
    ddragon: &DataDragonCache,
) -> Result<Vec<AramChampionRank>, String> {
    let rsc = decode_next_f_stream(html);
    let source = if rsc.is_empty() { html } else { &rsc };

    let marker = "\"champions\":[{\"key\":";
    let marker_idx = source
        .find(marker)
        .ok_or_else(|| "ARAM Mayhem champions payload not found in HTML".to_string())?;
    let array_start = marker_idx + "\"champions\":".len();
    let array_json = extract_balanced_json_array(source, array_start)
        .ok_or_else(|| "Malformed ARAM Mayhem champions array".to_string())?;

    let raw_items: Vec<WebMayhemChampionItem> = serde_json::from_str(array_json)
        .map_err(|e| format!("Failed to deserialize ARAM Mayhem champions: {}", e))?;

    let mut champions: Vec<AramChampionRank> = raw_items
        .into_iter()
        .map(|item| {
            let dd_meta = ddragon.champions.get(&item.id);
            let icon_url = dd_meta.map(|m| m.icon_url.clone()).unwrap_or_default();
            let name = dd_meta
                .map(|m| m.display_name.clone())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| {
                    if !item.name.is_empty() {
                        item.name
                    } else {
                        format!("Champion {}", item.id)
                    }
                });
            let key = dd_meta
                .map(|m| normalize_opgg_champion_key(&m.key_name))
                .filter(|k| !k.is_empty())
                .unwrap_or_else(|| normalize_opgg_champion_key(&item.key));

            AramChampionRank {
                id: item.id,
                key,
                name,
                icon_url,
                // OP.GG Mayhem tier starts at 1 (1 = OP, 2 = Tier 1, 3 = Tier 2, ...).
                // Normalize to 0-based canonical tiers (0 = OP, 1 = Tier 1, ..., 5 = Tier 5).
                tier: item.tier.saturating_sub(1),
                rank: item.rank,
                win_rate: None,
                pick_rate: None,
            }
        })
        .collect();

    champions.sort_by(|a, b| {
        let a_valid = if a.rank > 0 { 0 } else { 1 };
        let b_valid = if b.rank > 0 { 0 } else { 1 };
        a_valid
            .cmp(&b_valid)
            .then_with(|| a.rank.cmp(&b.rank))
            .then_with(|| a.tier.cmp(&b.tier))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(champions)
}

pub fn parse_aram_mayhem_augments_html_for_locale(
    html: &str,
    ddragon: &DataDragonCache,
    locale: AppLocale,
) -> Result<Vec<AramAugmentEntry>, String> {
    let rsc = decode_next_f_stream(html);
    let source = if rsc.is_empty() { html } else { &rsc };

    let mut search_from = 0usize;
    let mut matched_array: Option<&str> = None;

    while let Some(rel_idx) = source[search_from..].find("\"data\":[{\"id\":") {
        let abs_idx = search_from + rel_idx;
        let array_start = abs_idx + "\"data\":".len();
        if let Some(candidate) = extract_balanced_json_array(source, array_start)
            && candidate.contains("\"performance\":")
            && candidate.contains("\"popular\":")
        {
            matched_array = Some(candidate);
            break;
        }
        search_from = abs_idx + 8;
    }

    let array_json = matched_array
        .ok_or_else(|| "ARAM Mayhem augments payload not found in HTML".to_string())?;

    let raw_items: Vec<WebMayhemAugmentItem> = serde_json::from_str(array_json)
        .map_err(|e| format!("Failed to deserialize ARAM Mayhem augments: {}", e))?;

    let mut augments: Vec<AramAugmentEntry> = raw_items
        .into_iter()
        .filter_map(|item| {
            let meta = ddragon.arena_augments.get(&item.id)?;
            let name = meta.name.trim();
            if name.is_empty() || is_internal_augment_key(name) {
                return None;
            }
            let desc = if !meta.desc.trim().is_empty() {
                meta.desc.clone()
            } else {
                clean_augment_description_with_context(
                    item.desc.as_deref().unwrap_or(""),
                    locale,
                    meta.ability_slot.as_deref(),
                )
            };

            Some(AramAugmentEntry {
                id: item.id,
                name: name.to_string(),
                tier: item.tier,
                rarity: (meta.rarity > 0).then_some(meta.rarity),
                performance: (item.performance * 100.0).round() / 100.0,
                popular: (item.popular * 100.0).round() / 100.0,
                icon_url: meta.icon_url.clone(),
                desc,
            })
        })
        .collect();

    augments.sort_by(|a, b| {
        a.tier
            .cmp(&b.tier)
            .then_with(|| b.popular.total_cmp(&a.popular))
            .then_with(|| b.performance.total_cmp(&a.performance))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(augments)
}

pub fn parse_aram_mayhem_augments_html(
    html: &str,
    ddragon: &DataDragonCache,
) -> Result<Vec<AramAugmentEntry>, String> {
    parse_aram_mayhem_augments_html_for_locale(html, ddragon, AppLocale::EnUs)
}
