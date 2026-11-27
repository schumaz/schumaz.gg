use serde::{Deserialize, Serialize};
use specta::Type;

/// Sanitized local diagnostic snapshot exported for troubleshooting without external telemetry.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticSummary {
    pub app_version: String,
    pub distribution_mode: String,
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
    pub recent_issues: Vec<String>,
    pub formatted_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapStatus {
    pub stage: String,
    pub progress: u8,
    pub disk_ready: bool,
    pub network_ready: bool,
}

impl Default for BootstrapStatus {
    fn default() -> Self {
        Self {
            stage: "initializing".to_string(),
            progress: 0,
            disk_ready: false,
            network_ready: false,
        }
    }
}
