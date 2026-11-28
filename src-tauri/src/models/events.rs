use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use schumaz_models::{
    BootstrapStatus, CacheClearScope, ChampSelectReconciledPayload, ChampSelectedPayload,
    ChampionBuild, InjectionReport, SummonerInfo,
};

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

/// Emitted after local caches are cleared so frontend hooks invalidate their in-memory caches.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct CacheCleared(pub CacheClearScope);
