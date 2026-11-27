use super::super::client::OPGG_REST_API_BASE;
use super::{
    build_arena_url, build_parser, cache, fetch_arena_json, fetch_arena_versions_with_cache_dir,
};
use crate::build_provider::ddragon::enrich_build_with_ddragon_for_queue_with_flash_key_and_locale;
use crate::build_provider::models::{ChampionBuild, DataDragonCache};
use crate::lcu_injector::FlashKeyPreference;
use crate::models::{AppLocale, QUEUE_ARENA};
use std::path::Path;
use std::time::Instant;

const ARENA_MIN_CHAMPION_PATCH_PLAYS: i64 = 100;

#[allow(clippy::too_many_arguments)]
pub async fn fetch_arena_champion_build_with_base_and_dir(
    client: &reqwest::Client,
    rest_base: &str,
    champion_id: i32,
    patch: Option<&str>,
    ddragon: &DataDragonCache,
    patch_48h_protection: bool,
    now_unix: u64,
    locale: AppLocale,
    arena_dir: Option<&Path>,
) -> Result<ChampionBuild, String> {
    let started = Instant::now();
    let detail = format!("champ_id={champion_id}");
    let manual_patch = patch.map(str::trim).filter(|s| !s.is_empty());
    let cache_key = format!(
        "{}:{}:{}:{}:{}",
        rest_base,
        champion_id,
        manual_patch.unwrap_or(""),
        patch_48h_protection,
        locale.as_str()
    );

    if let Ok(guard) = cache::get_arena_build_cache().read()
        && let Some((ts, cached)) = guard.get(&cache_key)
        && ts.elapsed() < cache::ARENA_BUILD_CACHE_TTL
    {
        let mut build = cached.clone();
        enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
            &mut build,
            ddragon,
            FlashKeyPreference::D,
            locale,
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

    let disk_env = arena_dir.and_then(|dir| {
        let path = cache::arena_build_file_path(
            dir,
            champion_id,
            manual_patch,
            patch_48h_protection,
            locale,
        );
        crate::storage::read_lz4_envelope::<ChampionBuild>(&path)
    });

    if let Some(ref env) = disk_env
        && !env.payload.item_blocks.is_empty()
        && crate::storage::is_envelope_fresh(
            env.created_at_unix,
            crate::storage::OPGG_CACHE_TTL_SECS,
            now_unix,
        )
    {
        let known_latest = cache::peek_latest_cached_arena_version();
        let patch_valid = match (manual_patch, known_latest.as_deref()) {
            (Some(p), _) => env.patch == p,
            (None, Some(latest)) if !env.payload.patch_fallback_applied => env.patch == latest,
            _ => true,
        };
        if patch_valid {
            let mut build = env.payload.clone();
            enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
                &mut build,
                ddragon,
                FlashKeyPreference::D,
                locale,
                Some(QUEUE_ARENA),
            );
            if let Ok(mut guard) = cache::get_arena_build_cache().write() {
                guard.insert(cache_key, (Instant::now(), build.clone()));
            }
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
        fetch_arena_versions_with_cache_dir(client, rest_base, arena_dir).await;
    let mut effective_patch = manual_patch
        .map(str::to_string)
        .unwrap_or_else(|| available_versions.first().cloned().unwrap_or_default());
    let mut patch_fallback_applied = false;
    let patch_protection_remaining_hours: Option<u32> = None;

    let probe_version = (!effective_patch.is_empty()).then_some(effective_patch.as_str());
    let probe_url = build_arena_url(rest_base, Some(champion_id), probe_version);
    let mut fetch_res = fetch_arena_json(client, &probe_url, "Arena champion build").await;

    if manual_patch.is_none() && patch_48h_protection && available_versions.len() >= 2 {
        let low_sample = match &fetch_res {
            Ok(json) => {
                build_parser::extract_arena_champion_play_count(json)
                    < ARENA_MIN_CHAMPION_PATCH_PLAYS
            }
            Err(_) => true,
        };
        if low_sample {
            let prev_patch = available_versions[1].clone();
            let fallback_url = build_arena_url(rest_base, Some(champion_id), Some(&prev_patch));
            if let Ok(prev_json) =
                fetch_arena_json(client, &fallback_url, "Arena champion build fallback").await
            {
                effective_patch = prev_patch;
                patch_fallback_applied = true;
                fetch_res = Ok(prev_json);
            }
        }
    }

    let champ_name = ddragon
        .champions
        .get(&champion_id)
        .map(|c| c.display_name.as_str())
        .unwrap_or("Unknown");

    let mut build = match fetch_res.and_then(|json| {
        build_parser::parse_arena_champion_payload(&json, champion_id, champ_name, ddragon)
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
                    locale,
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
        locale,
        Some(QUEUE_ARENA),
    );

    if let Some(dir) = arena_dir {
        let path = cache::arena_build_file_path(
            dir,
            champion_id,
            manual_patch,
            patch_48h_protection,
            locale,
        );
        let _ = crate::storage::write_lz4_envelope(&path, &effective_patch, &build);
    }
    if let Ok(mut guard) = cache::get_arena_build_cache().write() {
        guard.insert(cache_key, (Instant::now(), build.clone()));
    }

    crate::logging::record_data_resolution(
        "arena_build",
        "network",
        started.elapsed().as_millis(),
        &detail,
    );
    Ok(build)
}

pub async fn fetch_arena_champion_build(
    client: &reqwest::Client,
    champion_id: i32,
    patch: Option<&str>,
    ddragon: &DataDragonCache,
) -> Result<ChampionBuild, String> {
    let prefs = crate::lcu_injector::get_user_preferences();
    let dir = crate::storage::global_paths().map(|p| p.opgg_arena_dir.as_path());
    fetch_arena_champion_build_with_base_and_dir(
        client,
        OPGG_REST_API_BASE,
        champion_id,
        patch,
        ddragon,
        prefs.patch_48h_protection,
        crate::storage::now_unix_secs(),
        prefs.language,
        dir,
    )
    .await
}
