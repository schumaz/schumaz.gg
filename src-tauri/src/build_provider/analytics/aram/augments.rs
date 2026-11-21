use super::super::client::peek_latest_cached_opgg_version;
use super::BROWSER_USER_AGENT;
use super::cache::{
    ARAM_CACHE_TTL, aram_augments_file_path, aram_mayhem_augments_web_base, get_augments_cache,
    rehydrate_aram_augments,
};
use super::html_extract::normalize_opgg_champion_key;
use super::parsers::parse_aram_mayhem_augments_html;
use crate::build_provider::models::DataDragonCache;
use crate::models::{AppLocale, AramAugmentEntry};
use std::path::Path;
use std::time::Instant;

pub async fn fetch_aram_mayhem_augments_with_base_locale_and_dir(
    client: &reqwest::Client,
    champion_key: &str,
    web_base: &str,
    locale: AppLocale,
    ddragon: &DataDragonCache,
    aram_dir: Option<&Path>,
) -> Result<Vec<AramAugmentEntry>, String> {
    let started = Instant::now();
    let normalized = normalize_opgg_champion_key(champion_key);
    if normalized.is_empty() {
        return Err("Champion key cannot be empty".to_string());
    }

    let cache_key = format!("{}:{}:{}", locale.as_str(), web_base, normalized);
    if let Ok(guard) = get_augments_cache().read()
        && let Some((ts, cached)) = guard.get(&cache_key)
        && ts.elapsed() < ARAM_CACHE_TTL
    {
        let mut list = cached.clone();
        rehydrate_aram_augments(&mut list, ddragon);
        if !list.is_empty() {
            crate::logging::record_data_resolution(
                "aram_augments",
                "ram_hit",
                started.elapsed().as_millis(),
                "",
            );
            return Ok(list);
        }
    }

    let disk_env = aram_dir.and_then(|dir| {
        let path = aram_augments_file_path(dir, locale, &normalized);
        crate::storage::read_lz4_envelope::<Vec<AramAugmentEntry>>(&path)
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
            rehydrate_aram_augments(&mut list, ddragon);
            if !list.is_empty() {
                if let Ok(mut guard) = get_augments_cache().write() {
                    guard.insert(cache_key.clone(), (Instant::now(), list.clone()));
                }
                crate::logging::record_data_resolution(
                    "aram_augments",
                    "disk_hit",
                    started.elapsed().as_millis(),
                    "",
                );
                return Ok(list);
            }
        }
    }

    let url = format!("{}/{}/augments", web_base.trim_end_matches('/'), normalized);
    let fetch_res: Result<Vec<AramAugmentEntry>, String> = async {
        let t0 = Instant::now();
        let res = client
            .get(&url)
            .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT)
            .send()
            .await
            .map_err(|e| {
                crate::logging::record_http_network_error_log(
                    "analytics",
                    t0.elapsed().as_millis(),
                    &e.to_string(),
                );
                format!("Failed to fetch ARAM Mayhem augments: {}", e)
            })?;

        crate::logging::record_http_status_log(
            "analytics",
            res.status().as_u16(),
            t0.elapsed().as_millis(),
        );
        if !res.status().is_success() {
            return Err(format!("ARAM Mayhem augments HTTP error: {}", res.status()));
        }

        let html = res
            .text()
            .await
            .map_err(|e| format!("Failed to read ARAM Mayhem augments HTML: {}", e))?;
        parse_aram_mayhem_augments_html(&html, ddragon)
    }
    .await;

    let augments = match fetch_res {
        Ok(list) => list,
        Err(err) => {
            // Stale-While-Error offline fallback
            if let Some(env) = disk_env
                && !env.payload.is_empty()
            {
                let mut rescued = env.payload;
                rehydrate_aram_augments(&mut rescued, ddragon);
                if !rescued.is_empty() {
                    crate::logging::record_data_resolution(
                        "aram_augments",
                        "stale_fallback",
                        started.elapsed().as_millis(),
                        "",
                    );
                    return Ok(rescued);
                }
            }
            return Err(err);
        }
    };

    if !augments.is_empty() {
        if let Some(dir) = aram_dir {
            let path = aram_augments_file_path(dir, locale, &normalized);
            let patch = known_latest.as_deref().unwrap_or("");
            let _ = crate::storage::write_lz4_envelope(&path, patch, &augments);
        }
        if let Ok(mut guard) = get_augments_cache().write() {
            guard.insert(cache_key, (Instant::now(), augments.clone()));
        }
    }

    crate::logging::record_data_resolution(
        "aram_augments",
        "network",
        started.elapsed().as_millis(),
        "",
    );
    Ok(augments)
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
        crate::lcu_injector::get_active_locale(),
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
    let locale = crate::lcu_injector::get_active_locale();
    fetch_aram_mayhem_augments_for_locale(client, champion_key, locale, ddragon).await
}
