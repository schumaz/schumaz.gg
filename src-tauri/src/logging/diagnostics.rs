use std::path::Path;

use crate::models::DiagnosticSummary;

use super::metrics::{RuntimeMetricsSnapshot, resolve_recent_issues_with_fallback};
use super::sanitizer::sanitize_log_message;

/// Pure input snapshot used to build a sanitized `DiagnosticSummary` deterministically.
#[derive(Debug, Clone)]
pub struct DiagnosticInput {
    pub app_version: String,
    pub is_portable: bool,
    pub os_version: String,
    pub architecture: String,
    pub locale: String,
    pub gpu_saved: bool,
    pub gpu_active: bool,
    pub close_to_tray: bool,
    pub lcu_connected: bool,
    pub gameflow_phase: String,
    pub ddragon_version: Option<String>,
    pub active_patch: Option<String>,
    pub metrics: RuntimeMetricsSnapshot,
    pub in_memory_issues: Vec<String>,
}

/// Formats the host OS version using `sysinfo` without exposing user account information.
pub fn detect_os_version() -> String {
    sysinfo::System::long_os_version()
        .or_else(sysinfo::System::os_version)
        .unwrap_or_else(|| std::env::consts::OS.to_string())
}

fn clean_optional_field(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(sanitize_log_message)
        .unwrap_or_else(|| "unavailable".to_string())
}

/// Pure constructor assembling a sanitized `DiagnosticSummary` ready to copy into support issues.
/// Never references external data providers or internal client executable names.
pub fn build_diagnostic_summary(
    input: &DiagnosticInput,
    log_path: Option<&Path>,
) -> DiagnosticSummary {
    let distribution_mode = if input.is_portable {
        "Portable".to_string()
    } else {
        "NSIS".to_string()
    };
    let app_version = sanitize_log_message(input.app_version.trim());
    let os_version = sanitize_log_message(input.os_version.trim());
    let architecture = sanitize_log_message(input.architecture.trim());
    let locale = sanitize_log_message(input.locale.trim());
    let gameflow_phase = {
        let trimmed = input.gameflow_phase.trim();
        if trimmed.is_empty() {
            "None".to_string()
        } else {
            sanitize_log_message(trimmed)
        }
    };
    let ddragon_version = input
        .ddragon_version
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(sanitize_log_message);
    let active_patch = input
        .active_patch
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(sanitize_log_message);

    let sanitized_mem: Vec<String> = input
        .in_memory_issues
        .iter()
        .map(|line| sanitize_log_message(line))
        .filter(|line| !line.is_empty())
        .collect();

    let recent_issues = resolve_recent_issues_with_fallback(sanitized_mem, log_path);
    let cache = &input.metrics.cache;
    let http = &input.metrics.http;

    let mut lines = vec![
        "=== schumaz.gg Diagnostic Summary ===".to_string(),
        format!("App: v{app_version} ({distribution_mode})"),
        format!("OS: {os_version} ({architecture})"),
        format!("Locale: {locale}"),
        format!(
            "Graphics GPU: saved={} active={}",
            if input.gpu_saved { "on" } else { "off" },
            if input.gpu_active { "on" } else { "off" }
        ),
        format!(
            "Close to Tray: {}",
            if input.close_to_tray { "on" } else { "off" }
        ),
        format!(
            "Game Client: {} (phase={})",
            if input.lcu_connected {
                "connected"
            } else {
                "disconnected"
            },
            gameflow_phase
        ),
        format!(
            "Game Data: ddragon={} patch={}",
            clean_optional_field(ddragon_version.as_deref()),
            clean_optional_field(active_patch.as_deref())
        ),
        format!(
            "Cache Metrics: ram_hits={} disk_hits={} misses={} stale_fallbacks={}",
            cache.ram_hits, cache.disk_hits, cache.misses, cache.stale_fallbacks
        ),
        format!(
            "HTTP Metrics: 2xx={} 4xx={} 5xx={} network_errors={} dropped_logs={}",
            http.status_2xx,
            http.status_4xx,
            http.status_5xx,
            http.network_errors,
            input.metrics.dropped_events
        ),
    ];

    lines.push("Recent Warnings/Errors:".to_string());
    if recent_issues.is_empty() {
        lines.push("  (none)".to_string());
    } else {
        for entry in &recent_issues {
            lines.push(format!("  {entry}"));
        }
    }

    DiagnosticSummary {
        app_version,
        distribution_mode,
        os_version,
        architecture,
        locale,
        gpu_saved: input.gpu_saved,
        gpu_active: input.gpu_active,
        close_to_tray: input.close_to_tray,
        lcu_connected: input.lcu_connected,
        gameflow_phase,
        ddragon_version,
        active_patch,
        recent_issues,
        formatted_text: lines.join("\n"),
    }
}

#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod tests;
