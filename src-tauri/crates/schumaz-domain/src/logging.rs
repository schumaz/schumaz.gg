pub mod diagnostics;
pub mod metrics;
pub mod sanitizer;
pub mod writer;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use self::metrics::{push_recent_issue, record_dropped_event};
use self::sanitizer::sanitize_log_message;
use self::writer::{LogWriterHandle, write_panic_sync};

static GLOBAL_LOGGER: OnceLock<LogWriterHandle> = OnceLock::new();
static PANIC_HOOK_INSTALLED: OnceLock<()> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

/// Converts days since `1970-01-01` into proleptic Gregorian `(year, month, day)`.
/// Howard Hinnant's `civil_from_days` algorithm in pure `std`.
fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + i64::from(m <= 2);
    (year, m as u32, d as u32)
}

/// Formats a `SystemTime` as compact UTC ISO-8601 with milliseconds (`YYYY-MM-DDTHH:MM:SS.mmmZ`).
pub fn format_utc_iso8601_millis(time: SystemTime) -> String {
    let dur = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let total_secs = dur.as_secs();
    let millis = dur.subsec_millis();
    let days = (total_secs / 86_400) as i64;
    let sec_of_day = total_secs % 86_400;
    let hour = sec_of_day / 3_600;
    let minute = (sec_of_day % 3_600) / 60;
    let second = sec_of_day % 60;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

/// Formats a single-line sanitized `logfmt` record (`<iso> <LEVEL> [<target>] <sanitized_msg>`).
pub fn format_log_line_at(
    time: SystemTime,
    level: LogLevel,
    target: &str,
    message: &str,
) -> String {
    let ts = format_utc_iso8601_millis(time);
    let clean_target = sanitize_log_message(target);
    let clean_msg = sanitize_log_message(message);
    format!("{ts} {} [{clean_target}] {clean_msg}", level.as_str())
}

/// Initializes the global background logger and installs the synchronous panic hook once.
pub fn init_logging(logs_dir: &Path) -> Result<(), String> {
    if GLOBAL_LOGGER.get().is_some() {
        return Ok(());
    }
    let handle = LogWriterHandle::start(logs_dir.to_path_buf())?;
    let log_path = handle.log_path().to_path_buf();
    let _ = GLOBAL_LOGGER.set(handle);
    install_panic_hook(log_path);
    Ok(())
}

/// Installs a panic hook that writes a sanitized crash record synchronously before aborting.
pub fn install_panic_hook(log_path: PathBuf) {
    PANIC_HOOK_INSTALLED.get_or_init(|| {
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "non-string panic payload".to_string()
            };
            let location = panic_info
                .location()
                .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
                .unwrap_or_else(|| "unknown".to_string());
            let raw = format!("event=panic location={location} message={payload}");
            let line = format_log_line_at(SystemTime::now(), LogLevel::Error, "panic", &raw);
            write_panic_sync(&log_path, &line);
            flush_logs();
            prev_hook(panic_info);
        }));
    });
}

/// Returns the active log file path when logging has been initialized.
pub fn active_log_file_path() -> Option<&'static Path> {
    GLOBAL_LOGGER.get().map(LogWriterHandle::log_path)
}

/// Core non-blocking log entry point applying mandatory central sanitization.
pub fn log_event(level: LogLevel, target: &str, message: &str) {
    let line = format_log_line_at(SystemTime::now(), level, target, message);
    if matches!(level, LogLevel::Warn | LogLevel::Error) {
        push_recent_issue(line.clone());
    }
    if let Some(logger) = GLOBAL_LOGGER.get()
        && !logger.try_send(level, line)
    {
        record_dropped_event();
    }
}

pub fn log_debug(target: &str, message: impl AsRef<str>) {
    log_event(LogLevel::Debug, target, message.as_ref());
}

pub fn log_info(target: &str, message: impl AsRef<str>) {
    log_event(LogLevel::Info, target, message.as_ref());
}

pub fn log_warn(target: &str, message: impl AsRef<str>) {
    log_event(LogLevel::Warn, target, message.as_ref());
}

pub fn log_error(target: &str, message: impl AsRef<str>) {
    log_event(LogLevel::Error, target, message.as_ref());
}

/// Records an HTTP status bucket and emits a concise structured log entry.
pub fn record_http_status_log(target: &str, status: u16, elapsed_ms: u128) {
    metrics::record_http_status(status);
    let msg =
        format!("event=http_response target={target} status={status} elapsed_ms={elapsed_ms}");
    if status >= 400 {
        log_warn("http.client", msg);
    } else {
        log_debug("http.client", msg);
    }
}

/// Records a network transport error bucket and emits a sanitized `WARN` entry.
pub fn record_http_network_error_log(target: &str, elapsed_ms: u128, err: &str) {
    metrics::record_http_network_error();
    log_warn(
        "http.client",
        format!("event=http_error target={target} elapsed_ms={elapsed_ms} err={err}"),
    );
}

/// Records a build/tier-list/metadata resolution outcome and updates lock-free cache metrics.
pub fn record_data_resolution(kind: &str, source: &str, elapsed_ms: u128, detail: &str) {
    match source {
        "ram_hit" => metrics::record_cache_ram_hit(),
        "disk_hit" => metrics::record_cache_disk_hit(),
        "stale_fallback" => metrics::record_cache_stale_fallback(),
        _ => metrics::record_cache_miss(),
    }
    let msg = if detail.is_empty() {
        format!("event=resolve kind={kind} source={source} elapsed_ms={elapsed_ms}")
    } else {
        format!("event=resolve kind={kind} source={source} elapsed_ms={elapsed_ms} {detail}")
    };
    if source == "stale_fallback" {
        log_warn("build.provider", msg);
    } else {
        log_info("build.provider", msg);
    }
}

/// Flushes buffered log lines synchronously (e.g. before restart or opening the logs folder).
pub fn flush_logs() {
    if let Some(logger) = GLOBAL_LOGGER.get() {
        logger.flush();
    }
}

/// Flushes and cleanly terminates the background logger thread on application exit.
pub fn shutdown_logging() {
    if let Some(logger) = GLOBAL_LOGGER.get() {
        logger.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_format_utc_iso8601_millis_epoch_and_known_date() {
        assert_eq!(
            format_utc_iso8601_millis(UNIX_EPOCH),
            "1970-01-01T00:00:00.000Z"
        );
        // 2026-11-21T16:20:05.123Z = 1_795_278_005s + 123ms
        // Verify leap year boundary 2024-02-29T23:59:59.999Z = 1_709_251_199s + 999ms
        let leap = UNIX_EPOCH + Duration::new(1_709_251_199, 999_000_000);
        assert_eq!(format_utc_iso8601_millis(leap), "2024-02-29T23:59:59.999Z");
    }

    #[test]
    fn test_format_log_line_applies_logfmt_and_central_sanitization() {
        let line = format_log_line_at(
            UNIX_EPOCH,
            LogLevel::Info,
            "lcu.monitor",
            "event=connected header=Basic cmlvdDpzZWNyZXQ=",
        );
        assert_eq!(
            line,
            "1970-01-01T00:00:00.000Z INFO [lcu.monitor] event=connected header=Basic <redacted>"
        );
    }
}
