use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use super::{
    ChampSelectReconciledPayload, ChampSelectedPayload, ChampionBuild, InjectionReport,
    SummonerInfo,
};

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

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct BootstrapProgress(pub BootstrapStatus);

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct LcuConnected(pub SummonerInfo);

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct LcuDisconnected;

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ChampSelected(pub ChampSelectedPayload);

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ChampSelectEnded;

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct RecommendedBuild(pub ChampionBuild);

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct BuildInjected(pub InjectionReport);

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ChampSelectReconciled(pub ChampSelectReconciledPayload);

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ProfileRefreshRequested;

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct GameflowPhaseChanged(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct MatchEnded;

/// Emitted when the main window becomes minimized/hidden (`true`) or restored/visible (`false`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct WindowMinimizedChanged(pub bool);
