use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

/// Per-key negative-cache cooldown applied after HTTP 429, 5xx, or network failures
/// to prevent burst retry storms against upstream analytics endpoints.
pub const FAILURE_COOLDOWN_TTL: Duration = Duration::from_secs(45);
const MAX_COOLDOWN_ENTRIES: usize = 256;

#[derive(Debug, Clone)]
struct CooldownEntry {
    until: Instant,
    reason: String,
}

/// Thread-safe, bounded per-key failure cooldown store.
pub struct FailureCooldownStore {
    ttl: Duration,
    max_entries: usize,
    entries: RwLock<HashMap<String, CooldownEntry>>,
}

impl FailureCooldownStore {
    pub fn new(ttl: Duration) -> Self {
        Self::with_capacity(ttl, MAX_COOLDOWN_ENTRIES)
    }

    pub fn with_capacity(ttl: Duration, max_entries: usize) -> Self {
        Self {
            ttl,
            max_entries: max_entries.max(1),
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Returns `Some((remaining, reason))` if `key` is currently cooling down at `now`.
    pub fn check_at(&self, key: &str, now: Instant) -> Option<(Duration, String)> {
        let guard = self.entries.read().ok()?;
        let entry = guard.get(key)?;
        if entry.until > now {
            Some((
                entry.until.saturating_duration_since(now),
                entry.reason.clone(),
            ))
        } else {
            None
        }
    }

    /// Records a failure cooldown for `key` lasting `self.ttl` starting from `now`.
    pub fn record_failure_at(&self, key: &str, reason: impl Into<String>, now: Instant) {
        let Ok(mut guard) = self.entries.write() else {
            return;
        };
        if guard.len() >= self.max_entries && !guard.contains_key(key) {
            guard.retain(|_, entry| entry.until > now);
            if guard.len() >= self.max_entries
                && let Some(oldest_key) = guard
                    .iter()
                    .min_by_key(|(_, entry)| entry.until)
                    .map(|(k, _)| k.clone())
            {
                guard.remove(&oldest_key);
            }
        }
        guard.insert(
            key.to_string(),
            CooldownEntry {
                until: now + self.ttl,
                reason: reason.into(),
            },
        );
    }

    /// Clears any active or expired cooldown entry for `key`.
    pub fn clear(&self, key: &str) {
        if let Ok(mut guard) = self.entries.write() {
            guard.remove(key);
        }
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.read().map(|g| g.len()).unwrap_or(0)
    }
}

static ANALYTICS_FAILURE_COOLDOWN: OnceLock<FailureCooldownStore> = OnceLock::new();

fn global_cooldown_store() -> &'static FailureCooldownStore {
    ANALYTICS_FAILURE_COOLDOWN.get_or_init(|| FailureCooldownStore::new(FAILURE_COOLDOWN_TTL))
}

/// Returns `true` for transient upstream statuses that should activate anti-flood cooldown (`429`, `5xx`).
pub(crate) fn is_cooldown_status(status: u16) -> bool {
    status == 429 || (500..=599).contains(&status)
}

/// Checks if `key` is in cooldown and returns a fast-fail `Err` without touching the network.
pub(crate) fn check_cooldown(key: &str) -> Result<(), String> {
    if let Some((remaining, reason)) = global_cooldown_store().check_at(key, Instant::now()) {
        let secs = remaining.as_secs().max(1);
        crate::logging::log_debug(
            "analytics.cooldown",
            format!("event=cooldown_short_circuit reason={reason} remaining_secs={secs}"),
        );
        return Err(format!(
            "Analytics request in cooldown ({secs}s remaining, reason={reason})"
        ));
    }
    Ok(())
}

/// Records a network/transport failure cooldown for `key`.
pub(crate) fn record_network_failure(key: &str) {
    global_cooldown_store().record_failure_at(key, "network_error", Instant::now());
    crate::logging::log_warn(
        "analytics.cooldown",
        format!(
            "event=cooldown_activated reason=network_error ttl_secs={}",
            FAILURE_COOLDOWN_TTL.as_secs()
        ),
    );
}

/// Records cooldown on HTTP `429`/`5xx` or clears an existing entry on HTTP `2xx`.
pub(crate) fn record_http_status(key: &str, status: u16) {
    if is_cooldown_status(status) {
        let reason = format!("http_{status}");
        global_cooldown_store().record_failure_at(key, &reason, Instant::now());
        crate::logging::log_warn(
            "analytics.cooldown",
            format!(
                "event=cooldown_activated reason={reason} ttl_secs={}",
                FAILURE_COOLDOWN_TTL.as_secs()
            ),
        );
    } else if (200..=299).contains(&status) {
        global_cooldown_store().clear(key);
    }
}

/// Executes an analytics HTTP request guarded by the per-key failure cooldown and uniform HTTP telemetry.
pub(crate) async fn send_analytics_request(
    cooldown_key: &str,
    request: reqwest::RequestBuilder,
    label: &str,
) -> Result<reqwest::Response, String> {
    check_cooldown(cooldown_key)?;

    let started = Instant::now();
    let res = request.send().await.map_err(|err| {
        crate::logging::record_http_network_error_log(
            "analytics",
            started.elapsed().as_millis(),
            &err.to_string(),
        );
        record_network_failure(cooldown_key);
        format!("Failed to fetch {label}: {err}")
    })?;

    let status = res.status();
    crate::logging::record_http_status_log(
        "analytics",
        status.as_u16(),
        started.elapsed().as_millis(),
    );
    record_http_status(cooldown_key, status.as_u16());

    if !status.is_success() {
        return Err(format!("{label} HTTP error: {status}"));
    }
    Ok(res)
}

#[cfg(test)]
#[path = "cooldown_tests.rs"]
mod tests;
