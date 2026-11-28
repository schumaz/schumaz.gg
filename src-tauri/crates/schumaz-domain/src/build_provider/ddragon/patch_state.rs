use super::super::analytics::versions::derive_patch_major_minor;
use crate::models::AppLocale;
use std::collections::HashSet;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

/// Interval to re-verify `/api/versions.json` during long-running tray sessions
/// so DataDragon caches expire automatically when a new Riot patch launches.
pub(super) const DDRAGON_PATCH_RECHECK_INTERVAL: Duration =
    Duration::from_secs(crate::storage::VERSIONS_DISK_TTL_SECS);

#[derive(Default)]
struct DdragonPatchState {
    latest_known_version: Option<String>,
    last_version_check_at: Option<Instant>,
    bg_in_flight_locales: HashSet<AppLocale>,
}

static DDRAGON_PATCH_STATE: OnceLock<RwLock<DdragonPatchState>> = OnceLock::new();

fn patch_state_store() -> &'static RwLock<DdragonPatchState> {
    DDRAGON_PATCH_STATE.get_or_init(|| RwLock::new(DdragonPatchState::default()))
}

pub(super) fn clear_patch_state() {
    if let Ok(mut guard) = patch_state_store().write() {
        guard.latest_known_version = None;
        guard.last_version_check_at = None;
        guard.bg_in_flight_locales.clear();
    }
}

pub(super) fn latest_known_ddragon_version() -> Option<String> {
    patch_state_store()
        .read()
        .ok()
        .and_then(|g| g.latest_known_version.clone())
}

pub(super) fn last_version_check_elapsed() -> Option<Duration> {
    patch_state_store()
        .read()
        .ok()
        .and_then(|g| g.last_version_check_at.map(|ts| ts.elapsed()))
}

pub(super) fn record_latest_known_version(version: &str) {
    if let Ok(mut guard) = patch_state_store().write() {
        guard.latest_known_version = Some(version.to_string());
        guard.last_version_check_at = Some(Instant::now());
    }
}

pub(super) fn claim_bg_patch_check(locale: AppLocale) -> bool {
    patch_state_store()
        .write()
        .map(|mut g| g.bg_in_flight_locales.insert(locale))
        .unwrap_or(false)
}

pub(super) fn release_bg_patch_check(locale: AppLocale) {
    if let Ok(mut g) = patch_state_store().write() {
        g.bg_in_flight_locales.remove(&locale);
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
