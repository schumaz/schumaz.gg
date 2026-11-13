use serde::{Deserialize, Serialize};
use specta::Type;

pub const QUEUE_SOLO_DUO: i32 = 420;
pub const QUEUE_FLEX: i32 = 440;
pub const QUEUE_ARAM: i32 = 450;
pub const QUEUE_ARAM_MAYHEM: i32 = 2400;
pub const QUEUE_NORMAL_DRAFT: i32 = 400;
pub const QUEUE_QUICKPLAY: i32 = 490;
pub const QUEUE_PRACTICE_TOOL: i32 = -1;

/// Game queue classifications.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Type)]
pub enum QueueMode {
    SummonersRift,
    Aram,
    AramMayhem,
    PracticeTool,
    Other,
}

impl From<i32> for QueueMode {
    fn from(queue_id: i32) -> Self {
        match queue_id {
            QUEUE_ARAM => QueueMode::Aram,
            QUEUE_ARAM_MAYHEM => QueueMode::AramMayhem,
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
