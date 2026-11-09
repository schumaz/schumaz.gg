//! Module 2: Champion Select and Queue Detection via WebSocket
//! Handles real-time monitoring of LoL client lobby and draft states without polling.

pub mod extract;
pub mod polling;

pub use extract::*;
pub use polling::*;
