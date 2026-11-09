//! Module 4: Client Injection (LCU)
//! Handles applying rune pages, summoner spells, and item sets back to the local LCU client.

pub mod injection;
pub mod payloads;
pub mod preferences;
pub mod selections;
pub mod spell_rules;

pub use injection::*;
pub use payloads::*;
pub use preferences::*;
pub use selections::*;
pub use spell_rules::*;
