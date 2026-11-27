use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use sysinfo::System;

use super::*;

fn temp_test_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "schumazgg_discovery_live_{tag}_{}_{nanos}",
        std::process::id()
    ));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn test_process_matchers_never_match_game_process() {
    assert!(is_client_ux_process_name(OsStr::new(CLIENT_PROCESS_NAME)));
    assert!(!is_client_ux_process_name(OsStr::new(
        LOCKFILE_OWNER_PROCESS_NAME
    )));
    assert!(!is_client_ux_process_name(OsStr::new(
        "League of Legends.exe"
    )));
    assert!(!is_client_ux_process_name(OsStr::new(
        "League of Legends"
    )));

    assert!(is_valid_lockfile_pid_process(OsStr::new(
        LOCKFILE_OWNER_PROCESS_NAME
    )));
    assert!(is_valid_lockfile_pid_process(OsStr::new(
        CLIENT_PROCESS_NAME
    )));
    assert!(!is_valid_lockfile_pid_process(OsStr::new(
        "League of Legends.exe"
    )));
    assert!(!is_valid_lockfile_pid_process(OsStr::new(
        "League of Legends"
    )));
    assert!(!is_valid_lockfile_pid_process(OsStr::new("notepad.exe")));
}

#[test]
fn test_try_discover_from_lockfile_rejects_dead_and_foreign_pids() {
    let root = temp_test_dir("stale_pid");
    let lf_path = root.join("lockfile");
    let mut system = System::new();

    // Dead / nonexistent PID
    fs::write(&lf_path, "LeagueClient:999999999:54321:secretTok:https").unwrap();
    assert!(try_discover_from_lockfile_path(&mut system, &lf_path).is_none());

    // Live PID belonging to this test process (must be rejected because it is not LeagueClient)
    let self_pid = std::process::id();
    fs::write(
        &lf_path,
        format!("LeagueClient:{self_pid}:54321:secretTok:https"),
    )
    .unwrap();
    assert!(try_discover_from_lockfile_path(&mut system, &lf_path).is_none());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_lcu_tls_client_config_loads_embedded_riot_ca() {
    let config = lcu_tls_client_config().expect("Embedded Riot Games CA PEM should load cleanly");
    assert!(Arc::strong_count(&config) >= 1);
}
