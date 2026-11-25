//! Module 4: Client Injection (LCU)
//! Handles applying rune pages, summoner spells, and item sets back to the local LCU client.

pub mod injection;
pub mod injection_state;
pub mod lobby_injection;
pub mod lobby_payloads;
pub mod payloads;
pub mod preferences;
pub mod selections;
pub mod service;
pub mod spell_rules;

pub use injection::*;
pub use injection_state::*;
pub use lobby_injection::*;
pub use lobby_payloads::*;
pub use payloads::*;
pub use preferences::*;
pub use selections::*;
pub use service::*;
pub use spell_rules::*;
