//! Module 4: Client Injection (LCU)
//! Handles applying rune pages, summoner spells, and item sets back to the local LCU client.

pub mod live_injection;
pub mod live_lobby;
pub mod service;

pub use live_injection::*;
pub use live_lobby::*;
pub use schumaz_domain::lcu_injector::*;
pub use service::*;
