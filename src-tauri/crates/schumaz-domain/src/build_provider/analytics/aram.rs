pub mod augments;
mod cache;
pub mod html_extract;
pub mod mcp_augments;
mod parsers;
mod tooltip_clean;

pub use augments::*;
pub use cache::*;
pub use html_extract::*;
pub use mcp_augments::*;
pub use parsers::*;
pub use tooltip_clean::*;

use super::client::peek_latest_cached_opgg_version;
use super::cooldown::send_analytics_request;
use crate::build_provider::cache::build_provider_caches;
use crate::build_provider::models::DataDragonCache;
use crate::models::{AppLocale, AramChampionRank, AramMode};
use std::path::Path;
use std::time::Instant;

pub const OPGG_ARAM_REST_URL: &str = "https://lol-api-champion.op.gg/api/global/champions/aram";
pub const OPGG_ARAM_MAYHEM_WEB_URL: &str = "https://op.gg/lol/modes/aram-mayhem";
pub const OPGG_ARAM_MAYHEM_AUGMENTS_WEB_BASE: &str = "https://op.gg/pt/lol/modes/aram-mayhem";

/// Single switch controlling the isolated HTML extractor for ARAM Mayhem.
pub const ARAM_MAYHEM_HTML_FALLBACK_ENABLED: bool = true;

pub(crate) const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Copy)]
pub(crate) struct AramTierQuery<'a> {
    pub mode: AramMode,
    pub aram_rest_url: &'a str,
    pub mayhem_web_url: &'a str,
    pub locale: AppLocale,
    pub aram_dir: Option<&'a Path>,
    pub known_latest: Option<&'a str>,
    pub html_fallback_enabled: bool,
}

async fn fetch_aram_rest_champions(
    client: &reqwest::Client,
    aram_rest_url: &str,
    ddragon: &DataDragonCache,
    label: &str,
) -> Result<Vec<AramChampionRank>, String> {
    let res = send_analytics_request(aram_rest_url, client.get(aram_rest_url), label).await?;
    let parsed: parsers::RestAramChampionResponse = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse ARAM tier list JSON: {}", e))?;
    let list = parsers::parse_classic_aram_champions_from_resp(parsed, ddragon);
    if list.is_empty() {
        return Err("ARAM REST tier list returned no champions".to_string());
    }
    Ok(list)
}

async fn fetch_mayhem_champions_html_fallback(
    client: &reqwest::Client,
    mayhem_web_url: &str,
    ddragon: &DataDragonCache,
) -> Result<Vec<AramChampionRank>, String> {
    let res = send_analytics_request(
        mayhem_web_url,
        client
            .get(mayhem_web_url)
            .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT),
        "ARAM Mayhem tier list (HTML fallback)",
    )
    .await?;
    let html = res
        .text()
        .await
        .map_err(|e| format!("Failed to read ARAM Mayhem HTML: {}", e))?;
    parsers::parse_aram_mayhem_champions_html(&html, ddragon)
}

fn persist_aram_tier_list(
    list: &[AramChampionRank],
    query: &AramTierQuery<'_>,
    cache_key: String,
) {
    if list.is_empty() {
        return;
    }
    let patch = query.known_latest.unwrap_or("");
    let cache = &build_provider_caches().aram_tiers;
    if let Some(dir) = query.aram_dir {
        let path = cache::aram_tier_file_path(dir, query.mode, query.locale);
        let _ = cache.write_disk(&path, patch, &list.to_vec());
    }
    cache.insert_ram(cache_key, patch, list.to_vec());
}

pub(crate) async fn fetch_aram_tier_list_internal(
    client: &reqwest::Client,
    query: &AramTierQuery<'_>,
    ddragon: &DataDragonCache,
) -> Result<Vec<AramChampionRank>, String> {
    let started = Instant::now();
    let kind = match query.mode {
        AramMode::Aram => "aram_tier_list",
        AramMode::AramMayhem => "aram_mayhem_tier_list",
    };
    let cache_key = cache::aram_tier_cache_key(
        query.mode,
        query.locale,
        query.aram_rest_url,
        query.mayhem_web_url,
    );
    let ram_ttl = cache::aram_ram_ttl(query.mode);
    let disk_ttl_secs = cache::aram_disk_ttl_secs(query.mode);
    let cache = &build_provider_caches().aram_tiers;

    if let Some(entry) = cache.get_ram(&cache_key, Some(ram_ttl))
        && cache::is_aram_patch_compatible(&entry.patch, query.known_latest)
    {
        let mut list = entry.payload;
        cache::rehydrate_aram_names(&mut list, ddragon);
        crate::logging::record_data_resolution(kind, "ram_hit", started.elapsed().as_millis(), "");
        return Ok(list);
    }

    cache
        .coalesce(cache_key.clone(), || async {
            if let Some(entry) = cache.get_ram(&cache_key, Some(ram_ttl))
                && cache::is_aram_patch_compatible(&entry.patch, query.known_latest)
            {
                let mut list = entry.payload;
                cache::rehydrate_aram_names(&mut list, ddragon);
                return Ok(list);
            }

            let disk_env = query.aram_dir.and_then(|dir| {
                let path = cache::aram_tier_file_path(dir, query.mode, query.locale);
                cache.read_disk(&path)
            });

            if let Some(ref env) = disk_env
                && !env.payload.is_empty()
                && crate::storage::is_envelope_fresh(
                    env.created_at_unix,
                    disk_ttl_secs,
                    crate::storage::now_unix_secs(),
                )
                && cache::is_aram_patch_compatible(&env.patch, query.known_latest)
            {
                let mut list = env.payload.clone();
                cache::rehydrate_aram_names(&mut list, ddragon);
                cache.insert_ram(cache_key.clone(), env.patch.clone(), list.clone());
                crate::logging::record_data_resolution(
                    kind,
                    "disk_hit",
                    started.elapsed().as_millis(),
                    "",
                );
                return Ok(list);
            }

            let primary_label = match query.mode {
                AramMode::Aram => "ARAM tier list",
                AramMode::AramMayhem => "ARAM Mayhem champion list",
            };
            let primary_res =
                fetch_aram_rest_champions(client, query.aram_rest_url, ddragon, primary_label)
                    .await;

            let (list, source_detail) = match primary_res {
                Ok(list) => (list, "source=rest"),
                Err(primary_err) => {
                    if let Some(env) = disk_env
                        && !env.payload.is_empty()
                    {
                        let mut rescued = env.payload;
                        cache::rehydrate_aram_names(&mut rescued, ddragon);
                        crate::logging::record_data_resolution(
                            kind,
                            "stale_fallback",
                            started.elapsed().as_millis(),
                            "",
                        );
                        return Ok(rescued);
                    }

                    if query.mode == AramMode::AramMayhem && query.html_fallback_enabled {
                        match fetch_mayhem_champions_html_fallback(
                            client,
                            query.mayhem_web_url,
                            ddragon,
                        )
                        .await
                        {
                            Ok(list) => (list, "source=html_fallback"),
                            Err(_) => return Err(primary_err),
                        }
                    } else {
                        return Err(primary_err);
                    }
                }
            };

            persist_aram_tier_list(&list, query, cache_key.clone());
            crate::logging::record_data_resolution(
                kind,
                "network",
                started.elapsed().as_millis(),
                source_detail,
            );
            Ok(list)
        })
        .await
}

pub async fn fetch_aram_tier_list_with_urls_and_dir(
    client: &reqwest::Client,
    mode: AramMode,
    ddragon: &DataDragonCache,
    aram_rest_url: &str,
    mayhem_web_url: &str,
    locale: AppLocale,
    aram_dir: Option<&Path>,
) -> Result<Vec<AramChampionRank>, String> {
    let known_latest = peek_latest_cached_opgg_version("global");
    fetch_aram_tier_list_internal(
        client,
        &AramTierQuery {
            mode,
            aram_rest_url,
            mayhem_web_url,
            locale,
            aram_dir,
            known_latest: known_latest.as_deref(),
            html_fallback_enabled: ARAM_MAYHEM_HTML_FALLBACK_ENABLED,
        },
        ddragon,
    )
    .await
}

pub async fn fetch_aram_tier_list_with_urls(
    client: &reqwest::Client,
    mode: AramMode,
    ddragon: &DataDragonCache,
    aram_rest_url: &str,
    mayhem_web_url: &str,
) -> Result<Vec<AramChampionRank>, String> {
    let locale = crate::storage::get_active_locale();
    let dir = if aram_rest_url == OPGG_ARAM_REST_URL && mayhem_web_url == OPGG_ARAM_MAYHEM_WEB_URL {
        crate::storage::global_paths().map(|p| p.opgg_aram_dir.as_path())
    } else {
        None
    };
    fetch_aram_tier_list_with_urls_and_dir(
        client,
        mode,
        ddragon,
        aram_rest_url,
        mayhem_web_url,
        locale,
        dir,
    )
    .await
}

pub async fn fetch_aram_tier_list(
    client: &reqwest::Client,
    mode: AramMode,
    ddragon: &DataDragonCache,
) -> Result<Vec<AramChampionRank>, String> {
    fetch_aram_tier_list_with_urls(
        client,
        mode,
        ddragon,
        OPGG_ARAM_REST_URL,
        OPGG_ARAM_MAYHEM_WEB_URL,
    )
    .await
}

#[cfg(test)]
#[path = "aram/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "aram/cache_tests.rs"]
mod cache_tests;

#[cfg(test)]
#[path = "aram/fallback_tests.rs"]
mod fallback_tests;
