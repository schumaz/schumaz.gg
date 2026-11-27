use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use super::sanitizer::sanitize_log_message;

/// Maximum number of recent `WARN`/`ERROR` entries kept in memory for instant diagnostics.
pub const RECENT_ISSUES_CAPACITY: usize = 25;
/// Maximum byte window read from the tail of `schumazgg.log` when the in-memory ring buffer is empty.
pub const BOUNDED_TAIL_MAX_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheMetricsSnapshot {
    pub ram_hits: u64,
    pub disk_hits: u64,
    pub misses: u64,
    pub stale_fallbacks: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HttpMetricsSnapshot {
    pub status_2xx: u64,
    pub status_4xx: u64,
    pub status_5xx: u64,
    pub network_errors: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeMetricsSnapshot {
    pub cache: CacheMetricsSnapshot,
    pub http: HttpMetricsSnapshot,
    pub dropped_events: u64,
}

/// Lock-free cache counters avoiding hot-path log spam.
#[derive(Default)]
pub struct CacheMetrics {
    ram_hits: AtomicU64,
    disk_hits: AtomicU64,
    misses: AtomicU64,
    stale_fallbacks: AtomicU64,
}

impl CacheMetrics {
    pub fn record_ram_hit(&self) {
        self.ram_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_disk_hit(&self) {
        self.disk_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_stale_fallback(&self) {
        self.stale_fallbacks.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> CacheMetricsSnapshot {
        CacheMetricsSnapshot {
            ram_hits: self.ram_hits.load(Ordering::Relaxed),
            disk_hits: self.disk_hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            stale_fallbacks: self.stale_fallbacks.load(Ordering::Relaxed),
        }
    }
}

/// Lock-free HTTP response bucket counters.
#[derive(Default)]
pub struct HttpMetrics {
    status_2xx: AtomicU64,
    status_4xx: AtomicU64,
    status_5xx: AtomicU64,
    network_errors: AtomicU64,
}

impl HttpMetrics {
    pub fn record_status(&self, status: u16) {
        match status {
            200..=299 => {
                self.status_2xx.fetch_add(1, Ordering::Relaxed);
            }
            400..=499 => {
                self.status_4xx.fetch_add(1, Ordering::Relaxed);
            }
            500..=599 => {
                self.status_5xx.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    pub fn record_network_error(&self) {
        self.network_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> HttpMetricsSnapshot {
        HttpMetricsSnapshot {
            status_2xx: self.status_2xx.load(Ordering::Relaxed),
            status_4xx: self.status_4xx.load(Ordering::Relaxed),
            status_5xx: self.status_5xx.load(Ordering::Relaxed),
            network_errors: self.network_errors.load(Ordering::Relaxed),
        }
    }
}

/// Bounded FIFO ring buffer storing recent sanitized `WARN`/`ERROR` log lines in memory.
pub struct IssueRingBuffer {
    capacity: usize,
    entries: Mutex<VecDeque<String>>,
}

impl IssueRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: Mutex::new(VecDeque::with_capacity(capacity.max(1))),
        }
    }

    pub fn push(&self, line: String) {
        if let Ok(mut guard) = self.entries.lock() {
            if guard.len() >= self.capacity {
                guard.pop_front();
            }
            guard.push_back(line);
        }
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.entries
            .lock()
            .map(|guard| guard.iter().cloned().collect())
            .unwrap_or_default()
    }
}

static CACHE_METRICS: CacheMetrics = CacheMetrics {
    ram_hits: AtomicU64::new(0),
    disk_hits: AtomicU64::new(0),
    misses: AtomicU64::new(0),
    stale_fallbacks: AtomicU64::new(0),
};

static HTTP_METRICS: HttpMetrics = HttpMetrics {
    status_2xx: AtomicU64::new(0),
    status_4xx: AtomicU64::new(0),
    status_5xx: AtomicU64::new(0),
    network_errors: AtomicU64::new(0),
};

static DROPPED_EVENTS: AtomicU64 = AtomicU64::new(0);
static RECENT_ISSUES: OnceLock<IssueRingBuffer> = OnceLock::new();

fn recent_issues_buffer() -> &'static IssueRingBuffer {
    RECENT_ISSUES.get_or_init(|| IssueRingBuffer::new(RECENT_ISSUES_CAPACITY))
}

pub fn record_cache_ram_hit() {
    CACHE_METRICS.record_ram_hit();
}

pub fn record_cache_disk_hit() {
    CACHE_METRICS.record_disk_hit();
}

pub fn record_cache_miss() {
    CACHE_METRICS.record_miss();
}

pub fn record_cache_stale_fallback() {
    CACHE_METRICS.record_stale_fallback();
}

pub fn record_http_status(status: u16) {
    HTTP_METRICS.record_status(status);
}

pub fn record_http_network_error() {
    HTTP_METRICS.record_network_error();
}

pub fn record_dropped_event() {
    DROPPED_EVENTS.fetch_add(1, Ordering::Relaxed);
}

pub fn dropped_events_total() -> u64 {
    DROPPED_EVENTS.load(Ordering::Relaxed)
}

pub fn push_recent_issue(sanitized_line: String) {
    recent_issues_buffer().push(sanitized_line);
}

pub fn recent_issues_snapshot() -> Vec<String> {
    recent_issues_buffer().snapshot()
}

pub fn runtime_metrics_snapshot() -> RuntimeMetricsSnapshot {
    RuntimeMetricsSnapshot {
        cache: CACHE_METRICS.snapshot(),
        http: HTTP_METRICS.snapshot(),
        dropped_events: dropped_events_total(),
    }
}

fn is_warn_or_error_line(line: &str) -> bool {
    line.contains(" WARN ") || line.contains(" ERROR ")
}

/// Reads at most the final `max_bytes` of `log_path` and returns up to `max_lines` sanitized entries.
/// Prefers `WARN`/`ERROR` entries when present; otherwise returns the latest lines from the tail.
pub fn read_bounded_log_tail(log_path: &Path, max_bytes: usize, max_lines: usize) -> Vec<String> {
    if max_bytes == 0 || max_lines == 0 {
        return Vec::new();
    }
    let Ok(mut file) = File::open(log_path) else {
        return Vec::new();
    };
    let Ok(metadata) = file.metadata() else {
        return Vec::new();
    };
    let file_len = metadata.len();
    if file_len == 0 {
        return Vec::new();
    }

    let read_len = (file_len as usize).min(max_bytes);
    let skipped_prefix = file_len > read_len as u64;
    if skipped_prefix && file.seek(SeekFrom::End(-(read_len as i64))).is_err() {
        return Vec::new();
    }

    let mut buf = vec![0u8; read_len];
    let Ok(bytes_read) = file.read(&mut buf) else {
        return Vec::new();
    };
    buf.truncate(bytes_read);

    let text = String::from_utf8_lossy(&buf);
    let mut lines = text.lines();
    if skipped_prefix {
        // Discard the potentially cut first line after seeking into the middle of the file.
        let _ = lines.next();
    }

    let all_lines: Vec<String> = lines
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(sanitize_log_message)
        .collect();

    let issues: Vec<String> = all_lines
        .iter()
        .filter(|line| is_warn_or_error_line(line))
        .cloned()
        .collect();

    let selected = if !issues.is_empty() {
        issues
    } else {
        all_lines
    };

    let skip = selected.len().saturating_sub(max_lines);
    selected.into_iter().skip(skip).collect()
}

/// Resolves recent issues from the in-memory ring buffer (< 1 ms), falling back to a bounded
/// `32 KiB` tail read from `log_path` when the current session has not recorded any warnings/errors.
pub fn resolve_recent_issues_with_fallback(
    in_memory: Vec<String>,
    log_path: Option<&Path>,
) -> Vec<String> {
    if !in_memory.is_empty() {
        return in_memory;
    }
    match log_path {
        Some(path) => read_bounded_log_tail(path, BOUNDED_TAIL_MAX_BYTES, RECENT_ISSUES_CAPACITY),
        None => Vec::new(),
    }
}

#[cfg(test)]
#[path = "metrics_tests.rs"]
mod tests;
