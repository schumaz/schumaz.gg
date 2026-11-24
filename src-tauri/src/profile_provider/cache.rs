use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::build_provider;
use crate::models::{
    AppLocale, CachedProfileResult, DataDragonCache, ItemMetadata, PlayerProfile, RecentMatchInfo,
};

use super::queue_mapper::map_queue_mode_for_locale;
use super::ranked_parser::translate_ranked_tier_for_locale;
use super::skins::{
    lookup_cached_skin_name, prewarm_skins_from_dir, resolve_background_splash_urls_for_locale,
};

pub fn profile_cache_file_path(dir: &Path, summoner_id: u64) -> PathBuf {
    dir.join(format!("summoner_{summoner_id}.lz4"))
}

pub fn save_profile_to_dir(dir: &Path, profile: &PlayerProfile, patch: &str) -> Result<(), String> {
    if profile.summoner.summoner_id == 0 {
        return Ok(());
    }
    let path = profile_cache_file_path(dir, profile.summoner.summoner_id);
    crate::storage::write_lz4_envelope(&path, patch, profile)
}

pub fn load_profile_from_dir(
    dir: &Path,
    summoner_id: u64,
    now_unix: u64,
) -> Option<CachedProfileResult> {
    if summoner_id == 0 {
        return None;
    }
    let path = profile_cache_file_path(dir, summoner_id);
    let env = crate::storage::read_lz4_envelope::<PlayerProfile>(&path)?;
    if !crate::storage::is_envelope_fresh(
        env.created_at_unix,
        crate::storage::PROFILE_CACHE_TTL_SECS,
        now_unix,
    ) {
        return None;
    }
    let is_stale = !crate::storage::is_envelope_fresh(
        env.created_at_unix,
        crate::storage::PROFILE_SWR_SECS,
        now_unix,
    );
    Some(CachedProfileResult {
        profile: env.payload,
        created_at_unix: env.created_at_unix,
        is_stale,
    })
}

/// Collects deduplicated DataDragon item metadata for all non-zero item IDs in `matches`.
pub fn collect_matches_item_metadata(
    matches: &[RecentMatchInfo],
    items_catalog: &HashMap<i32, ItemMetadata>,
) -> HashMap<i32, ItemMetadata> {
    let mut resolved = HashMap::new();
    for m in matches {
        for &item_id in &m.item_ids {
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

/// Re-localizes cached ranked tier labels, queue mode labels, champion names,
/// item tooltip metadata, and cached skin name in-memory when switching UI language without waiting on LCU.
pub fn relocalize_player_profile(
    profile: &mut PlayerProfile,
    locale: AppLocale,
    ddragon: Option<&DataDragonCache>,
) {
    for stat in &mut profile.ranked_stats {
        stat.tier_label = translate_ranked_tier_for_locale(&stat.tier, locale);
    }

    for m in &mut profile.recent_matches {
        let (_, label) = map_queue_mode_for_locale(m.queue_id, &m.game_mode_key, locale);
        m.game_mode_label = label;
        if let Some(cache) = ddragon
            && let Some(champ) = cache.champions.get(&m.champion_id)
        {
            m.champion_name = champ.display_name.clone();
        }
    }

    if let Some(cache) = ddragon {
        for mastery in &mut profile.top_masteries {
            if let Some(champ) = cache.champions.get(&mastery.champion_id) {
                mastery.champion_name = champ.display_name.clone();
            }
        }
        if !cache.items.is_empty() {
            profile.item_metadata =
                collect_matches_item_metadata(&profile.recent_matches, &cache.items);
        }
    }

    if let Some(skin_name) = lookup_cached_skin_name(locale, profile.background_skin_id) {
        profile.background_skin_name = skin_name;
    } else if profile.background_skin_id > 0
        && let Some(cache) = ddragon
    {
        let (_, _, default_label, _, _) =
            resolve_background_splash_urls_for_locale(profile.background_skin_id, cache, locale);
        if profile.background_skin_name.is_empty()
            || profile.background_skin_name.ends_with("(Clássica)")
            || profile.background_skin_name.ends_with("(Classic)")
        {
            profile.background_skin_name = default_label;
        }
    }
}

pub fn get_cached_profile_for_summoner_id(
    summoner_id: u64,
    locale: AppLocale,
) -> Option<CachedProfileResult> {
    let paths = crate::storage::global_paths()?;
    prewarm_skins_from_dir(&paths.ddragon_dir, locale);
    let now = crate::storage::now_unix_secs();
    let mut cached = load_profile_from_dir(&paths.profile_dir, summoner_id, now)?;
    let dd_cache = build_provider::get_ddragon_cache_store()
        .read()
        .ok()
        .and_then(|g| g.get(&locale).cloned())
        .or_else(|| build_provider::ddragon::prewarm_ddragon_from_disk(locale));
    relocalize_player_profile(&mut cached.profile, locale, dd_cache.as_deref());
    Some(cached)
}

pub fn get_cached_profile_for_last_summoner(locale: AppLocale) -> Option<CachedProfileResult> {
    let last_id = crate::storage::get_global_settings()
        .navigation
        .last_summoner_id?;
    get_cached_profile_for_summoner_id(last_id, locale)
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
