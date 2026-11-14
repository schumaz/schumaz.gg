use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use super::{
    ChampSelectReconciledPayload, ChampSelectedPayload, ChampionBuild, InjectionReport,
    SummonerInfo,
};

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
