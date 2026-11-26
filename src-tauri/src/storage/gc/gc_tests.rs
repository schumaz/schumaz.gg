use super::*;
use crate::storage::lz4_cache::write_lz4_envelope_at;
use crate::storage::paths::{atomic_write, next_test_seq, now_unix_secs, touch_mtime};
use std::time::{Duration, SystemTime};

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

fn set_mtime_secs_ago(path: &Path, secs_ago: u64) {
    let file = fs::OpenOptions::new().write(true).open(path).unwrap();
    let ts = SystemTime::now() - Duration::from_secs(secs_ago);
    file.set_modified(ts).unwrap();
}

#[test]
fn test_garbage_collector_cleans_tmp_old_patches_corrupt_and_inactive_files() {
    let paths = temp_test_paths("gc");
    let now = now_unix_secs();

    // 1. Orphan .tmp files (> 5m old are removed; recent in-flight .tmp files stay untouched)
    let tmp_cfg = paths.config_dir.join("UserSettings.ini.123.tmp");
    let tmp_cache = paths.opgg_builds_dir.join("build.lz4.456.tmp");
    let recent_tmp = paths.opgg_builds_dir.join("active_write.lz4.789.tmp");
    atomic_write(&tmp_cfg, b"tmp").unwrap();
    atomic_write(&tmp_cache, b"tmp").unwrap();
    atomic_write(&recent_tmp, b"in-flight").unwrap();
    set_mtime_secs_ago(&tmp_cfg, GC_TMP_MIN_AGE_SECS + 60);
    set_mtime_secs_ago(&tmp_cache, GC_TMP_MIN_AGE_SECS + 60);

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
    set_mtime_secs_ago(&old_asset, 8 * 24 * 3600);

    // 6. Recently hit asset (touch_mtime keeps it alive)
    let fresh_asset = paths.assets_dir.join("fresh_splash.jpg");
    atomic_write(&fresh_asset, b"jpegdata").unwrap();
    set_mtime_secs_ago(&fresh_asset, 8 * 24 * 3600);
    touch_mtime(&fresh_asset);

    let stats = run_garbage_collector(
        &paths,
        &["16.22".to_string(), "16.21".to_string()],
        Some("16.22.1"),
        now,
    );

    assert_eq!(stats.removed_tmp_files, 2);
    assert_eq!(stats.removed_stale_patch_files, 1);
    assert_eq!(stats.removed_corrupt_files, 1);
    assert_eq!(stats.removed_inactive_files, 1);
    assert_eq!(stats.removed_over_quota_files, 0);
    assert!(!tmp_cfg.exists());
    assert!(!tmp_cache.exists());
    assert!(recent_tmp.exists());
    assert!(!old_patch_file.exists());
    assert!(active_patch_file.exists());
    assert!(!corrupt_file.exists());
    assert!(!old_asset.exists());
    assert!(fresh_asset.exists());

    let _ = fs::remove_dir_all(&paths.root);
}

#[test]
fn test_garbage_collector_expires_ddragon_cache_by_current_patch_not_inactivity() {
    let paths = temp_test_paths("ddragon_patch_gc");
    let now = now_unix_secs();

    // Current-patch DataDragon cache untouched for 14 days must stay alive until the patch changes.
    let current_meta = paths.ddragon_dir.join("metadata_pt_BR.lz4");
    write_lz4_envelope_at(&current_meta, "16.22.1", &"pt_meta", now).unwrap();
    set_mtime_secs_ago(&current_meta, 14 * 24 * 3600);

    let current_skins = paths.ddragon_dir.join("skins_pt_BR.lz4");
    write_lz4_envelope_at(&current_skins, "16.22.1", &"pt_skins", now).unwrap();
    set_mtime_secs_ago(&current_skins, 14 * 24 * 3600);

    // Previous-patch (or older micropatch) DataDragon caches expire immediately even if fresh.
    let stale_meta = paths.ddragon_dir.join("metadata_en_US.lz4");
    write_lz4_envelope_at(&stale_meta, "16.21.1", &"en_meta", now).unwrap();

    let stale_micropatch = paths.ddragon_dir.join("skins_en_US.lz4");
    write_lz4_envelope_at(&stale_micropatch, "16.22.0", &"en_skins", now).unwrap();

    let stats = run_garbage_collector(
        &paths,
        &["16.22".to_string(), "16.21".to_string()],
        Some("16.22.1"),
        now,
    );

    assert_eq!(stats.removed_stale_patch_files, 2);
    assert_eq!(stats.removed_inactive_files, 0);
    assert!(current_meta.exists());
    assert!(current_skins.exists());
    assert!(!stale_meta.exists());
    assert!(!stale_micropatch.exists());

    // Fallback using active_patches major.minor when live DataDragon version is unavailable:
    // keeps 16.22.1 while evicting 16.21.1, and keeps all files when offline (empty).
    let old_fallback = paths.ddragon_dir.join("metadata_en_US.lz4");
    write_lz4_envelope_at(&old_fallback, "16.21.1", &"old_en", now).unwrap();
    let offline_stats = run_garbage_collector(&paths, &[], None, now);
    assert_eq!(offline_stats.removed_stale_patch_files, 0);
    assert!(old_fallback.exists());

    let fallback_stats = run_garbage_collector(&paths, &["16.22".to_string()], None, now);
    assert_eq!(fallback_stats.removed_stale_patch_files, 1);
    assert!(current_meta.exists());
    assert!(!old_fallback.exists());

    let _ = fs::remove_dir_all(&paths.root);
}

#[test]
fn test_garbage_collector_evicts_oldest_assets_lru_when_over_quota() {
    let paths = temp_test_paths("lru_quota");
    let now = now_unix_secs();

    let oldest = paths.assets_dir.join("oldest.png");
    let middle = paths.assets_dir.join("middle.png");
    let newest = paths.assets_dir.join("newest.png");

    atomic_write(&oldest, &[1u8; 60]).unwrap();
    atomic_write(&middle, &[2u8; 60]).unwrap();
    atomic_write(&newest, &[3u8; 60]).unwrap();

    set_mtime_secs_ago(&oldest, 300);
    set_mtime_secs_ago(&middle, 200);
    set_mtime_secs_ago(&newest, 100);

    // Total = 180 bytes; quota = 120 bytes -> must evict oldest (60 bytes), leaving middle + newest (120 bytes).
    let stats = run_garbage_collector_with_asset_quota(&paths, &[], None, now, 120);
    assert_eq!(stats.removed_over_quota_files, 1);
    assert!(!oldest.exists());
    assert!(middle.exists());
    assert!(newest.exists());

    let _ = fs::remove_dir_all(&paths.root);
}

#[test]
fn test_inspect_and_clear_local_cache_by_scope() {
    let paths = temp_test_paths("inspect_clear");

    let asset = paths.assets_dir.join("splash.jpg");
    let opgg = paths.opgg_builds_dir.join("ahri.lz4");
    let tmp = paths.cache_dir.join("partial.tmp");

    atomic_write(&asset, &[10u8; 50]).unwrap();
    atomic_write(&opgg, &[20u8; 30]).unwrap();
    atomic_write(&tmp, &[30u8; 99]).unwrap();

    let info = inspect_local_cache(&paths);
    assert_eq!(
        info,
        LocalCacheInfo {
            total_bytes: 80,
            assets_bytes: 50,
            data_bytes: 30,
            file_count: 2,
        }
    );

    // Clearing Builds only removes opgg/data files and keeps cached images intact.
    let after_builds = clear_local_cache_at(&paths, CacheClearScope::Builds).unwrap();
    assert_eq!(
        after_builds,
        LocalCacheInfo {
            total_bytes: 50,
            assets_bytes: 50,
            data_bytes: 0,
            file_count: 1,
        }
    );
    assert!(asset.exists());
    assert!(!opgg.exists());

    // Re-seed builds file and test Images-only scope.
    atomic_write(&opgg, &[20u8; 30]).unwrap();
    let after_images = clear_local_cache_at(&paths, CacheClearScope::Images).unwrap();
    assert_eq!(
        after_images,
        LocalCacheInfo {
            total_bytes: 30,
            assets_bytes: 0,
            data_bytes: 30,
            file_count: 1,
        }
    );
    assert!(!asset.exists());
    assert!(opgg.exists());

    // Re-seed asset and test All scope.
    atomic_write(&asset, &[10u8; 50]).unwrap();
    let after_all = clear_local_cache_at(&paths, CacheClearScope::All).unwrap();
    assert_eq!(after_all, LocalCacheInfo::default());
    assert!(!asset.exists());
    assert!(!opgg.exists());
    assert!(paths.assets_dir.is_dir());
    assert!(paths.opgg_builds_dir.is_dir());

    let _ = fs::remove_dir_all(&paths.root);
}
