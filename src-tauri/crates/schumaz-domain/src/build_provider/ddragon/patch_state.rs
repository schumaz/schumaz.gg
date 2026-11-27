use super::super::analytics::versions::derive_patch_major_minor;
use crate::models::AppLocale;
use std::collections::HashSet;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

/// Interval to re-verify `/api/versions.json` during long-running tray sessions
/// so DataDragon caches expire automatically when a new Riot patch launches.
pub(super) const DDRAGON_PATCH_RECHECK_INTERVAL: Duration =
    Duration::from_secs(crate::storage::VERSIONS_DISK_TTL_SECS);

static LATEST_KNOWN_DDRAGON_VERSION: OnceLock<RwLock<Option<String>>> = OnceLock::new();
static LAST_VERSION_CHECK_AT: OnceLock<RwLock<Option<Instant>>> = OnceLock::new();
static DDRAGON_BG_IN_FLIGHT: OnceLock<RwLock<HashSet<AppLocale>>> = OnceLock::new();

fn latest_known_version_store() -> &'static RwLock<Option<String>> {
    LATEST_KNOWN_DDRAGON_VERSION.get_or_init(|| RwLock::new(None))
}

fn last_version_check_store() -> &'static RwLock<Option<Instant>> {
    LAST_VERSION_CHECK_AT.get_or_init(|| RwLock::new(None))
}

fn bg_in_flight_locales() -> &'static RwLock<HashSet<AppLocale>> {
    DDRAGON_BG_IN_FLIGHT.get_or_init(|| RwLock::new(HashSet::new()))
}

pub(super) fn latest_known_ddragon_version() -> Option<String> {
    latest_known_version_store()
        .read()
        .ok()
        .and_then(|g| g.clone())
}

pub(super) fn last_version_check_elapsed() -> Option<Duration> {
    last_version_check_store()
        .read()
        .ok()
        .and_then(|g| g.map(|ts| ts.elapsed()))
}

pub(super) fn record_latest_known_version(version: &str) {
    if let Ok(mut guard) = latest_known_version_store().write() {
        *guard = Some(version.to_string());
    }
    if let Ok(mut guard) = last_version_check_store().write() {
        *guard = Some(Instant::now());
    }
}

pub(super) fn claim_bg_patch_check(locale: AppLocale) -> bool {
    bg_in_flight_locales()
        .write()
        .map(|mut set| set.insert(locale))
        .unwrap_or(false)
}

pub(super) fn release_bg_patch_check(locale: AppLocale) {
    if let Ok(mut set) = bg_in_flight_locales().write() {
        set.remove(&locale);
    }
}

/// Returns `true` when `cached_version` matches `latest_known_version` (or when no live version is known yet).
pub(super) fn is_ddragon_version_current(
    cached_version: &str,
    latest_known_version: Option<&str>,
) -> bool {
    let Some(latest) = latest_known_version
        .map(str::trim)
        .filter(|v| !v.is_empty())
    else {
        return true;
    };
    cached_version.trim() == latest
}

/// Returns `true` when a background `/api/versions.json` verification should run:
/// - Immediately when the latest known OP.GG patch (`major.minor`) differs from `cached_version`, or
/// - When no live check has occurred yet or `DDRAGON_PATCH_RECHECK_INTERVAL` has elapsed.
pub(super) fn should_recheck_current_patch(
    cached_version: &str,
    last_check_elapsed: Option<Duration>,
    latest_opgg_patch: Option<&str>,
) -> bool {
    if let Some(opgg_patch) = latest_opgg_patch.map(str::trim).filter(|p| !p.is_empty())
        && let (Some(cached_mm), Some(opgg_mm)) = (
            derive_patch_major_minor(cached_version),
            derive_patch_major_minor(opgg_patch),
        )
        && cached_mm != opgg_mm
    {
        return true;
    }
    match last_check_elapsed {
        None => true,
        Some(elapsed) => elapsed >= DDRAGON_PATCH_RECHECK_INTERVAL,
    }
}
