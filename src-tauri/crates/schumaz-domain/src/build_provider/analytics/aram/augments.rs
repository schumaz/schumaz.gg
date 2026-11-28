use super::super::client::{OPGG_MCP_ENDPOINT, peek_latest_cached_opgg_version};
use super::super::cooldown::send_analytics_request;
use super::cache::{
    ARAM_MAYHEM_CACHE_TTL, aram_augments_cache_key, aram_augments_file_path,
    aram_mayhem_augments_web_base, has_valid_augment_descriptions, is_aram_patch_compatible,
    persist_aram_augments, rehydrate_aram_augments, sanitize_stale_augment_entries,
};
use super::html_extract::normalize_opgg_champion_key;
use super::mcp_augments::{
    fetch_opgg_mcp_aram_augments, parse_aram_mayhem_augments_mcp_for_locale,
    resolve_champion_id_for_aram,
};
use super::parsers::{
    merge_mcp_with_top_tier_supplement, parse_aram_mayhem_augments_html_for_locale,
};
use super::{ARAM_MAYHEM_HTML_FALLBACK_ENABLED, BROWSER_USER_AGENT};
use crate::build_provider::cache::build_provider_caches;
use crate::build_provider::models::DataDragonCache;
use crate::models::{AppLocale, AramAugmentEntry};
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub(crate) struct AramAugmentsQuery<'a> {
    pub champion_key: &'a str,
    pub mcp_endpoint: &'a str,
    pub web_base: &'a str,
    pub locale: AppLocale,
    pub aram_dir: Option<&'a Path>,
    pub known_latest: Option<&'a str>,
    pub html_fallback_enabled: bool,
}

async fn fetch_mayhem_augments_html_fallback(
    client: &reqwest::Client,
    web_base: &str,
    normalized_champ: &str,
    locale: AppLocale,
    ddragon: &DataDragonCache,
) -> Result<Vec<AramAugmentEntry>, String> {
    let url = format!(
        "{}/{}/augments",
        web_base.trim_end_matches('/'),
        normalized_champ
    );
    let res = send_analytics_request(
        &url,
        client
            .get(&url)
            .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT),
        "ARAM Mayhem augments (HTML fallback)",
    )
    .await?;
    let html = res
        .text()
        .await
        .map_err(|e| format!("Failed to read ARAM Mayhem augments HTML: {e}"))?;
    parse_aram_mayhem_augments_html_for_locale(&html, ddragon, locale)
}

pub(crate) async fn fetch_aram_mayhem_augments_internal(
    client: &reqwest::Client,
    query: &AramAugmentsQuery<'_>,
    ddragon: &DataDragonCache,
) -> Result<Vec<AramAugmentEntry>, String> {
    let started = Instant::now();
    let normalized = normalize_opgg_champion_key(query.champion_key);
    if normalized.is_empty() {
        return Err("Champion key cannot be empty".to_string());
    }

    let cache_key = aram_augments_cache_key(query.locale, query.mcp_endpoint, query.web_base, &normalized);
    let cache = &build_provider_caches().aram_augments;

    if let Some(entry) = cache.get_ram(&cache_key, Some(ARAM_MAYHEM_CACHE_TTL))
        && is_aram_patch_compatible(&entry.patch, query.known_latest)
    {
        let mut list = entry.payload;
        rehydrate_aram_augments(&mut list, ddragon);
        if !list.is_empty() && has_valid_augment_descriptions(&list) {
            crate::logging::record_data_resolution("aram_augments", "ram_hit", started.elapsed().as_millis(), "");
            return Ok(list);
        }
    }

    cache
        .coalesce(cache_key.clone(), || async {
            if let Some(entry) = cache.get_ram(&cache_key, Some(ARAM_MAYHEM_CACHE_TTL))
                && is_aram_patch_compatible(&entry.patch, query.known_latest)
            {
                let mut list = entry.payload;
                rehydrate_aram_augments(&mut list, ddragon);
                if !list.is_empty() && has_valid_augment_descriptions(&list) {
                    return Ok(list);
                }
            }

            let disk_env = query.aram_dir.and_then(|dir| {
                let path = aram_augments_file_path(dir, query.locale, &normalized);
                cache.read_disk(&path)
            });

            if let Some(ref env) = disk_env
                && !env.payload.is_empty()
                && crate::storage::is_envelope_fresh(
                    env.created_at_unix,
                    crate::storage::ARAM_MAYHEM_DISK_TTL_SECS,
                    crate::storage::now_unix_secs(),
                )
                && is_aram_patch_compatible(&env.patch, query.known_latest)
            {
                let mut list = env.payload.clone();
                rehydrate_aram_augments(&mut list, ddragon);
                if !list.is_empty() && has_valid_augment_descriptions(&list) {
                    cache.insert_ram(cache_key.clone(), env.patch.clone(), list.clone());
                    crate::logging::record_data_resolution("aram_augments", "disk_hit", started.elapsed().as_millis(), "");
                    return Ok(list);
                }
            }

            let champion_id = resolve_champion_id_for_aram(query.champion_key, ddragon)
                .or_else(|| resolve_champion_id_for_aram(&normalized, ddragon));
            let mcp_res = match champion_id {
                Some(champ_id) => {
                    match fetch_opgg_mcp_aram_augments(client, query.mcp_endpoint, champ_id, query.locale).await {
                        Ok(json_body) => parse_aram_mayhem_augments_mcp_for_locale(&json_body, ddragon, query.locale),
                        Err(err) => Err(err),
                    }
                }
                None => Err(format!("Champion ID not found in DataDragon cache for key: {}", query.champion_key)),
            };

            let (augments, source_detail) = match mcp_res {
                Ok(mcp_list) => {
                    let missing_top_tiers = !mcp_list.iter().any(|entry| entry.tier < 3);
                    if missing_top_tiers && query.html_fallback_enabled {
                        match fetch_mayhem_augments_html_fallback(client, query.web_base, &normalized, query.locale, ddragon).await {
                            Ok(html_list) => (merge_mcp_with_top_tier_supplement(mcp_list, html_list), "source=mcp+html_top_tiers"),
                            Err(_) => (mcp_list, "source=mcp"),
                        }
                    } else {
                        (mcp_list, "source=mcp")
                    }
                }
                Err(primary_err) => {
                    let stale_candidate = disk_env.and_then(|env| {
                        if env.payload.is_empty() {
                            return None;
                        }
                        let mut rescued = env.payload;
                        rehydrate_aram_augments(&mut rescued, ddragon);
                        (!rescued.is_empty()).then_some(rescued)
                    });

                    if let Some(ref clean_stale) = stale_candidate
                        && has_valid_augment_descriptions(clean_stale)
                    {
                        crate::logging::record_data_resolution("aram_augments", "stale_fallback", started.elapsed().as_millis(), "");
                        return Ok(clean_stale.clone());
                    }

                    if query.html_fallback_enabled
                        && let Ok(html_list) = fetch_mayhem_augments_html_fallback(client, query.web_base, &normalized, query.locale, ddragon).await
                        && !html_list.is_empty()
                    {
                        (html_list, "source=html_fallback")
                    } else if let Some(mut dirty_stale) = stale_candidate {
                        sanitize_stale_augment_entries(&mut dirty_stale, query.locale);
                        if !dirty_stale.is_empty() {
                            crate::logging::record_data_resolution("aram_augments", "stale_fallback", started.elapsed().as_millis(), "");
                            return Ok(dirty_stale);
                        }
                        return Err(primary_err);
                    } else {
                        return Err(primary_err);
                    }
                }
            };

            persist_aram_augments(&augments, query.locale, &normalized, query.aram_dir, query.known_latest, cache_key.clone());
            crate::logging::record_data_resolution("aram_augments", "network", started.elapsed().as_millis(), source_detail);
            Ok(augments)
        })
        .await
}

pub async fn fetch_aram_mayhem_augments_with_endpoints_locale_and_dir(
    client: &reqwest::Client,
    champion_key: &str,
    mcp_endpoint: &str,
    web_base: &str,
    locale: AppLocale,
    ddragon: &DataDragonCache,
    aram_dir: Option<&Path>,
) -> Result<Vec<AramAugmentEntry>, String> {
    let known_latest = peek_latest_cached_opgg_version("global");
    fetch_aram_mayhem_augments_internal(
        client,
        &AramAugmentsQuery {
            champion_key,
            mcp_endpoint,
            web_base,
            locale,
            aram_dir,
            known_latest: known_latest.as_deref(),
            html_fallback_enabled: ARAM_MAYHEM_HTML_FALLBACK_ENABLED,
        },
        ddragon,
    )
    .await
}

pub async fn fetch_aram_mayhem_augments_with_base_locale_and_dir(
    client: &reqwest::Client,
    champion_key: &str,
    web_base: &str,
    locale: AppLocale,
    ddragon: &DataDragonCache,
    aram_dir: Option<&Path>,
) -> Result<Vec<AramAugmentEntry>, String> {
    fetch_aram_mayhem_augments_with_endpoints_locale_and_dir(
        client,
        champion_key,
        OPGG_MCP_ENDPOINT,
        web_base,
        locale,
        ddragon,
        aram_dir,
    )
    .await
}

pub async fn fetch_aram_mayhem_augments_with_base_and_locale(
    client: &reqwest::Client,
    champion_key: &str,
    web_base: &str,
    locale: AppLocale,
    ddragon: &DataDragonCache,
) -> Result<Vec<AramAugmentEntry>, String> {
    let dir = if web_base == aram_mayhem_augments_web_base(locale) {
        crate::storage::global_paths().map(|p| p.opgg_aram_dir.as_path())
    } else {
        None
    };
    fetch_aram_mayhem_augments_with_base_locale_and_dir(
        client,
        champion_key,
        web_base,
        locale,
        ddragon,
        dir,
    )
    .await
}

pub async fn fetch_aram_mayhem_augments_with_base(
    client: &reqwest::Client,
    champion_key: &str,
    web_base: &str,
    ddragon: &DataDragonCache,
) -> Result<Vec<AramAugmentEntry>, String> {
    fetch_aram_mayhem_augments_with_base_and_locale(
        client,
        champion_key,
        web_base,
        crate::storage::get_active_locale(),
        ddragon,
    )
    .await
}

pub async fn fetch_aram_mayhem_augments_for_locale(
    client: &reqwest::Client,
    champion_key: &str,
    locale: AppLocale,
    ddragon: &DataDragonCache,
) -> Result<Vec<AramAugmentEntry>, String> {
    fetch_aram_mayhem_augments_with_base_and_locale(
        client,
        champion_key,
        aram_mayhem_augments_web_base(locale),
        locale,
        ddragon,
    )
    .await
}

pub async fn fetch_aram_mayhem_augments(
    client: &reqwest::Client,
    champion_key: &str,
    ddragon: &DataDragonCache,
) -> Result<Vec<AramAugmentEntry>, String> {
    let locale = crate::storage::get_active_locale();
    fetch_aram_mayhem_augments_for_locale(client, champion_key, locale, ddragon).await
}
