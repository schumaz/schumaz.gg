use super::super::client::OPGG_REST_API_BASE;
use super::{
    build_arena_url, build_parser, cache, fetch_arena_json, fetch_arena_versions_with_cache_dir,
};
use crate::build_provider::cache::build_provider_caches;
use crate::build_provider::ddragon::enrich_build_with_ddragon_for_queue_with_flash_key_and_locale;
use crate::build_provider::models::{ChampionBuild, DataDragonCache};
use crate::models::{AppLocale, FlashKeyPreference, QUEUE_ARENA};
use std::path::Path;
use std::time::Instant;

const ARENA_MIN_CHAMPION_PATCH_PLAYS: i64 = 100;

#[derive(Debug, Clone, Copy)]
pub struct ArenaBuildQuery<'a> {
    pub rest_base: &'a str,
    pub champion_id: i32,
    pub patch: Option<&'a str>,
    pub patch_48h_protection: bool,
    pub now_unix: u64,
    pub locale: AppLocale,
    pub arena_dir: Option<&'a Path>,
}

pub async fn fetch_arena_champion_build_with_query(
    client: &reqwest::Client,
    query: &ArenaBuildQuery<'_>,
    ddragon: &DataDragonCache,
) -> Result<ChampionBuild, String> {
    let started = Instant::now();
    let detail = format!("champ_id={}", query.champion_id);
    let manual_patch = query.patch.map(str::trim).filter(|s| !s.is_empty());
    let cache_key = format!(
        "{}:{}:{}:{}:{}",
        query.rest_base,
        query.champion_id,
        manual_patch.unwrap_or(""),
        query.patch_48h_protection,
        query.locale.as_str()
    );
    let cache = &build_provider_caches().arena_builds;

    if let Some(entry) = cache.get_ram(&cache_key, Some(cache::ARENA_BUILD_CACHE_TTL)) {
        let mut build = entry.payload;
        enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
            &mut build,
            ddragon,
            FlashKeyPreference::D,
            query.locale,
            Some(QUEUE_ARENA),
        );
        crate::logging::record_data_resolution(
            "arena_build",
            "ram_hit",
            started.elapsed().as_millis(),
            &detail,
        );
        return Ok(build);
    }

    cache
        .coalesce(cache_key.clone(), || async {
            if let Some(entry) = cache.get_ram(&cache_key, Some(cache::ARENA_BUILD_CACHE_TTL)) {
                let mut build = entry.payload;
                enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
                    &mut build,
                    ddragon,
                    FlashKeyPreference::D,
                    query.locale,
                    Some(QUEUE_ARENA),
                );
                return Ok(build);
            }

            let disk_env = query.arena_dir.and_then(|dir| {
                let path = cache::arena_build_file_path(
                    dir,
                    query.champion_id,
                    manual_patch,
                    query.patch_48h_protection,
                    query.locale,
                );
                cache.read_disk(&path)
            });

            if let Some(ref env) = disk_env
                && !env.payload.item_blocks.is_empty()
                && crate::storage::is_envelope_fresh(
                    env.created_at_unix,
                    crate::storage::OPGG_CACHE_TTL_SECS,
                    query.now_unix,
                )
            {
                let known_latest = cache::peek_latest_cached_arena_version();
                let patch_valid = match (manual_patch, known_latest.as_deref()) {
                    (Some(p), _) => env.patch == p,
                    (None, Some(latest)) if !env.payload.patch_fallback_applied => {
                        env.patch == latest
                    }
                    _ => true,
                };
                if patch_valid {
                    let mut build = env.payload.clone();
                    enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
                        &mut build,
                        ddragon,
                        FlashKeyPreference::D,
                        query.locale,
                        Some(QUEUE_ARENA),
                    );
                    cache.insert_ram(cache_key.clone(), env.patch.clone(), build.clone());
                    crate::logging::record_data_resolution(
                        "arena_build",
                        "disk_hit",
                        started.elapsed().as_millis(),
                        &detail,
                    );
                    return Ok(build);
                }
            }

            let available_versions =
                fetch_arena_versions_with_cache_dir(client, query.rest_base, query.arena_dir)
                    .await;
            let mut effective_patch = manual_patch
                .map(str::to_string)
                .unwrap_or_else(|| available_versions.first().cloned().unwrap_or_default());
            let mut patch_fallback_applied = false;
            let patch_protection_remaining_hours: Option<u32> = None;

            let probe_version = (!effective_patch.is_empty()).then_some(effective_patch.as_str());
            let probe_url =
                build_arena_url(query.rest_base, Some(query.champion_id), probe_version);
            let mut fetch_res =
                fetch_arena_json(client, &probe_url, "Arena champion build").await;

            if manual_patch.is_none()
                && query.patch_48h_protection
                && available_versions.len() >= 2
            {
                let low_sample = match &fetch_res {
                    Ok(json) => {
                        build_parser::extract_arena_champion_play_count(json)
                            < ARENA_MIN_CHAMPION_PATCH_PLAYS
                    }
                    Err(_) => true,
                };
                if low_sample {
                    let prev_patch = available_versions[1].clone();
                    let fallback_url =
                        build_arena_url(query.rest_base, Some(query.champion_id), Some(&prev_patch));
                    if let Ok(prev_json) =
                        fetch_arena_json(client, &fallback_url, "Arena champion build fallback")
                            .await
                    {
                        effective_patch = prev_patch;
                        patch_fallback_applied = true;
                        fetch_res = Ok(prev_json);
                    }
                }
            }

            let champ_name = ddragon
                .champions
                .get(&query.champion_id)
                .map(|c| c.display_name.as_str())
                .unwrap_or("Unknown");

            let mut build = match fetch_res.and_then(|json| {
                build_parser::parse_arena_champion_payload(
                    &json,
                    query.champion_id,
                    champ_name,
                    ddragon,
                )
            }) {
                Ok(b) => b,
                Err(err) => {
                    if let Some(env) = disk_env
                        && !env.payload.item_blocks.is_empty()
                    {
                        let mut stale = env.payload;
                        enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
                            &mut stale,
                            ddragon,
                            FlashKeyPreference::D,
                            query.locale,
                            Some(QUEUE_ARENA),
                        );
                        crate::logging::record_data_resolution(
                            "arena_build",
                            "stale_fallback",
                            started.elapsed().as_millis(),
                            &detail,
                        );
                        return Ok(stale);
                    }
                    return Err(err);
                }
            };

            build.active_patch = effective_patch.clone();
            build.available_patches = available_versions;
            build.patch_fallback_applied = patch_fallback_applied;
            build.patch_protection_remaining_hours = patch_protection_remaining_hours;

            enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
                &mut build,
                ddragon,
                FlashKeyPreference::D,
                query.locale,
                Some(QUEUE_ARENA),
            );

            if let Some(dir) = query.arena_dir {
                let path = cache::arena_build_file_path(
                    dir,
                    query.champion_id,
                    manual_patch,
                    query.patch_48h_protection,
                    query.locale,
                );
                let _ = cache.write_disk(&path, &effective_patch, &build);
            }
            cache.insert_ram(cache_key.clone(), &effective_patch, build.clone());

            crate::logging::record_data_resolution(
                "arena_build",
                "network",
                started.elapsed().as_millis(),
                &detail,
            );
            Ok(build)
        })
        .await
}

pub async fn fetch_arena_champion_build(
    client: &reqwest::Client,
    champion_id: i32,
    patch: Option<&str>,
    ddragon: &DataDragonCache,
) -> Result<ChampionBuild, String> {
    let prefs = crate::storage::get_user_preferences();
    let dir = crate::storage::global_paths().map(|p| p.opgg_arena_dir.as_path());
    fetch_arena_champion_build_with_query(
        client,
        &ArenaBuildQuery {
            rest_base: OPGG_REST_API_BASE,
            champion_id,
            patch,
            patch_48h_protection: prefs.patch_48h_protection,
            now_unix: crate::storage::now_unix_secs(),
            locale: prefs.language,
            arena_dir: dir,
        },
        ddragon,
    )
    .await
}
