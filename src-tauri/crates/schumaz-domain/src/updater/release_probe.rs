use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::logging;
use crate::models::{UpdateProbeResult, UpdateProbeStatus};
use crate::storage;

use super::GITHUB_RELEASES_LATEST_URL;
use super::policy::{
    AUTO_UPDATE_CHECK_COOLDOWN_SECS, extract_release_tag_from_location, is_remote_version_newer,
    normalize_semver_tag, should_run_auto_check,
};

const PROBE_TIMEOUT_SECS: u64 = 10;

static PROBE_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Result of a lightweight `HEAD` redirect probe against GitHub Releases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseProbeOutcome {
    UpToDate,
    UpdateAvailable(String),
}

/// Builds a `reqwest::Client` that never follows redirects so probing
/// `/releases/latest` only inspects the `Location` header without downloading assets.
pub fn build_no_redirect_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(PROBE_TIMEOUT_SECS))
        .user_agent(format!("schumaz.gg/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|err| format!("Failed to build release probe HTTP client: {err}"))
}

fn shared_probe_client() -> Result<&'static reqwest::Client, String> {
    if let Some(client) = PROBE_HTTP_CLIENT.get() {
        return Ok(client);
    }
    let created = build_no_redirect_client()?;
    let _ = PROBE_HTTP_CLIENT.set(created);
    PROBE_HTTP_CLIENT
        .get()
        .ok_or_else(|| "Release probe HTTP client unavailable".to_string())
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Probes `latest_release_url` with an HTTP `HEAD` request (without following redirects)
/// and compares the redirected release tag against `current_version`.
///
/// This never touches `latest.json` or any release asset, keeping GitHub download counts clean.
pub async fn probe_latest_release_url(
    client: &reqwest::Client,
    latest_release_url: &str,
    current_version: &str,
) -> Result<ReleaseProbeOutcome, String> {
    let started = Instant::now();
    let response = match client.head(latest_release_url).send().await {
        Ok(resp) => resp,
        Err(err) => {
            logging::record_http_network_error_log(
                "updater.release_probe",
                started.elapsed().as_millis(),
                &err.to_string(),
            );
            return Err(format!("Release probe network error: {err}"));
        }
    };

    let status = response.status();
    logging::record_http_status_log(
        "updater.release_probe",
        status.as_u16(),
        started.elapsed().as_millis(),
    );

    if status.as_u16() == 404 {
        return Ok(ReleaseProbeOutcome::UpToDate);
    }

    if status.is_redirection() {
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|val| val.to_str().ok())
            .ok_or_else(|| "Release redirect missing Location header".to_string())?;

        let Some(raw_tag) = extract_release_tag_from_location(location) else {
            return Ok(ReleaseProbeOutcome::UpToDate);
        };
        let remote_version = normalize_semver_tag(&raw_tag)
            .ok_or_else(|| format!("Unparseable release tag in redirect: {raw_tag}"))?;

        return match is_remote_version_newer(current_version, &remote_version) {
            Some(true) => Ok(ReleaseProbeOutcome::UpdateAvailable(remote_version)),
            Some(false) => Ok(ReleaseProbeOutcome::UpToDate),
            None => Err(format!(
                "Invalid current version for SemVer comparison: {current_version}"
            )),
        };
    }

    Err(format!(
        "Unexpected HTTP status {} probing latest release",
        status.as_u16()
    ))
}

/// Evaluates the 24h persisted cooldown and, when due (or `manual`), performs a lightweight
/// `HEAD /releases/latest` probe before allowing `tauri-plugin-updater` to fetch `latest.json`.
pub async fn probe_for_app_update_with_context(
    is_portable: bool,
    current_version: &str,
    manual: bool,
) -> UpdateProbeResult {
    let last_check = storage::get_last_update_check_epoch_secs();
    let now = unix_now_secs();

    if !manual && !should_run_auto_check(last_check, now, AUTO_UPDATE_CHECK_COOLDOWN_SECS) {
        logging::log_info(
            "updater",
            format!(
                "event=probe_skipped_cooldown current={} last_check={}",
                current_version,
                last_check.unwrap_or(0)
            ),
        );
        return UpdateProbeResult {
            status: UpdateProbeStatus::Cooldown,
            latest_version: None,
            releases_url: GITHUB_RELEASES_LATEST_URL.to_string(),
            is_portable,
            last_check_epoch_secs: last_check,
        };
    }

    let client = match shared_probe_client() {
        Ok(c) => c,
        Err(err) => {
            logging::log_warn("updater", format!("event=probe_client_error err={err}"));
            return UpdateProbeResult {
                status: UpdateProbeStatus::Error,
                latest_version: None,
                releases_url: GITHUB_RELEASES_LATEST_URL.to_string(),
                is_portable,
                last_check_epoch_secs: last_check,
            };
        }
    };

    match probe_latest_release_url(client, GITHUB_RELEASES_LATEST_URL, current_version).await {
        Ok(ReleaseProbeOutcome::UpToDate) => {
            let recorded = persist_check_timestamp_async(now).await.or(last_check);
            logging::log_info(
                "updater",
                format!(
                    "event=probe_up_to_date manual={manual} current={current_version} recorded_ts={}",
                    recorded.unwrap_or(0)
                ),
            );
            UpdateProbeResult {
                status: UpdateProbeStatus::UpToDate,
                latest_version: None,
                releases_url: GITHUB_RELEASES_LATEST_URL.to_string(),
                is_portable,
                last_check_epoch_secs: recorded,
            }
        }
        Ok(ReleaseProbeOutcome::UpdateAvailable(remote_version)) => {
            // Portable finishes here without touching latest.json; NSIS persists after plugin check() succeeds.
            let recorded = if is_portable {
                persist_check_timestamp_async(now).await.or(last_check)
            } else {
                last_check
            };
            logging::log_info(
                "updater",
                format!(
                    "event=probe_update_available manual={manual} current={current_version} remote={remote_version} portable={is_portable}"
                ),
            );
            UpdateProbeResult {
                status: UpdateProbeStatus::UpdateAvailable,
                latest_version: Some(remote_version),
                releases_url: GITHUB_RELEASES_LATEST_URL.to_string(),
                is_portable,
                last_check_epoch_secs: recorded,
            }
        }
        Err(err) => {
            logging::log_warn(
                "updater",
                format!("event=probe_error manual={manual} err={err}"),
            );
            UpdateProbeResult {
                status: UpdateProbeStatus::Error,
                latest_version: None,
                releases_url: GITHUB_RELEASES_LATEST_URL.to_string(),
                is_portable,
                last_check_epoch_secs: last_check,
            }
        }
    }
}

fn persist_check_timestamp(now: u64) -> Option<u64> {
    if now == 0 {
        return None;
    }
    match storage::record_last_update_check_epoch_secs(now) {
        Ok(_) => Some(now),
        Err(err) => {
            logging::log_warn(
                "updater",
                format!("event=record_check_timestamp_failed err={err}"),
            );
            None
        }
    }
}

async fn persist_check_timestamp_async(now: u64) -> Option<u64> {
    tokio::task::spawn_blocking(move || persist_check_timestamp(now))
        .await
        .ok()
        .flatten()
}

/// Persists the current Unix timestamp as the last completed update check in `UserSettings.ini`.
pub fn record_update_check_completed() -> Option<u64> {
    let now = unix_now_secs();
    persist_check_timestamp(now).or_else(storage::get_last_update_check_epoch_secs)
}
