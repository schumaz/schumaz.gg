//! Module 2: Champion Select and Queue Detection via WebSocket
//! Handles real-time monitoring of LoL client lobby and draft states without polling.

pub mod actor;
pub mod filter_reinject;
pub mod gameflow;
pub mod lobby_actions;
pub mod lobby_handler;
pub mod polling;
pub mod reconcile;
pub mod session_handler;
mod subscription;
mod ws_client;

pub use filter_reinject::*;
pub use lobby_actions::*;
pub use polling::*;
pub use reconcile::*;
pub use schumaz_domain::draft_session::*;
pub use session_handler::*;
pub use subscription::start_ws_listener;
