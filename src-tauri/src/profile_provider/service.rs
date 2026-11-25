use crate::build_provider;
use crate::lcu_connector::LcuConnection;
use crate::models::{CachedProfileResult, DataDragonCache, PlayerProfile};

use super::cache::{
    get_cached_profile_for_last_summoner, get_cached_profile_for_summoner_id, save_profile_to_dir,
};
use super::masteries_parser::parse_top_masteries;
use super::match_metadata::{
    collect_matches_augment_metadata, collect_matches_item_metadata, collect_matches_rune_metadata,
    collect_matches_spell_metadata,
};
use super::matches_parser::parse_recent_matches_for_locale;
use super::ranked_parser::parse_ranked_stats_for_locale;
use super::skins::{
    extract_background_skin_id, fetch_skin_name_for_locale,
    resolve_background_splash_urls_for_locale,
};

/// Fetches and assembles the full `PlayerProfile` from an active LCU connection + DataDragon cache,
/// supporting disk `.lz4` caching (48h TTL, SWR > 5m, `force_refresh` bypass) and `LastSummonerId` persistence.
pub async fn load_player_profile_with_cache(
    conn: &LcuConnection,
    force_refresh: bool,
) -> Result<PlayerProfile, String> {
    let locale = crate::lcu_injector::get_active_locale();
    let (summoner, puuid) = conn.get_current_summoner_with_puuid().await?;
    let http_client = build_provider::shared_http_client();

    if summoner.summoner_id > 0 {
        let current_last = crate::storage::get_global_settings()
            .navigation
            .last_summoner_id;
        if current_last != Some(summoner.summoner_id) {
            let _ = crate::storage::update_global_settings(|s| {
                s.navigation.last_summoner_id = Some(summoner.summoner_id);
            });
        }
    }

    if !force_refresh
        && let Some(cached) = get_cached_profile_for_summoner_id(summoner.summoner_id, locale)
        && !cached.is_stale
    {
        return Ok(cached.profile);
    }

    let cache =
        match build_provider::get_or_init_ddragon_cache_for_locale(http_client, locale).await {
            Ok(c) => c.as_ref().clone(),
            Err(_) => DataDragonCache {
                version: build_provider::latest_cached_ddragon_version().unwrap_or_default(),
                ..Default::default()
            },
        };

    let (profile_res, masteries_res, ranked_res, history_res) = tokio::join!(
        conn.get_summoner_profile(),
        conn.get_champion_masteries(summoner.summoner_id, puuid.as_deref()),
        conn.get_ranked_stats(),
        conn.get_match_history()
    );

    let background_skin_id = profile_res
        .as_ref()
        .map(extract_background_skin_id)
        .unwrap_or(0);

    let (background_splash_url, background_splash_fallback_url, default_label, key_name, skin_num) =
        resolve_background_splash_urls_for_locale(background_skin_id, &cache, locale);

    let background_skin_name = if background_skin_id > 0 && !key_name.is_empty() {
        fetch_skin_name_for_locale(
            http_client,
            &cache.version,
            &key_name,
            skin_num,
            background_skin_id,
            locale,
        )
        .await
        .unwrap_or(default_label)
    } else {
        String::new()
    };

    let ranked_stats = ranked_res
        .as_ref()
        .map(|val| parse_ranked_stats_for_locale(val, locale))
        .unwrap_or_else(|_| parse_ranked_stats_for_locale(&serde_json::Value::Null, locale));

    let top_masteries = masteries_res
        .as_ref()
        .map(|val| parse_top_masteries(val, &cache, 3))
        .unwrap_or_default();

    let recent_matches = history_res
        .as_ref()
        .map(|val| parse_recent_matches_for_locale(val, summoner.summoner_id, &cache, 10, locale))
        .unwrap_or_default();
    let item_metadata = collect_matches_item_metadata(&recent_matches, &cache.items);
    let rune_metadata = collect_matches_rune_metadata(&recent_matches, &cache.runes);
    let spell_metadata = collect_matches_spell_metadata(&recent_matches, &cache, locale);
    let augment_metadata =
        collect_matches_augment_metadata(&recent_matches, &cache.arena_augments);

    let profile_icon_url = if !cache.version.is_empty() {
        format!(
            "https://ddragon.leagueoflegends.com/cdn/{}/img/profileicon/{}.png",
            cache.version, summoner.profile_icon_id
        )
    } else {
        String::new()
    };

    let profile = PlayerProfile {
        summoner,
        profile_icon_url,
        background_skin_id,
        background_skin_name,
        background_splash_url,
        background_splash_fallback_url,
        ranked_stats,
        top_masteries,
        recent_matches,
        item_metadata,
        rune_metadata,
        spell_metadata,
        augment_metadata,
    };

    if let Some(paths) = crate::storage::global_paths() {
        let _ = save_profile_to_dir(&paths.profile_dir, &profile, &cache.version);
    }

    Ok(profile)
}

pub async fn load_player_profile(conn: &LcuConnection) -> Result<PlayerProfile, String> {
    load_player_profile_with_cache(conn, false).await
}

/// Returns the cached profile for `summoner_id` (or the last known summoner) using the active locale.
pub fn get_cached_profile_for_active_locale(
    summoner_id: Option<u64>,
) -> Option<CachedProfileResult> {
    let locale = crate::lcu_injector::get_active_locale();
    match summoner_id {
        Some(id) if id > 0 => get_cached_profile_for_summoner_id(id, locale),
        _ => get_cached_profile_for_last_summoner(locale),
    }
}

/// Connects to the local LCU and loads the current player profile if connected.
pub async fn fetch_live_player_profile(force_refresh: bool) -> Option<PlayerProfile> {
    let conn = LcuConnection::connect().ok()?;
    load_player_profile_with_cache(&conn, force_refresh)
        .await
        .ok()
}
