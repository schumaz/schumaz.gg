use super::*;
use std::fs;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_SEQ: AtomicU64 = AtomicU64::new(1);

fn temp_log_file(prefix: &str) -> std::path::PathBuf {
    let seq = TEST_SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "schumazgg_metrics_test_{prefix}_{}_{seq}.log",
        std::process::id()
    ))
}

#[test]
fn test_cache_and_http_atomic_counters() {
    let cache = CacheMetrics::default();
    cache.record_ram_hit();
    cache.record_ram_hit();
    cache.record_disk_hit();
    cache.record_miss();
    cache.record_stale_fallback();
    assert_eq!(
        cache.snapshot(),
        CacheMetricsSnapshot {
            ram_hits: 2,
            disk_hits: 1,
            misses: 1,
            stale_fallbacks: 1,
        }
    );

    let http = HttpMetrics::default();
    http.record_status(200);
    http.record_status(204);
    http.record_status(404);
    http.record_status(503);
    http.record_network_error();
    assert_eq!(
        http.snapshot(),
        HttpMetricsSnapshot {
            status_2xx: 2,
            status_4xx: 1,
            status_5xx: 1,
            network_errors: 1,
        }
    );
}

#[test]
fn test_ring_buffer_evicts_oldest_entries_fifo() {
    let buf = IssueRingBuffer::new(3);
    buf.push("issue-1".to_string());
    buf.push("issue-2".to_string());
    buf.push("issue-3".to_string());
    buf.push("issue-4".to_string());
    assert_eq!(buf.snapshot(), vec!["issue-2", "issue-3", "issue-4"]);
}

#[test]
fn test_bounded_tail_fallback_reads_last_32k_and_prefers_warn_error() {
    let path = temp_log_file("tail");
    {
        let mut f = fs::File::create(&path).unwrap();
        // Write > 40 KiB of INFO lines so the 32 KiB tail seek lands in the middle of the file.
        for i in 0..700 {
            writeln!(
                f,
                "2026-11-21T16:00:00.000Z INFO [test] event=noise seq={i:04} padding=0123456789abcdef0123456789abcdef"
            )
            .unwrap();
        }
        writeln!(
            f,
            "2026-11-21T16:01:00.000Z WARN [http.client] event=http_response status=503 path=C:\\Users\\Alice\\file"
        )
        .unwrap();
        writeln!(
            f,
            "2026-11-21T16:02:00.000Z ERROR [ws.listener] event=ws_error header=Basic cmlvdDpzZWNyZXQ="
        )
        .unwrap();
    }

    let in_mem = vec!["2026-11-21T16:10:00.000Z WARN [mem] event=in_memory".to_string()];
    assert_eq!(
        resolve_recent_issues_with_fallback(in_mem.clone(), Some(&path)),
        in_mem
    );

    let tail = resolve_recent_issues_with_fallback(Vec::new(), Some(&path));
    assert_eq!(tail.len(), 2);
    assert!(tail[0].contains("WARN [http.client]"));
    assert!(tail[0].contains(r"C:\Users\<redacted>\file"));
    assert!(!tail[0].contains("Alice"));
    assert!(tail[1].contains("ERROR [ws.listener]"));
    assert!(tail[1].contains("Basic <redacted>"));

    let _ = fs::remove_file(&path);
}
