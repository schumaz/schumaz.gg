//! Module 2: Champion Select and Queue Detection via WebSocket
//! Handles real-time monitoring of LoL client lobby and draft states without polling.

pub mod coordinator;
pub mod extract;
pub mod gameflow;
pub mod polling;
pub mod reconcile;
pub mod reconcile_state;
pub mod session_handler;
mod subscription;

pub use coordinator::*;
pub use extract::*;
pub use gameflow::*;
pub use polling::*;
pub use reconcile::*;
pub use reconcile_state::*;
pub use session_handler::*;
