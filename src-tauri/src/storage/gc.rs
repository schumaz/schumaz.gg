use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::lz4_cache::{CACHE_SCHEMA_VERSION, read_lz4_envelope_header};
use super::paths::AppPaths;

pub const GC_MAX_INACTIVE_SECS: u64 = 7 * 24 * 3600;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GcStats {
    pub removed_tmp_files: usize,
    pub removed_stale_patch_files: usize,
    pub removed_inactive_files: usize,
    pub removed_corrupt_files: usize,
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
/// - `.lz4` files under `Cache\opgg\` whose envelope `patch` is non-empty and not in `active_patches`
/// - Corrupt or outdated-schema `.lz4` files
/// - `.lz4` or cached asset files whose `mtime` has been inactive for > 7 days
pub fn run_garbage_collector(
    paths: &AppPaths,
    active_patches: &[String],
    now_unix: u64,
) -> GcStats {
    let mut stats = GcStats::default();
    let mut all_files = Vec::new();
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

        if is_lz4 {
            match read_lz4_envelope_header(&path) {
                Some(header) => {
                    if header.schema_version != CACHE_SCHEMA_VERSION {
                        if fs::remove_file(&path).is_ok() {
                            stats.removed_corrupt_files += 1;
                        }
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

        if let Ok(meta) = fs::metadata(&path)
            && let Ok(modified) = meta.modified()
            && let Ok(dur) = modified.duration_since(UNIX_EPOCH)
        {
            let mtime_unix = dur.as_secs();
            if now_unix.saturating_sub(mtime_unix) > GC_MAX_INACTIVE_SECS
                && fs::remove_file(&path).is_ok()
            {
                stats.removed_inactive_files += 1;
            }
        }
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::lz4_cache::write_lz4_envelope_at;
    use crate::storage::paths::{atomic_write, next_test_seq, now_unix_secs, touch_mtime};
    use std::time::SystemTime;

    fn temp_test_paths(suffix: &str) -> AppPaths {
        let dir = std::env::temp_dir().join(format!(
            "schumazgg_gc_test_{}_{}_{}",
            std::process::id(),
            suffix,
            next_test_seq()
        ));
        let _ = fs::remove_dir_all(&dir);
        let paths = AppPaths::from_root(dir);
        paths.ensure_dirs().unwrap();
        paths
    }

    #[test]
    fn test_garbage_collector_cleans_tmp_old_patches_corrupt_and_inactive_files() {
        let paths = temp_test_paths("gc");
        let now = now_unix_secs();

        // 1. Orphan .tmp files
        let tmp_cfg = paths.config_dir.join("UserSettings.ini.123.tmp");
        let tmp_cache = paths.opgg_builds_dir.join("build.lz4.456.tmp");
        atomic_write(&tmp_cfg, b"tmp").unwrap();
        atomic_write(&tmp_cache, b"tmp").unwrap();

        // 2. Old patch build file (patch 16.19 not in active ["16.22", "16.21"])
        let old_patch_file = paths.opgg_builds_dir.join("old_patch.lz4");
        write_lz4_envelope_at(&old_patch_file, "16.19", &vec![1, 2], now).unwrap();

        // 3. Active patch build file (should stay)
        let active_patch_file = paths.opgg_builds_dir.join("active_patch.lz4");
        write_lz4_envelope_at(&active_patch_file, "16.22", &vec![3, 4], now).unwrap();

        // 4. Corrupt .lz4
        let corrupt_file = paths.profile_dir.join("corrupt.lz4");
        atomic_write(&corrupt_file, b"broken").unwrap();

        // 5. Inactive asset > 7 days (mtime backdated 8 days)
        let old_asset = paths.assets_dir.join("old_splash.jpg");
        atomic_write(&old_asset, b"jpegdata").unwrap();
        {
            let file = fs::OpenOptions::new().write(true).open(&old_asset).unwrap();
            let eight_days_ago = SystemTime::now() - std::time::Duration::from_secs(8 * 24 * 3600);
            file.set_modified(eight_days_ago).unwrap();
        }

        // 6. Recently hit asset (touch_mtime keeps it alive)
        let fresh_asset = paths.assets_dir.join("fresh_splash.jpg");
        atomic_write(&fresh_asset, b"jpegdata").unwrap();
        {
            let file = fs::OpenOptions::new()
                .write(true)
                .open(&fresh_asset)
                .unwrap();
            let eight_days_ago = SystemTime::now() - std::time::Duration::from_secs(8 * 24 * 3600);
            file.set_modified(eight_days_ago).unwrap();
        }
        touch_mtime(&fresh_asset);

        let stats = run_garbage_collector(&paths, &["16.22".to_string(), "16.21".to_string()], now);

        assert_eq!(stats.removed_tmp_files, 2);
        assert_eq!(stats.removed_stale_patch_files, 1);
        assert_eq!(stats.removed_corrupt_files, 1);
        assert_eq!(stats.removed_inactive_files, 1);
        assert!(!tmp_cfg.exists());
        assert!(!tmp_cache.exists());
        assert!(!old_patch_file.exists());
        assert!(active_patch_file.exists());
        assert!(!corrupt_file.exists());
        assert!(!old_asset.exists());
        assert!(fresh_asset.exists());

        let _ = fs::remove_dir_all(&paths.root);
    }
}
