use serde::{Deserialize, Serialize};
use specta::Type;

pub const QUEUE_SOLO_DUO: i32 = 420;
pub const QUEUE_FLEX: i32 = 440;
pub const QUEUE_ARAM: i32 = 450;
pub const QUEUE_ARAM_MAYHEM: i32 = 2400;
pub const QUEUE_ARENA: i32 = 1700;
pub const QUEUE_ARENA_1710: i32 = 1710;
pub const QUEUE_ARENA_1740: i32 = 1740;
pub const QUEUE_ARENA_1750: i32 = 1750;
pub const QUEUE_NORMAL_DRAFT: i32 = 400;
pub const QUEUE_SWIFTPLAY: i32 = 480;
pub const QUEUE_QUICKPLAY: i32 = 490;
pub const QUEUE_CLASH: i32 = 700;
pub const QUEUE_PRACTICE_TOOL: i32 = -1;
pub const MAP_ARENA: i32 = 30;

/// Returns `true` for any known Riot Arena (`CHERRY` / 3x6) queue ID (`1700`, `1710`, `1740`, `1750`).
pub const fn is_arena_queue(queue_id: i32) -> bool {
    matches!(
        queue_id,
        QUEUE_ARENA | QUEUE_ARENA_1710 | QUEUE_ARENA_1740 | QUEUE_ARENA_1750
    )
}

/// Returns `true` for Riot Arena (`CHERRY` / `ARENA` / `TGR` / `ARENA 3X6`) mode or queue type tokens.
pub fn is_arena_game_mode(mode_or_type: &str) -> bool {
    let upper = mode_or_type.trim().to_ascii_uppercase();
    upper == "CHERRY" || upper == "ARENA" || upper == "TGR" || upper.contains("ARENA")
}

/// Returns `true` when the queue uses pre-selection slots inside `/lol-lobby/v2/lobby`
/// (Swiftplay `480` / Quickplay `490`) instead of `/lol-champ-select/v1/session`.
pub const fn is_lobby_preselect_queue(queue_id: i32) -> bool {
    matches!(queue_id, QUEUE_SWIFTPLAY | QUEUE_QUICKPLAY)
}

/// Returns `true` for Riot Clash (`700`) tournament queue.
pub const fn is_clash_queue(queue_id: i32) -> bool {
    queue_id == QUEUE_CLASH
}

/// Returns `true` for Swiftplay mode/queue type tokens.
pub fn is_swiftplay_game_mode(mode_or_type: &str) -> bool {
    let upper = mode_or_type.trim().to_ascii_uppercase();
    upper == "SWIFTPLAY" || upper.contains("SWIFTPLAY")
}

/// Normalizes queue IDs for build lookup and caching:
/// - Clash (`700`) and Practice Tool (`-1` / `0`) strictly redirect to Ranked Solo/Duo (`420`)
///   so Clash never maintains an independent build database/cache partition.
/// - Swiftplay (`480`), Quickplay (`490`), ARAM, ARAM Mayhem, and Arena retain their own IDs.
pub const fn canonical_build_queue_id(queue_id: i32) -> i32 {
    if queue_id <= 0 || queue_id == QUEUE_CLASH {
        QUEUE_SOLO_DUO
    } else {
        queue_id
    }
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
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Type)]
pub enum QueueMode {
    SummonersRift,
    Aram,
    AramMayhem,
    Arena,
    PracticeTool,
    #[default]
    Other,
}

impl From<i32> for QueueMode {
    fn from(queue_id: i32) -> Self {
        match queue_id {
            QUEUE_ARAM => QueueMode::Aram,
            QUEUE_ARAM_MAYHEM => QueueMode::AramMayhem,
            QUEUE_ARENA | QUEUE_ARENA_1710 | QUEUE_ARENA_1740 | QUEUE_ARENA_1750 => {
                QueueMode::Arena
            }
            QUEUE_PRACTICE_TOOL => QueueMode::PracticeTool,
            QUEUE_SOLO_DUO
            | QUEUE_FLEX
            | QUEUE_NORMAL_DRAFT
            | QUEUE_SWIFTPLAY
            | QUEUE_QUICKPLAY
            | QUEUE_CLASH => QueueMode::SummonersRift,
            _ => QueueMode::Other,
        }
    }
}

/// A champion/position slot pre-selected directly in a Swiftplay/Quickplay LCU lobby.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Type)]
pub struct LobbyPreselectSlot {
    pub slot_index: u32,
    pub champion_id: i32,
    #[serde(default)]
    pub champion_name: String,
    #[serde(default)]
    pub champion_icon_url: String,
    #[serde(default)]
    pub assigned_position: Option<String>,
}

/// Payload emitted to the Tauri frontend when the user selects or locks in a champion.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Type)]
pub struct ChampSelectedPayload {
    pub champion_id: i32,
    pub queue_id: i32,
    pub queue_mode: QueueMode,
    pub locked_in: bool,
    #[serde(default)]
    pub assigned_position: Option<String>,
    #[serde(default)]
    pub is_lobby_preselect: bool,
    #[serde(default)]
    pub active_slot_index: Option<u32>,
    #[serde(default)]
    pub lobby_slots: Vec<LobbyPreselectSlot>,
}

/// Payload emitted when final Champ Select reconciliation runs near timer expiration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Type)]
pub struct ChampSelectReconciledPayload {
    pub champ_selected: ChampSelectedPayload,
    #[serde(default)]
    pub build: Option<crate::models::ChampionBuild>,
    pub champion_diverged: bool,
}

/// Consolidated LCU bootstrap snapshot returned on frontend mount so an already-running
/// Champ Select session and its compatible cached build hydrate immediately without missing events.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Type)]
pub struct ActiveChampSelectSnapshot {
    #[serde(default)]
    pub gameflow_phase: Option<String>,
    #[serde(default)]
    pub champ_selected: Option<ChampSelectedPayload>,
    #[serde(default)]
    pub build: Option<crate::models::ChampionBuild>,
}
