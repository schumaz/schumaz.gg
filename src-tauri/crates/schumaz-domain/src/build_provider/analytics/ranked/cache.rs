use super::super::client::{OPGG_REST_API_BASE, peek_latest_cached_opgg_version};
use super::super::cooldown::send_analytics_request;
use crate::build_provider::models::DataDragonCache;
use crate::models::{AppLocale, RankedTierListResponse};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

pub const RANKED_CACHE_TTL: Duration = Duration::from_secs(900);

pub type RankedCacheMap = HashMap<String, (Instant, RankedTierListResponse)>;

static RANKED_TIER_CACHE: OnceLock<RwLock<RankedCacheMap>> = OnceLock::new();

pub(crate) fn get_ranked_cache() -> &'static RwLock<RankedCacheMap> {
    RANKED_TIER_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub(super) async fn fetch_ranked_json(
    client: &reqwest::Client,
    rest_base: &str,
    region: &str,
    tier: &str,
    version: Option<&str>,
) -> Result<serde_json::Value, String> {
    let mut url = format!(
        "{}/{}/champions/ranked?tier={}",
        rest_base.trim_end_matches('/'),
        region,
        tier
    );
    if let Some(v) = version
        && !v.trim().is_empty()
    {
        url.push_str(&format!("&version={}", v.trim()));
    }

    let res = send_analytics_request(&url, client.get(&url), "Ranked tier list").await?;
    res.json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse ranked tier list JSON: {}", e))
}

pub fn ranked_tier_list_file_path(
    dir: &Path,
    region: &str,
    tier: &str,
    manual_patch: Option<&str>,
    patch_48h_protection: bool,
    locale: AppLocale,
) -> PathBuf {
    let reg = crate::storage::sanitize_key_component(region);
    let tr = crate::storage::sanitize_key_component(tier);
    let pat = crate::storage::sanitize_key_component(manual_patch.unwrap_or("auto"));
    let prot = if patch_48h_protection { "p1" } else { "p0" };
    dir.join(format!(
        "tier_{reg}_{tr}_{pat}_{prot}_{}.lz4",
        locale.as_str()
    ))
}

pub(crate) fn rehydrate_ranked_names(
    response: &mut RankedTierListResponse,
    ddragon: &DataDragonCache,
) {
    for champ in &mut response.champions {
        if let Some(meta) = ddragon.champions.get(&champ.id) {
            champ.name = meta.display_name.clone();
            if !meta.icon_url.is_empty() {
                champ.icon_url = meta.icon_url.clone();
            }
        }
    }
}

/// Pre-warms a valid (< 48h) Ranked Tier List from `.lz4` disk into RAM on startup.
pub fn prewarm_ranked_tier_list_from_dir(
    dir: &Path,
    region: &str,
    tier: &str,
    manual_patch: Option<&str>,
    patch_48h_protection: bool,
    locale: AppLocale,
    ddragon: &DataDragonCache,
) -> bool {
    let path = ranked_tier_list_file_path(
        dir,
        region,
        tier,
        manual_patch,
        patch_48h_protection,
        locale,
    );
    let Some(mut env) = crate::storage::read_lz4_envelope::<RankedTierListResponse>(&path) else {
        return false;
    };
    let now = crate::storage::now_unix_secs();
    if !crate::storage::is_envelope_fresh(
        env.created_at_unix,
        crate::storage::OPGG_CACHE_TTL_SECS,
        now,
    ) {
        return false;
    }
    if manual_patch.is_none()
        && let Some(latest) = peek_latest_cached_opgg_version(region)
        && !env.payload.patch_fallback_applied
        && env.patch != latest
    {
        return false;
    }

    rehydrate_ranked_names(&mut env.payload, ddragon);
    let cache_key = format!(
        "{}:{}:{}:{}:{}:{}",
        OPGG_REST_API_BASE,
        region,
        tier,
        manual_patch.unwrap_or(""),
        patch_48h_protection,
        locale.as_str()
    );
    if let Ok(mut guard) = get_ranked_cache().write() {
        guard.insert(cache_key, (Instant::now(), env.payload));
        return true;
    }
    false
}
