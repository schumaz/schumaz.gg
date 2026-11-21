pub mod augments;
mod cache;
pub mod html_extract;
mod parsers;

pub use augments::*;
pub use cache::*;
pub use html_extract::*;
pub use parsers::*;

use super::client::peek_latest_cached_opgg_version;
use crate::build_provider::models::DataDragonCache;
use crate::models::{AppLocale, AramChampionRank, AramMode};
use std::path::Path;
use std::time::Instant;

pub const OPGG_ARAM_REST_URL: &str = "https://lol-api-champion.op.gg/api/global/champions/aram";
pub const OPGG_ARAM_MAYHEM_WEB_URL: &str = "https://op.gg/lol/modes/aram-mayhem";
pub const OPGG_ARAM_MAYHEM_AUGMENTS_WEB_BASE: &str = "https://op.gg/pt/lol/modes/aram-mayhem";

pub(crate) const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";

pub async fn fetch_aram_tier_list_with_urls_and_dir(
    client: &reqwest::Client,
    mode: AramMode,
    ddragon: &DataDragonCache,
    aram_rest_url: &str,
    mayhem_web_url: &str,
    locale: AppLocale,
    aram_dir: Option<&Path>,
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

    if let Ok(guard) = cache::get_tier_cache().read()
        && let Some((ts, cached)) = guard.get(&cache_key)
        && ts.elapsed() < cache::ARAM_CACHE_TTL
    {
        let mut list = cached.clone();
        cache::rehydrate_aram_names(&mut list, ddragon);
        crate::logging::record_data_resolution(kind, "ram_hit", started.elapsed().as_millis(), "");
        return Ok(list);
    }

    let disk_env = aram_dir.and_then(|dir| {
        let path = cache::aram_tier_file_path(dir, mode, locale);
        crate::storage::read_lz4_envelope::<Vec<AramChampionRank>>(&path)
    });
    let known_latest = peek_latest_cached_opgg_version("global");

    if let Some(ref env) = disk_env
        && !env.payload.is_empty()
        && crate::storage::is_envelope_fresh(
            env.created_at_unix,
            crate::storage::OPGG_CACHE_TTL_SECS,
            crate::storage::now_unix_secs(),
        )
    {
        let patch_ok = match known_latest.as_deref() {
            Some(latest) if !env.patch.is_empty() => env.patch == latest,
            _ => true,
        };
        if patch_ok {
            let mut list = env.payload.clone();
            cache::rehydrate_aram_names(&mut list, ddragon);
            if let Ok(mut guard) = cache::get_tier_cache().write() {
                guard.insert(cache_key.clone(), (Instant::now(), list.clone()));
            }
            crate::logging::record_data_resolution(
                kind,
                "disk_hit",
                started.elapsed().as_millis(),
                "",
            );
            return Ok(list);
        }
    }

    let fetch_res: Result<Vec<AramChampionRank>, String> = match mode {
        AramMode::Aram => {
            async {
                let t0 = Instant::now();
                let res = client.get(aram_rest_url).send().await.map_err(|e| {
                    crate::logging::record_http_network_error_log(
                        "analytics",
                        t0.elapsed().as_millis(),
                        &e.to_string(),
                    );
                    format!("Failed to fetch ARAM tier list: {}", e)
                })?;
                crate::logging::record_http_status_log(
                    "analytics",
                    res.status().as_u16(),
                    t0.elapsed().as_millis(),
                );
                if !res.status().is_success() {
                    return Err(format!("ARAM tier list HTTP error: {}", res.status()));
                }
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
                let t0 = Instant::now();
                let res = client
                    .get(mayhem_web_url)
                    .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT)
                    .send()
                    .await
                    .map_err(|e| {
                        crate::logging::record_http_network_error_log(
                            "analytics",
                            t0.elapsed().as_millis(),
                            &e.to_string(),
                        );
                        format!("Failed to fetch ARAM Mayhem tier list: {}", e)
                    })?;
                crate::logging::record_http_status_log(
                    "analytics",
                    res.status().as_u16(),
                    t0.elapsed().as_millis(),
                );
                if !res.status().is_success() {
                    return Err(format!(
                        "ARAM Mayhem tier list HTTP error: {}",
                        res.status()
                    ));
                }
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
        if let Some(dir) = aram_dir {
            let path = cache::aram_tier_file_path(dir, mode, locale);
            let patch = known_latest.as_deref().unwrap_or("");
            let _ = crate::storage::write_lz4_envelope(&path, patch, &list);
        }
        if let Ok(mut guard) = cache::get_tier_cache().write() {
            guard.insert(cache_key, (Instant::now(), list.clone()));
        }
    }

    crate::logging::record_data_resolution(kind, "network", started.elapsed().as_millis(), "");
    Ok(list)
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
