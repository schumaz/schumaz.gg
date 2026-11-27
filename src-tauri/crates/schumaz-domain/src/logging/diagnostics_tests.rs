use super::*;
use crate::logging::metrics::{CacheMetricsSnapshot, HttpMetricsSnapshot, RuntimeMetricsSnapshot};
use std::fs;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

static DIAG_TEST_SEQ: AtomicU64 = AtomicU64::new(1);

fn sample_input(in_memory_issues: Vec<String>) -> DiagnosticInput {
    DiagnosticInput {
        app_version: "0.4.0".to_string(),
        is_portable: false,
        os_version: "Windows 11 24H2".to_string(),
        architecture: "x86_64".to_string(),
        locale: "pt_BR".to_string(),
        gpu_saved: true,
        gpu_active: false,
        close_to_tray: true,
        lcu_connected: true,
        gameflow_phase: "ChampSelect".to_string(),
        ddragon_version: Some("16.22.1".to_string()),
        active_patch: Some("16.22".to_string()),
        metrics: RuntimeMetricsSnapshot {
            cache: CacheMetricsSnapshot {
                ram_hits: 14,
                disk_hits: 32,
                misses: 3,
                stale_fallbacks: 1,
            },
            http: HttpMetricsSnapshot {
                status_2xx: 12,
                status_4xx: 1,
                status_5xx: 0,
                network_errors: 1,
            },
            dropped_events: 0,
        },
        in_memory_issues,
    }
}

#[test]
fn test_build_diagnostic_summary_formats_and_sanitizes_without_forbidden_brand_terms() {
    let input = sample_input(vec![
        "2026-11-21T16:20:05.123Z WARN [lcu.discovery] err=failed Basic cmlvdDpzZWNyZXQ= at C:\\Users\\Alice\\lockfile".to_string(),
    ]);

    let summary = build_diagnostic_summary(&input, None);
    assert_eq!(summary.app_version, "0.4.0");
    assert_eq!(summary.distribution_mode, "NSIS");
    assert_eq!(summary.locale, "pt_BR");
    assert!(summary.gpu_saved);
    assert!(!summary.gpu_active);
    assert!(summary.close_to_tray);
    assert!(summary.lcu_connected);
    assert_eq!(summary.gameflow_phase, "ChampSelect");
    assert_eq!(summary.ddragon_version.as_deref(), Some("16.22.1"));
    assert_eq!(summary.active_patch.as_deref(), Some("16.22"));
    assert_eq!(summary.recent_issues.len(), 1);

    let text = &summary.formatted_text;
    assert!(text.contains("App: v0.4.0 (NSIS)"));
    assert!(text.contains("OS: Windows 11 24H2 (x86_64)"));
    assert!(text.contains("Graphics GPU: saved=on active=off"));
    assert!(text.contains("Close to Tray: on"));
    assert!(text.contains("Game Client: connected (phase=ChampSelect)"));
    assert!(text.contains("Game Data: ddragon=16.22.1 patch=16.22"));
    assert!(text.contains("ram_hits=14 disk_hits=32 misses=3 stale_fallbacks=1"));
    assert!(text.contains("2xx=12 4xx=1 5xx=0 network_errors=1 dropped_logs=0"));
    assert!(text.contains("Basic <redacted>"));
    assert!(text.contains(r"C:\Users\<redacted>\lockfile"));
    assert!(!text.contains("Alice"));
    assert!(!text.contains("cmlvdDpzZWNyZXQ="));
    assert!(!text.to_ascii_lowercase().contains("op.gg"));
    assert!(!text.contains("LeagueClientUx.exe"));
}

#[test]
fn test_build_diagnostic_summary_falls_back_to_disk_tail_when_session_clean() {
    let seq = DIAG_TEST_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "schumazgg_diag_tail_{}_{seq}.log",
        std::process::id()
    ));
    {
        let mut f = fs::File::create(&path).unwrap();
        writeln!(
            f,
            "2026-11-21T15:00:00.000Z WARN [http.client] event=http_response target=analytics status=502"
        )
        .unwrap();
    }

    let mut input = sample_input(Vec::new());
    input.is_portable = true;
    input.gameflow_phase = String::new();
    let summary = build_diagnostic_summary(&input, Some(&path));

    assert_eq!(summary.distribution_mode, "Portable");
    assert_eq!(summary.gameflow_phase, "None");
    assert_eq!(summary.recent_issues.len(), 1);
    assert!(summary.recent_issues[0].contains("status=502"));
    assert!(summary.formatted_text.contains("App: v0.4.0 (Portable)"));
    assert!(summary.formatted_text.contains("status=502"));

    let _ = fs::remove_file(&path);
}
