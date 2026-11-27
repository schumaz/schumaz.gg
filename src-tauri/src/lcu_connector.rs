//! Module 1: Connection and Authentication with the LCU (LeagueClientUx.exe)
//! Handles locating the client, authenticating via Basic Auth, and performing REST calls.

pub mod client;
pub mod discovery;
pub mod endpoints;
pub mod missions_runner;
pub mod monitor;

pub use crate::models::{LcuStatus, SummonerInfo};
pub use client::{ConnectedLcuClient, DefaultRequestClient, LcuConnection};
pub use discovery::{
    LcuCredentials, get_cached_lcu_credentials, get_cached_or_discover_lcu_credentials,
    invalidate_lcu_credentials, lcu_tls_client_config, shared_request_client,
};
pub use endpoints::*;
pub use missions_runner::*;
pub use monitor::*;
pub use schumaz_domain::lcu_parsers::{
    discovery_manifests, discovery_parsers, endpoints as parser_endpoints, missions_parser,
    missions_tracker,
};
pub use schumaz_domain::lcu_parsers::{missions_parser::*, missions_tracker::*};
