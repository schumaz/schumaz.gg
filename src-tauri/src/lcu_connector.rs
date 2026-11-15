//! Module 1: Connection and Authentication with the LCU (LeagueClientUx.exe)
//! Handles locating the client, authenticating via Basic Auth, and performing REST calls.

pub mod client;
pub mod endpoints;
pub mod missions_parser;
pub mod missions_runner;
pub mod missions_tracker;
pub mod monitor;

pub use crate::models::{LcuStatus, SummonerInfo};
pub use client::{ConnectedLcuClient, DefaultRequestClient, LcuConnection};
pub use endpoints::*;
pub use missions_parser::*;
pub use missions_runner::*;
pub use missions_tracker::*;
pub use monitor::*;
