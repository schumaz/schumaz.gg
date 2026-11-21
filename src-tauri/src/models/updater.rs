use serde::{Deserialize, Serialize};
use specta::Type;

/// Runtime distribution metadata used by the frontend updater to distinguish
/// NSIS `currentUser` installations from standalone portable executables.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppDistributionInfo {
    pub is_portable: bool,
    pub current_version: String,
    pub releases_url: String,
}

/// OS autostart status returned to the Settings UI (disabled and blocked in portable builds).
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutostartStatus {
    pub enabled: bool,
    pub supported: bool,
    pub is_portable: bool,
}
