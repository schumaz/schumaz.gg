use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static WRITER_TEST_SEQ: AtomicU64 = AtomicU64::new(1);

fn temp_logs_dir(name: &str) -> PathBuf {
    let seq = WRITER_TEST_SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "schumazgg_writer_test_{name}_{}_{seq}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn test_rotation_at_exact_1_mib_caps_at_two_files() {
    let dir = temp_logs_dir("rotate_1mib");
    let handle =
        LogWriterHandle::start_with_config(dir.clone(), LOG_MAX_FILE_BYTES, LOG_CHANNEL_CAPACITY)
            .unwrap();

    // Write ~2.3 MiB of records in batches so rotation triggers multiple times.
    let payload = format!(
        "2026-11-21T16:20:05.123Z INFO [bench] event=fill data={}",
        "x".repeat(440)
    );
    for idx in 0..5_000 {
        while !handle.try_send(LogLevel::Info, format!("{payload} seq={idx}")) {
            handle.flush();
        }
        if idx % 200 == 0 {
            handle.flush();
        }
    }
    handle.flush();

    // Verify shared read access works while the background writer still holds the file handle.
    let active_content = fs::read_to_string(dir.join(LOG_FILE_NAME)).unwrap();
    assert!(!active_content.is_empty());

    handle.shutdown();

    let active_meta = fs::metadata(dir.join(LOG_FILE_NAME)).unwrap();
    let backup_meta = fs::metadata(dir.join(LOG_BACKUP_FILE_NAME)).unwrap();
    assert!(active_meta.len() <= LOG_MAX_FILE_BYTES);
    assert!(backup_meta.len() <= LOG_MAX_FILE_BYTES);
    assert!(backup_meta.len() > LOG_MAX_FILE_BYTES - 1024);

    let files: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(files.len(), 2);
    assert!(files.contains(&LOG_FILE_NAME.to_string()));
    assert!(files.contains(&LOG_BACKUP_FILE_NAME.to_string()));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_channel_saturation_drops_non_blocking_and_logs_consolidated_warning() {
    let dir = temp_logs_dir("saturation");
    // Tiny channel (capacity 2) makes saturation deterministic under rapid producer burst.
    let handle = LogWriterHandle::start_with_config(dir.clone(), LOG_MAX_FILE_BYTES, 2).unwrap();

    let mut dropped = 0u64;
    for i in 0..4_000 {
        let line = format!("2026-11-21T16:20:05.123Z INFO [burst] event=tick seq={i}");
        if !handle.try_send(LogLevel::Info, line) {
            dropped += 1;
        }
    }

    // Flush and send one follow-up line to ensure any pending consolidated drop warning is emitted.
    handle.flush();
    let _ = handle.try_send(
        LogLevel::Warn,
        "2026-11-21T16:20:06.000Z WARN [burst] event=after_burst".to_string(),
    );
    handle.shutdown();

    assert_eq!(handle.dropped_total(), dropped);
    if dropped > 0 {
        let content = fs::read_to_string(dir.join(LOG_FILE_NAME)).unwrap();
        assert!(content.contains("event=dropped_events count="));
    }
    let content = fs::read_to_string(dir.join(LOG_FILE_NAME)).unwrap();
    assert!(content.contains("event=after_burst"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_write_panic_sync_appends_immediately() {
    let dir = temp_logs_dir("panic_sync");
    let log_path = dir.join(LOG_FILE_NAME);
    write_panic_sync(
        &log_path,
        "2026-11-21T16:20:05.123Z ERROR [panic] event=panic message=boom",
    );
    let content = fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("event=panic message=boom"));
    let _ = fs::remove_dir_all(&dir);
}
