use serde::{Deserialize, Serialize};
use specta::Type;

pub const QUEUE_SOLO_DUO: i32 = 420;
pub const QUEUE_FLEX: i32 = 440;
pub const QUEUE_ARAM: i32 = 450;
pub const QUEUE_ARAM_MAYHEM: i32 = 2400;
pub const QUEUE_ARENA: i32 = 1700;
pub const QUEUE_ARENA_1710: i32 = 1710;
pub const QUEUE_ARENA_1750: i32 = 1750;
pub const QUEUE_NORMAL_DRAFT: i32 = 400;
pub const QUEUE_QUICKPLAY: i32 = 490;
pub const QUEUE_PRACTICE_TOOL: i32 = -1;
pub const MAP_ARENA: i32 = 30;

/// Returns `true` for any known Riot Arena (`CHERRY`) queue ID (`1700`, `1710`, `1750`).
pub const fn is_arena_queue(queue_id: i32) -> bool {
    matches!(queue_id, QUEUE_ARENA | QUEUE_ARENA_1710 | QUEUE_ARENA_1750)
}

/// Returns `true` when the queue supports custom rune pages (disabled in ARAM Mayhem and Arena).
pub const fn queue_supports_runes(queue_id: i32) -> bool {
    queue_id != QUEUE_ARAM_MAYHEM && !is_arena_queue(queue_id)
}

/// Returns `true` when the queue supports custom summoner spell selection (disabled in Arena).
pub const fn queue_supports_spells(queue_id: i32) -> bool {
    !is_arena_queue(queue_id)
}

/// Game queue classifications.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Type)]
pub enum QueueMode {
    SummonersRift,
    Aram,
    AramMayhem,
    Arena,
    PracticeTool,
    Other,
}

impl From<i32> for QueueMode {
    fn from(queue_id: i32) -> Self {
        match queue_id {
            QUEUE_ARAM => QueueMode::Aram,
            QUEUE_ARAM_MAYHEM => QueueMode::AramMayhem,
            QUEUE_ARENA | QUEUE_ARENA_1710 | QUEUE_ARENA_1750 => QueueMode::Arena,
            QUEUE_PRACTICE_TOOL => QueueMode::PracticeTool,
            QUEUE_SOLO_DUO | QUEUE_FLEX | QUEUE_NORMAL_DRAFT | QUEUE_QUICKPLAY => {
                QueueMode::SummonersRift
            }
            _ => QueueMode::Other,
        }
    }
}

/// Payload emitted to the Tauri frontend when the user selects or locks in a champion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
pub struct ChampSelectedPayload {
    pub champion_id: i32,
    pub queue_id: i32,
    pub queue_mode: QueueMode,
    pub locked_in: bool,
    #[serde(default)]
    pub assigned_position: Option<String>,
}

/// Payload emitted when final Champ Select reconciliation runs near timer expiration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Type)]
pub struct ChampSelectReconciledPayload {
    pub champ_selected: ChampSelectedPayload,
    #[serde(default)]
    pub build: Option<crate::models::ChampionBuild>,
    pub champion_diverged: bool,
}
