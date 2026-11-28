use heck::ToKebabCase;
use serde::{Deserialize, Serialize};
use specta::{Type, Types, datatype::DataType};

use crate::{
    BootstrapStatus, CacheClearScope, ChampSelectReconciledPayload, ChampSelectedPayload,
    ChampionBuild, InjectionReport, SummonerInfo,
};

/// IPC event metadata registered purely for TypeScript bindings generation without pulling Tauri.
pub struct IpcEventSpec {
    pub struct_name: &'static str,
    pub event_name: String,
    pub data_type: DataType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BootstrapProgress(pub BootstrapStatus);

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LcuConnected(pub SummonerInfo);

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LcuDisconnected;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ChampSelected(pub ChampSelectedPayload);

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ChampSelectEnded;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RecommendedBuild(pub ChampionBuild);

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BuildInjected(pub InjectionReport);

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ChampSelectReconciled(pub ChampSelectReconciledPayload);

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProfileRefreshRequested;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct GameflowPhaseChanged(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MatchEnded;

/// Emitted when the main window becomes minimized/hidden (`true`) or restored/visible (`false`).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct WindowMinimizedChanged(pub bool);

/// Emitted after local caches are cleared so frontend hooks invalidate their in-memory caches.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CacheCleared(pub CacheClearScope);

fn spec<T: Type>(struct_name: &'static str, types: &mut Types) -> IpcEventSpec {
    IpcEventSpec {
        struct_name,
        event_name: struct_name.to_kebab_case(),
        data_type: T::definition(types),
    }
}

pub fn collect_events(types: &mut Types) -> Vec<IpcEventSpec> {
    vec![
        spec::<BootstrapProgress>("BootstrapProgress", types),
        spec::<LcuConnected>("LcuConnected", types),
        spec::<LcuDisconnected>("LcuDisconnected", types),
        spec::<ChampSelected>("ChampSelected", types),
        spec::<ChampSelectEnded>("ChampSelectEnded", types),
        spec::<RecommendedBuild>("RecommendedBuild", types),
        spec::<BuildInjected>("BuildInjected", types),
        spec::<ChampSelectReconciled>("ChampSelectReconciled", types),
        spec::<ProfileRefreshRequested>("ProfileRefreshRequested", types),
        spec::<GameflowPhaseChanged>("GameflowPhaseChanged", types),
        spec::<MatchEnded>("MatchEnded", types),
        spec::<WindowMinimizedChanged>("WindowMinimizedChanged", types),
        spec::<CacheCleared>("CacheCleared", types),
    ]
}
