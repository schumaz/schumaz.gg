use std::collections::HashMap;

use crate::build_provider::ddragon::get_spell_metadata_for_locale;
use crate::models::{
    AppLocale, ArenaAugmentMetadata, DataDragonCache, ItemMetadata, RecentMatchInfo, RuneMetadata,
    SpellMetadata, queue_supports_runes,
};
use crate::storage::is_allowed_asset_url;

const KNOWN_STANDARD_SPELL_IDS: [i32; 11] = [1, 3, 4, 6, 7, 11, 12, 13, 14, 21, 32];

/// Extracted keystone and primary/secondary rune tree metadata for a single match row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MatchRuneSummary {
    pub primary_rune_id: i32,
    pub primary_rune_icon_url: String,
    pub primary_style_id: i32,
    pub primary_style_icon_url: String,
    pub sub_style_id: i32,
    pub sub_style_icon_url: String,
}

fn positive_i32(stats: &serde_json::Value, key: &str) -> i32 {
    stats
        .get(key)
        .and_then(|v| v.as_i64())
        .filter(|&n| n > 0 && n <= i32::MAX as i64)
        .map(|n| n as i32)
        .unwrap_or(0)
}

/// Resolves a rune or style icon URL from `DataDragonCache` only when `rune_id > 0`
/// and the URL passes the asset allowlist.
pub fn resolve_allowed_rune_icon(cache: &DataDragonCache, rune_id: i32) -> String {
    if rune_id <= 0 {
        return String::new();
    }
    cache
        .runes
        .get(&rune_id)
        .map(|meta| meta.icon_url.trim())
        .filter(|url| !url.is_empty() && is_allowed_asset_url(url))
        .unwrap_or("")
        .to_string()
}

/// Extracts `perk0` (keystone), `perkPrimaryStyle`, and `perkSubStyle` and resolves icons
/// via `DataDragonCache.runes` strictly when IDs are positive (`> 0`).
pub fn extract_match_runes(stats: &serde_json::Value, cache: &DataDragonCache) -> MatchRuneSummary {
    let primary_rune_id = positive_i32(stats, "perk0");
    let primary_style_id = positive_i32(stats, "perkPrimaryStyle");
    let sub_style_id = positive_i32(stats, "perkSubStyle");

    MatchRuneSummary {
        primary_rune_id,
        primary_rune_icon_url: resolve_allowed_rune_icon(cache, primary_rune_id),
        primary_style_id,
        primary_style_icon_url: resolve_allowed_rune_icon(cache, primary_style_id),
        sub_style_id,
        sub_style_icon_url: resolve_allowed_rune_icon(cache, sub_style_id),
    }
}

/// Extracts `stats.item6` (trinket/ward) and builds its DataDragon item icon URL when present.
pub fn extract_match_trinket(stats: &serde_json::Value, ddragon_version: &str) -> (i32, String) {
    let trinket_id = positive_i32(stats, "item6");
    let version = ddragon_version.trim();
    let icon_url = if trinket_id > 0 && !version.is_empty() {
        format!("https://ddragon.leagueoflegends.com/cdn/{version}/img/item/{trinket_id}.png")
    } else {
        String::new()
    };
    (trinket_id, icon_url)
}

/// Returns true when the match ended via early remake surrender or < 3:30 with zero K/D/A.
pub fn is_remake_match(
    stats: &serde_json::Value,
    game_duration_secs: i64,
    kills: i32,
    deaths: i32,
    assists: i32,
) -> bool {
    if stats
        .get("gameEndedInEarlySurrender")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return true;
    }
    game_duration_secs > 0 && game_duration_secs < 210 && kills == 0 && deaths == 0 && assists == 0
}

/// Extracts Arena placement (`1..=8`) from `subteamPlacement` with `placement` fallback.
pub fn extract_arena_placement(stats: &serde_json::Value, game_mode_key: &str) -> Option<i32> {
    if game_mode_key != "arena" {
        return None;
    }
    stats
        .get("subteamPlacement")
        .and_then(|v| v.as_i64())
        .filter(|&n| (1..=8).contains(&n))
        .or_else(|| {
            stats
                .get("placement")
                .and_then(|v| v.as_i64())
                .filter(|&n| (1..=8).contains(&n))
        })
        .map(|n| n as i32)
}

/// Extracts non-zero augment IDs (`playerAugment1..6`) for rune-less modes such as Arena and ARAM Mayhem.
pub fn extract_match_augments(
    stats: &serde_json::Value,
    queue_id: i32,
    game_mode_key: &str,
    primary_rune_id: i32,
) -> Vec<i32> {
    let no_runes_mode = !queue_supports_runes(queue_id)
        || game_mode_key == "arena"
        || game_mode_key == "aram_mayhem"
        || primary_rune_id <= 0;
    if !no_runes_mode {
        return Vec::new();
    }

    let mut ids = Vec::new();
    for slot in 1..=6 {
        let key = format!("playerAugment{slot}");
        let id = positive_i32(stats, &key);
        if id > 0 && !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

/// Collects deduplicated DataDragon item metadata for all non-zero items and trinkets in `matches`.
pub fn collect_matches_item_metadata(
    matches: &[RecentMatchInfo],
    items_catalog: &HashMap<i32, ItemMetadata>,
) -> HashMap<i32, ItemMetadata> {
    let mut resolved = HashMap::new();
    for m in matches {
        for item_id in m.item_ids.iter().copied().chain(std::iter::once(m.trinket_id)) {
            if item_id > 0
                && !resolved.contains_key(&item_id)
                && let Some(meta) = items_catalog.get(&item_id)
            {
                resolved.insert(item_id, meta.clone());
            }
        }
    }
    resolved
}

/// Collects deduplicated DataDragon rune/style metadata for all positive rune IDs in `matches`.
pub fn collect_matches_rune_metadata(
    matches: &[RecentMatchInfo],
    runes_catalog: &HashMap<i32, RuneMetadata>,
) -> HashMap<i32, RuneMetadata> {
    let mut resolved = HashMap::new();
    for m in matches {
        for rune_id in [m.primary_rune_id, m.primary_style_id, m.sub_style_id] {
            if rune_id > 0
                && !resolved.contains_key(&rune_id)
                && let Some(meta) = runes_catalog.get(&rune_id)
            {
                resolved.insert(rune_id, meta.clone());
            }
        }
    }
    resolved
}

/// Collects deduplicated summoner spell metadata for all positive spell IDs in `matches`.
/// Falls back to localized built-in metadata only for known standard spells so Arena/special IDs
/// never receive misleading icons or labels.
pub fn collect_matches_spell_metadata(
    matches: &[RecentMatchInfo],
    cache: &DataDragonCache,
    locale: AppLocale,
) -> HashMap<i32, SpellMetadata> {
    let mut resolved = HashMap::new();
    for m in matches {
        for spell_id in [m.spell1_id, m.spell2_id] {
            if spell_id <= 0 || resolved.contains_key(&spell_id) {
                continue;
            }
            if let Some(meta) = cache.spells.get(&spell_id) {
                resolved.insert(spell_id, meta.clone());
            } else if KNOWN_STANDARD_SPELL_IDS.contains(&spell_id) {
                resolved.insert(
                    spell_id,
                    get_spell_metadata_for_locale(&cache.version, spell_id, locale),
                );
            }
        }
    }
    resolved
}

/// Collects deduplicated localized Arena/Mayhem augment metadata for all positive augment IDs in `matches`.
pub fn collect_matches_augment_metadata(
    matches: &[RecentMatchInfo],
    augments_catalog: &HashMap<i32, ArenaAugmentMetadata>,
) -> HashMap<i32, ArenaAugmentMetadata> {
    let mut resolved = HashMap::new();
    for m in matches {
        for &aug_id in &m.augment_ids {
            if aug_id > 0
                && !resolved.contains_key(&aug_id)
                && let Some(meta) = augments_catalog.get(&aug_id)
            {
                resolved.insert(aug_id, meta.clone());
            }
        }
    }
    resolved
}
