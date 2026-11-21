pub mod augments;
mod cache;
pub mod html_extract;
mod parsers;

pub use augments::*;
pub use cache::*;
pub use html_extract::*;
pub use parsers::*;

use super::client::peek_latest_cached_opgg_version;
use super::cooldown::send_analytics_request;
use crate::build_provider::models::DataDragonCache;
use crate::models::{AppLocale, AramChampionRank, AramMode};
use std::path::Path;
use std::time::Instant;

pub const OPGG_ARAM_REST_URL: &str = "https://lol-api-champion.op.gg/api/global/champions/aram";
pub const OPGG_ARAM_MAYHEM_WEB_URL: &str = "https://op.gg/lol/modes/aram-mayhem";
pub const OPGG_ARAM_MAYHEM_AUGMENTS_WEB_BASE: &str = "https://op.gg/pt/lol/modes/aram-mayhem";

pub(crate) const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";

#[allow(clippy::too_many_arguments)]
pub(crate) async fn fetch_aram_tier_list_internal(
    client: &reqwest::Client,
    mode: AramMode,
    ddragon: &DataDragonCache,
    aram_rest_url: &str,
    mayhem_web_url: &str,
    locale: AppLocale,
    aram_dir: Option<&Path>,
    known_latest: Option<&str>,
) -> Result<Vec<AramChampionRank>, String> {
    let started = Instant::now();
    let kind = match mode {
        AramMode::Aram => "aram_tier_list",
        AramMode::AramMayhem => "aram_mayhem_tier_list",
    };
    let cache_key = match mode {
        AramMode::Aram => format!("aram:{}:{}", locale.as_str(), aram_rest_url),
        AramMode::AramMayhem => format!("mayhem:{}:{}", locale.as_str(), mayhem_web_url),
    };
    let ram_ttl = cache::aram_ram_ttl(mode);
    let disk_ttl_secs = cache::aram_disk_ttl_secs(mode);

    if let Ok(guard) = cache::get_tier_cache().read()
        && let Some(entry) = guard.get(&cache_key)
        && entry.cached_at.elapsed() < ram_ttl
        && cache::is_aram_patch_compatible(&entry.patch, known_latest)
    {
        let mut list = entry.payload.clone();
        cache::rehydrate_aram_names(&mut list, ddragon);
        crate::logging::record_data_resolution(kind, "ram_hit", started.elapsed().as_millis(), "");
        return Ok(list);
    }

    let disk_env = aram_dir.and_then(|dir| {
        let path = cache::aram_tier_file_path(dir, mode, locale);
        crate::storage::read_lz4_envelope::<Vec<AramChampionRank>>(&path)
    });

    if let Some(ref env) = disk_env
        && !env.payload.is_empty()
        && crate::storage::is_envelope_fresh(
            env.created_at_unix,
            disk_ttl_secs,
            crate::storage::now_unix_secs(),
        )
        && cache::is_aram_patch_compatible(&env.patch, known_latest)
    {
        let mut list = env.payload.clone();
        cache::rehydrate_aram_names(&mut list, ddragon);
        if let Ok(mut guard) = cache::get_tier_cache().write() {
            guard.insert(
                cache_key.clone(),
                cache::CachedAramEntry {
                    cached_at: Instant::now(),
                    patch: env.patch.clone(),
                    payload: list.clone(),
                },
            );
        }
        crate::logging::record_data_resolution(kind, "disk_hit", started.elapsed().as_millis(), "");
        return Ok(list);
    }

    let fetch_res: Result<Vec<AramChampionRank>, String> = match mode {
        AramMode::Aram => {
            async {
                let res = send_analytics_request(
                    aram_rest_url,
                    client.get(aram_rest_url),
                    "ARAM tier list",
                )
                .await?;
                let json: serde_json::Value = res
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse ARAM tier list JSON: {}", e))?;
                parsers::parse_classic_aram_champions(&json, ddragon)
            }
            .await
        }
        AramMode::AramMayhem => {
            async {
                let res = send_analytics_request(
                    mayhem_web_url,
                    client
                        .get(mayhem_web_url)
                        .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT),
                    "ARAM Mayhem tier list",
                )
                .await?;
                let html = res
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read ARAM Mayhem HTML: {}", e))?;
                parsers::parse_aram_mayhem_champions_html(&html, ddragon)
            }
            .await
        }
    };

    let list = match fetch_res {
        Ok(list) => list,
        Err(err) => {
            // Stale-While-Error offline fallback
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
            return Err(err);
        }
    };

    if !list.is_empty() {
        let patch = known_latest.unwrap_or("");
        if let Some(dir) = aram_dir {
            let path = cache::aram_tier_file_path(dir, mode, locale);
            let _ = crate::storage::write_lz4_envelope(&path, patch, &list);
        }
        if let Ok(mut guard) = cache::get_tier_cache().write() {
            guard.insert(
                cache_key,
                cache::CachedAramEntry {
                    cached_at: Instant::now(),
                    patch: patch.to_string(),
                    payload: list.clone(),
                },
            );
        }
    }

    crate::logging::record_data_resolution(kind, "network", started.elapsed().as_millis(), "");
    Ok(list)
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
        mode,
        ddragon,
        aram_rest_url,
        mayhem_web_url,
        locale,
        aram_dir,
        known_latest.as_deref(),
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
    let locale = crate::lcu_injector::get_active_locale();
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
