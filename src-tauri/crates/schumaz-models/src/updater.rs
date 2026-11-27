use serde::{Deserialize, Serialize};
use specta::Type;

/// Runtime distribution metadata used by the frontend updater to distinguish
/// NSIS `currentUser` installations from standalone portable executables and
/// expose persisted auto-check cadence metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppDistributionInfo {
    pub is_portable: bool,
    pub current_version: String,
    pub releases_url: String,
    pub last_check_epoch_secs: Option<u64>,
    pub check_cooldown_secs: u64,
}

/// Outcome of the lightweight `HEAD /releases/latest` pre-check gate.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateProbeStatus {
    Cooldown,
    UpToDate,
    UpdateAvailable,
    Error,
}

/// Result returned to the frontend by `probe_for_app_update`.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProbeResult {
    pub status: UpdateProbeStatus,
    pub latest_version: Option<String>,
    pub releases_url: String,
    pub is_portable: bool,
    pub last_check_epoch_secs: Option<u64>,
}

/// OS autostart status returned to the Settings UI (disabled and blocked in portable builds).
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutostartStatus {
    pub enabled: bool,
    pub supported: bool,
    pub is_portable: bool,
}
