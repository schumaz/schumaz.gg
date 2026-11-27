use super::*;
use std::sync::atomic::AtomicUsize;

#[test]
fn test_app_paths_centralizes_config_cache_logs_and_webview_under_root() {
    let root = std::env::temp_dir().join(format!(
        "schumazgg_paths_test_{}_{}",
        std::process::id(),
        next_test_seq()
    ));
    let _ = fs::remove_dir_all(&root);

    let paths = AppPaths::from_root(root.clone());
    assert_eq!(paths.root, root);
    assert_eq!(paths.config_dir, root.join("Config"));
    assert_eq!(paths.cache_dir, root.join("Cache"));
    assert_eq!(paths.logs_dir, root.join("Logs"));
    assert_eq!(paths.webview_dir, root.join("WebView"));
    assert_eq!(paths.ddragon_dir, root.join("Cache").join("ddragon"));
    assert_eq!(paths.opgg_dir, root.join("Cache").join("opgg"));
    assert_eq!(
        paths.opgg_arena_dir,
        root.join("Cache").join("opgg").join("arena")
    );
    assert_eq!(paths.profile_dir, root.join("Cache").join("profile"));
    assert_eq!(paths.assets_dir, root.join("Cache").join("assets"));
    assert_eq!(
        paths.user_settings_ini_path(),
        root.join("Config").join("UserSettings.ini")
    );
    assert_eq!(
        paths.log_file_path(),
        root.join("Logs").join("schumazgg.log")
    );
    assert_eq!(
        paths.lcu_lockfile_cache_path(),
        root.join("Config").join("lcu_lockfile_path.txt")
    );
    assert!(!paths.logs_dir.starts_with(&paths.cache_dir));

    paths.ensure_dirs().unwrap();
    assert!(paths.config_dir.is_dir());
    assert!(paths.cache_dir.is_dir());
    assert!(paths.logs_dir.is_dir());
    assert!(paths.webview_dir.is_dir());
    assert!(paths.ddragon_dir.is_dir());
    assert!(paths.opgg_dir.is_dir());
    assert!(paths.opgg_arena_dir.is_dir());
    assert!(paths.profile_dir.is_dir());
    assert!(paths.assets_dir.is_dir());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_lazy_touch_mtime_only_updates_when_older_than_12h() {
    let dir = std::env::temp_dir().join(format!(
        "schumazgg_touch_test_{}_{}",
        std::process::id(),
        next_test_seq()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file_path = dir.join("asset.png");
    atomic_write(&file_path, b"png").unwrap();

    // Backdate by 1h (< 12h): touch_mtime must skip NTFS write.
    let one_hour_ago = SystemTime::now() - Duration::from_secs(3600);
    force_touch_mtime_at(&file_path, one_hour_ago);
    let mtime_before = fs::metadata(&file_path).unwrap().modified().unwrap();
    touch_mtime(&file_path);
    let mtime_after = fs::metadata(&file_path).unwrap().modified().unwrap();
    assert_eq!(mtime_before, mtime_after);

    // Backdate by 13h (>= 12h): touch_mtime must refresh mtime to ~now.
    let thirteen_hours_ago = SystemTime::now() - Duration::from_secs(13 * 3600);
    force_touch_mtime_at(&file_path, thirteen_hours_ago);
    touch_mtime(&file_path);
    let refreshed = fs::metadata(&file_path).unwrap().modified().unwrap();
    let age_secs = SystemTime::now()
        .duration_since(refreshed)
        .unwrap_or_default()
        .as_secs();
    assert!(age_secs < 60, "expected refreshed mtime, age={age_secs}s");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_rename_with_retry_recovers_on_windows_sharing_and_access_errors() {
    let src = Path::new("tmp_src");
    let dst = Path::new("tmp_dst");

    let attempts = AtomicUsize::new(0);
    let ok = rename_with_retry(src, dst, |_, _| {
        let n = attempts.fetch_add(1, Ordering::Relaxed);
        match n {
            0 => Err(std::io::Error::from_raw_os_error(5)),
            1 => Err(std::io::Error::from_raw_os_error(32)),
            _ => Ok(()),
        }
    });
    assert!(ok.is_ok());
    assert_eq!(attempts.load(Ordering::Relaxed), 3);

    // Non-transient error must fail immediately without retrying.
    let non_retry_attempts = AtomicUsize::new(0);
    let err = rename_with_retry(src, dst, |_, _| {
        non_retry_attempts.fetch_add(1, Ordering::Relaxed);
        Err(std::io::Error::from_raw_os_error(2))
    });
    assert!(err.is_err());
    assert_eq!(non_retry_attempts.load(Ordering::Relaxed), 1);
}
