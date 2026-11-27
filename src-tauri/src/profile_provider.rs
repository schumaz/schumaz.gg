use crate::lcu_connector::LcuConnection;
use crate::models::{MatchHistoryPage, MatchScoreboardDetail, PlayerProfile, SummonerInfo};

pub use schumaz_domain::profile_provider::*;

impl ProfileLcuClient for LcuConnection {
    async fn get_current_summoner_with_puuid(&self) -> Result<(SummonerInfo, Option<String>), String> {
        LcuConnection::get_current_summoner_with_puuid(self).await
    }

    async fn get_current_summoner(&self) -> Result<SummonerInfo, String> {
        LcuConnection::get_current_summoner(self).await
    }

    async fn get_summoner_profile(&self) -> Result<serde_json::Value, String> {
        LcuConnection::get_summoner_profile(self).await
    }

    async fn get_champion_masteries(
        &self,
        summoner_id: u64,
        puuid: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        LcuConnection::get_champion_masteries(self, summoner_id, puuid).await
    }

    async fn get_ranked_stats(&self) -> Result<serde_json::Value, String> {
        LcuConnection::get_ranked_stats(self).await
    }

    async fn get_match_history(&self) -> Result<serde_json::Value, String> {
        LcuConnection::get_match_history(self).await
    }

    async fn get_match_history_page(
        &self,
        beg_index: u32,
        end_index: u32,
    ) -> Result<serde_json::Value, String> {
        LcuConnection::get_match_history_page(self, beg_index, end_index).await
    }

    async fn get_match_details(&self, game_id: u64) -> Result<serde_json::Value, String> {
        LcuConnection::get_match_details(self, game_id).await
    }
}

/// Connects to the local LCU and loads the current player profile if connected.
pub async fn fetch_live_player_profile(force_refresh: bool) -> Option<PlayerProfile> {
    let conn = LcuConnection::connect().ok()?;
    load_player_profile_with_cache(&conn, force_refresh)
        .await
        .ok()
}

/// Connects to the local LCU and returns the requested paginated match history window.
pub async fn fetch_live_match_history_page(
    beg_index: u32,
    count: Option<u32>,
) -> Result<MatchHistoryPage, String> {
    let conn = LcuConnection::connect()?;
    load_match_history_page(&conn, beg_index, count).await
}

/// Connects to the local LCU and returns the full match scoreboard for `game_id`.
pub async fn fetch_live_match_scoreboard_detail(
    game_id: u64,
) -> Result<MatchScoreboardDetail, String> {
    let conn = LcuConnection::connect()?;
    load_match_scoreboard_detail(&conn, game_id).await
}
