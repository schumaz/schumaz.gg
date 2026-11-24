use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::lz4_cache::{CACHE_SCHEMA_VERSION, read_lz4_envelope_header};
use super::paths::AppPaths;

pub const GC_MAX_INACTIVE_SECS: u64 = 7 * 24 * 3600;
/// Maximum total disk footprint allowed for `Cache\assets\` (1 GiB).
pub const ASSET_CACHE_MAX_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GcStats {
    pub removed_tmp_files: usize,
    pub removed_stale_patch_files: usize,
    pub removed_inactive_files: usize,
    pub removed_corrupt_files: usize,
    pub removed_over_quota_files: usize,
}

/// Disk usage summary for `%LOCALAPPDATA%\schumazgg\Cache\`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalCacheInfo {
    pub total_bytes: u64,
    pub assets_bytes: u64,
    pub data_bytes: u64,
    pub file_count: u32,
}

/// Target subset when clearing local cache from Settings.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CacheClearScope {
    Builds,
    Images,
    All,
}

struct SurvivingAsset {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

fn patch_major_minor(version: &str) -> Option<(&str, &str)> {
    let mut parts = version.trim().split('.');
    let major = parts.next()?.trim();
    let minor = parts.next()?.trim();
    if major.parse::<u32>().is_ok() && minor.parse::<u32>().is_ok() {
        Some((major, minor))
    } else {
        None
    }
}

/// Evaluates whether a DataDragon `.lz4` envelope matches the current live patch.
/// Returns `None` when neither the live DataDragon version nor an active patch is known (offline).
fn ddragon_patch_matches_current(
    envelope_patch: &str,
    current_ddragon_version: Option<&str>,
    active_patches: &[String],
) -> Option<bool> {
    let env_patch = envelope_patch.trim();
    if let Some(current_ver) = current_ddragon_version
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        return Some(!env_patch.is_empty() && env_patch == current_ver);
    }
    if let Some(current_patch) = active_patches
        .first()
        .map(String::as_str)
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        let cur_mm = patch_major_minor(current_patch)?;
        return Some(patch_major_minor(env_patch) == Some(cur_mm));
    }
    None
}

fn collect_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

/// Scans `Cache\` (and `Config\` for orphan `.tmp` files), removing:
/// - Orphan `.tmp` files
/// - `.lz4` files under `Cache\ddragon\` whose envelope `patch` does not match the current patch
///   (never expired by 7-day inactivity while still on the current patch)
/// - `.lz4` files under `Cache\opgg\` whose envelope `patch` is non-empty and not in `active_patches`
/// - Corrupt or outdated-schema `.lz4` files
/// - Non-DataDragon `.lz4` or cached asset files whose `mtime` has been inactive for > 7 days
/// - Oldest `Cache\assets\` files (LRU by `mtime`) when total asset cache size exceeds `ASSET_CACHE_MAX_BYTES`
pub fn run_garbage_collector(
    paths: &AppPaths,
    active_patches: &[String],
    current_ddragon_version: Option<&str>,
    now_unix: u64,
) -> GcStats {
    run_garbage_collector_with_asset_quota(
        paths,
        active_patches,
        current_ddragon_version,
        now_unix,
        ASSET_CACHE_MAX_BYTES,
    )
}

/// Runs garbage collection using an explicit byte quota for `Cache\assets\`.
pub fn run_garbage_collector_with_asset_quota(
    paths: &AppPaths,
    active_patches: &[String],
    current_ddragon_version: Option<&str>,
    now_unix: u64,
    max_asset_bytes: u64,
) -> GcStats {
    let mut stats = GcStats::default();
    let mut all_files = Vec::new();
    let mut surviving_assets = Vec::new();
    collect_files_recursive(&paths.config_dir, &mut all_files);
    collect_files_recursive(&paths.cache_dir, &mut all_files);

    for path in all_files {
        let is_tmp = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("tmp"));
        if is_tmp {
            if fs::remove_file(&path).is_ok() {
                stats.removed_tmp_files += 1;
            }
            continue;
        }

        if !path.starts_with(&paths.cache_dir) {
            continue;
        }

        let is_lz4 = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("lz4"));
        let is_ddragon = path.starts_with(&paths.ddragon_dir);

        if is_lz4 {
            match read_lz4_envelope_header(&path) {
                Some(header) => {
                    if header.schema_version != CACHE_SCHEMA_VERSION {
                        if fs::remove_file(&path).is_ok() {
                            stats.removed_corrupt_files += 1;
                        }
                        continue;
                    }
                    if is_ddragon {
                        if ddragon_patch_matches_current(
                            &header.patch,
                            current_ddragon_version,
                            active_patches,
                        ) == Some(false)
                            && fs::remove_file(&path).is_ok()
                        {
                            stats.removed_stale_patch_files += 1;
                        }
                        // DataDragon cache expires strictly on patch changes, not by 7-day inactivity.
                        continue;
                    }
                    if path.starts_with(&paths.opgg_dir)
                        && !active_patches.is_empty()
                        && !header.patch.trim().is_empty()
                        && !active_patches.iter().any(|p| p == &header.patch)
                    {
                        if fs::remove_file(&path).is_ok() {
                            stats.removed_stale_patch_files += 1;
                        }
                        continue;
                    }
                }
                None => {
                    if fs::remove_file(&path).is_ok() {
                        stats.removed_corrupt_files += 1;
                    }
                    continue;
                }
            }
        }

        if let Ok(meta) = fs::metadata(&path) {
            let modified = meta.modified().unwrap_or(UNIX_EPOCH);
            if let Ok(dur) = modified.duration_since(UNIX_EPOCH) {
                let mtime_unix = dur.as_secs();
                if now_unix.saturating_sub(mtime_unix) > GC_MAX_INACTIVE_SECS {
                    if fs::remove_file(&path).is_ok() {
                        stats.removed_inactive_files += 1;
                    }
                    continue;
                }
            }

            if path.starts_with(&paths.assets_dir) {
                surviving_assets.push(SurvivingAsset {
                    path,
                    size: meta.len(),
                    modified,
                });
            }
        }
    }

    let mut total_asset_bytes: u64 = surviving_assets.iter().map(|entry| entry.size).sum();
    if total_asset_bytes > max_asset_bytes {
        surviving_assets.sort_by(|a, b| {
            a.modified
                .cmp(&b.modified)
                .then_with(|| a.path.cmp(&b.path))
        });
        for entry in surviving_assets {
            if total_asset_bytes <= max_asset_bytes {
                break;
            }
            if fs::remove_file(&entry.path).is_ok() {
                total_asset_bytes = total_asset_bytes.saturating_sub(entry.size);
                stats.removed_over_quota_files += 1;
            }
        }
    }

    stats
}

/// Computes current disk space used by cached files under `Cache\`.
pub fn inspect_local_cache(paths: &AppPaths) -> LocalCacheInfo {
    let mut files = Vec::new();
    collect_files_recursive(&paths.cache_dir, &mut files);

    let mut info = LocalCacheInfo::default();
    for path in files {
        let is_tmp = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("tmp"));
        if is_tmp {
            continue;
        }

        let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        info.total_bytes = info.total_bytes.saturating_add(size);
        if path.starts_with(&paths.assets_dir) {
            info.assets_bytes = info.assets_bytes.saturating_add(size);
        } else {
            info.data_bytes = info.data_bytes.saturating_add(size);
        }
        info.file_count = info.file_count.saturating_add(1);
    }
    info
}

/// Removes cached files under `Cache\` matching `scope` while preserving the directory structure.
pub fn clear_local_cache_at(
    paths: &AppPaths,
    scope: CacheClearScope,
) -> Result<LocalCacheInfo, String> {
    let mut files = Vec::new();
    collect_files_recursive(&paths.cache_dir, &mut files);
    for path in files {
        let should_remove = match scope {
            CacheClearScope::Builds => !path.starts_with(&paths.assets_dir),
            CacheClearScope::Images => path.starts_with(&paths.assets_dir),
            CacheClearScope::All => true,
        };
        if should_remove {
            let _ = fs::remove_file(&path);
        }
    }
    paths.ensure_dirs()?;
    Ok(inspect_local_cache(paths))
}

#[cfg(test)]
#[path = "gc/gc_tests.rs"]
mod tests;
