use super::analytics::{
    ArenaBuildQuery, fetch_aram_mayhem_augments_for_locale, fetch_aram_tier_list,
    fetch_arena_champion_build, fetch_arena_champion_build_with_query, fetch_arena_tier_list,
    fetch_ranked_tier_list, shared_http_client,
};
use super::ddragon::{
    enrich_build_with_ddragon_for_queue, get_or_init_ddragon_cache,
    get_or_init_ddragon_cache_for_locale,
};
use super::fallback::build_fallback_champion_build_with_ddragon;
use super::models::{ChampionBuild, DataDragonCache};
use super::query::BuildQuery;
use super::service::resolve_recommended_build;
use super::service_helpers::FastSpellRecommendation;
use crate::models::{
    AramAugmentEntry, AramChampionRank, AramMode, ArenaTierListResponse, QUEUE_ARAM,
    QUEUE_ARAM_MAYHEM, QUEUE_SOLO_DUO, RankedTierListResponse,
};
use std::time::Instant;

pub(super) async fn resolve_arena_build_or_fallback(
    client: &reqwest::Client,
    query: &BuildQuery<'_>,
    champ_name: &str,
    ddragon_cache: &DataDragonCache,
    started: Instant,
    detail: &str,
) -> ChampionBuild {
    let queue_id = query.canonical_queue_id();
    let arena_dir = query.arena_dir();
    let now_unix = crate::storage::now_unix_secs();
    match fetch_arena_champion_build_with_query(
        client,
        &ArenaBuildQuery {
            rest_base: query.rest_base,
            champion_id: query.champion_id,
            patch: query.patch,
            patch_48h_protection: query.patch_48h_protection,
            now_unix,
            locale: query.locale,
            arena_dir,
        },
        ddragon_cache,
    )
    .await
    {
        Ok(build) => build,
        Err(_) => {
            let mut fallback = build_fallback_champion_build_with_ddragon(
                query.champion_id,
                champ_name,
                queue_id,
                Some(ddragon_cache),
            );
            enrich_build_with_ddragon_for_queue(&mut fallback, ddragon_cache, queue_id);
            crate::logging::record_data_resolution(
                "arena_build",
                "stale_fallback",
                started.elapsed().as_millis(),
                detail,
            );
            fallback
        }
    }
}

/// Resolves a recommended champion build from a `BuildQuery`, initializing DataDragon for `query.locale`.
pub async fn resolve_recommended_build_with_query(
    client: &reqwest::Client,
    query: &BuildQuery<'_>,
    fast_spells_tx: Option<tokio::sync::oneshot::Sender<FastSpellRecommendation>>,
) -> Result<ChampionBuild, String> {
    let ddragon_cache = get_or_init_ddragon_cache_for_locale(client, query.locale).await?;
    resolve_recommended_build(client, query, &ddragon_cache, fast_spells_tx).await
}

pub async fn get_recommended_build_with_filters(
    client: &reqwest::Client,
    champion_id: i32,
    queue_id: i32,
    position: Option<&str>,
    tier: Option<&str>,
    region: Option<&str>,
    patch: Option<&str>,
) -> Result<ChampionBuild, String> {
    let query = BuildQuery::new(champion_id, queue_id).with_filters(position, tier, region, patch);
    resolve_recommended_build_with_query(client, &query, None).await
}

/// Production entry point to obtain recommended build for champion.
pub async fn get_recommended_build(
    client: &reqwest::Client,
    champion_id: i32,
    queue_id: i32,
) -> Result<ChampionBuild, String> {
    let query = BuildQuery::new(champion_id, queue_id);
    resolve_recommended_build_with_query(client, &query, None).await
}

/// Domain service entry point for IPC commands requesting a champion build without filters.
pub async fn get_recommended_build_for_champion_service(
    champion_id: i32,
    queue_id: Option<i32>,
) -> Result<ChampionBuild, String> {
    let http_client = shared_http_client();
    let q_id = queue_id.unwrap_or(QUEUE_SOLO_DUO);
    get_recommended_build(http_client, champion_id, q_id).await
}

/// Domain service entry point for fetching ARAM / ARAM Mayhem tier list.
pub async fn get_aram_tier_list_service(mode: AramMode) -> Result<Vec<AramChampionRank>, String> {
    let http_client = shared_http_client();
    let ddragon = get_or_init_ddragon_cache(http_client).await?;
    fetch_aram_tier_list(http_client, mode, &ddragon).await
}

/// Domain service entry point for fetching ARAM Mayhem augments for the active locale.
pub async fn get_aram_mayhem_augments_service(
    champion_key: &str,
) -> Result<Vec<AramAugmentEntry>, String> {
    let http_client = shared_http_client();
    let locale = crate::storage::get_active_locale();
    let ddragon = get_or_init_ddragon_cache_for_locale(http_client, locale).await?;
    fetch_aram_mayhem_augments_for_locale(http_client, champion_key, locale, &ddragon).await
}

/// Domain service entry point for fetching the Ranked Solo/Duo tier list.
pub async fn get_ranked_tier_list_service(
    region: Option<&str>,
    tier: Option<&str>,
    patch: Option<&str>,
) -> Result<RankedTierListResponse, String> {
    let http_client = shared_http_client();
    let ddragon = get_or_init_ddragon_cache(http_client).await?;
    fetch_ranked_tier_list(http_client, region, tier, patch, &ddragon).await
}

/// Domain service entry point for fetching a read-only Ranked champion build preview.
pub async fn get_ranked_champion_build_service(
    champion_id: i32,
    position: Option<&str>,
    tier: Option<&str>,
    region: Option<&str>,
    patch: Option<&str>,
) -> Result<ChampionBuild, String> {
    let http_client = shared_http_client();
    get_recommended_build_with_filters(
        http_client,
        champion_id,
        QUEUE_SOLO_DUO,
        position,
        tier,
        region,
        patch,
    )
    .await
}

/// Domain service entry point for fetching the Arena (`CHERRY`) tier list.
pub async fn get_arena_tier_list_service(
    patch: Option<&str>,
) -> Result<ArenaTierListResponse, String> {
    let http_client = shared_http_client();
    let ddragon = get_or_init_ddragon_cache(http_client).await?;
    fetch_arena_tier_list(http_client, patch, &ddragon).await
}

/// Domain service entry point for fetching a read-only Arena (`CHERRY`) champion build preview.
pub async fn get_arena_champion_build_service(
    champion_id: i32,
    patch: Option<&str>,
) -> Result<ChampionBuild, String> {
    let http_client = shared_http_client();
    let ddragon = get_or_init_ddragon_cache(http_client).await?;
    fetch_arena_champion_build(http_client, champion_id, patch, &ddragon).await
}

/// Maps an ARAM sub-mode to its corresponding Riot queue ID.
pub const fn aram_mode_to_queue_id(mode: AramMode) -> i32 {
    match mode {
        AramMode::Aram => QUEUE_ARAM,
        AramMode::AramMayhem => QUEUE_ARAM_MAYHEM,
    }
}

/// Domain service entry point for fetching a read-only ARAM or ARAM Mayhem champion build preview.
pub async fn get_aram_champion_build_service(
    champion_id: i32,
    mode: AramMode,
    tier: Option<&str>,
    region: Option<&str>,
    patch: Option<&str>,
) -> Result<ChampionBuild, String> {
    let http_client = shared_http_client();
    let queue_id = aram_mode_to_queue_id(mode);
    get_recommended_build_with_filters(http_client, champion_id, queue_id, None, tier, region, patch)
        .await
}
