use super::analytics::{
    OPGG_MCP_ENDPOINT, OPGG_REST_API_BASE, fetch_aram_mayhem_augments_for_locale,
    fetch_aram_tier_list, fetch_arena_champion_build, fetch_arena_champion_build_with_base_and_dir,
    fetch_arena_tier_list, fetch_ranked_tier_list, shared_http_client,
};
use super::ddragon::{
    enrich_build_with_ddragon_for_queue, get_or_init_ddragon_cache,
    get_or_init_ddragon_cache_for_locale,
};
use super::fallback::build_fallback_champion_build_with_ddragon;
use super::models::{ChampionBuild, DataDragonCache};
use super::service::get_recommended_build_with_filters_and_endpoints_with_dir;
use super::service_helpers::FastSpellRecommendation;
use crate::models::{
    AppLocale, AramAugmentEntry, AramChampionRank, AramMode, ArenaTierListResponse, QUEUE_ARAM,
    QUEUE_ARAM_MAYHEM, QUEUE_SOLO_DUO, RankedTierListResponse,
};
use std::path::Path;
use std::time::Instant;

#[allow(clippy::too_many_arguments)]
pub(super) async fn resolve_arena_build_or_fallback(
    client: &reqwest::Client,
    rest_base: &str,
    champion_id: i32,
    champ_name: &str,
    queue_id: i32,
    patch: Option<&str>,
    ddragon_cache: &DataDragonCache,
    patch_48h_protection: bool,
    active_locale: AppLocale,
    builds_dir: Option<&Path>,
    started: Instant,
    detail: &str,
) -> ChampionBuild {
    let arena_dir = if rest_base == OPGG_REST_API_BASE {
        crate::storage::global_paths()
            .map(|p| p.opgg_arena_dir.as_path())
            .or(builds_dir)
    } else {
        builds_dir
    };
    let now_unix = crate::storage::now_unix_secs();
    match fetch_arena_champion_build_with_base_and_dir(
        client,
        rest_base,
        champion_id,
        patch,
        ddragon_cache,
        patch_48h_protection,
        now_unix,
        active_locale,
        arena_dir,
    )
    .await
    {
        Ok(build) => build,
        Err(_) => {
            let mut fallback = build_fallback_champion_build_with_ddragon(
                champion_id,
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

pub async fn get_recommended_build_with_filters(
    client: &reqwest::Client,
    champion_id: i32,
    queue_id: i32,
    position: Option<&str>,
    tier: Option<&str>,
    region: Option<&str>,
    patch: Option<&str>,
) -> Result<ChampionBuild, String> {
    get_recommended_build_with_filters_and_fast_spells(
        client,
        champion_id,
        queue_id,
        position,
        tier,
        region,
        patch,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn get_recommended_build_with_filters_and_fast_spells(
    client: &reqwest::Client,
    champion_id: i32,
    queue_id: i32,
    position: Option<&str>,
    tier: Option<&str>,
    region: Option<&str>,
    patch: Option<&str>,
    fast_spells_tx: Option<tokio::sync::oneshot::Sender<FastSpellRecommendation>>,
) -> Result<ChampionBuild, String> {
    let ddragon_cache = get_or_init_ddragon_cache(client).await?;
    get_recommended_build_with_filters_and_endpoints_internal(
        client,
        champion_id,
        queue_id,
        position,
        tier,
        region,
        patch,
        &ddragon_cache,
        OPGG_REST_API_BASE,
        OPGG_MCP_ENDPOINT,
        fast_spells_tx,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn get_recommended_build_with_filters_and_endpoints(
    client: &reqwest::Client,
    champion_id: i32,
    queue_id: i32,
    position: Option<&str>,
    tier: Option<&str>,
    region: Option<&str>,
    patch: Option<&str>,
    ddragon_cache: &DataDragonCache,
    rest_base: &str,
    mcp_endpoint: &str,
) -> Result<ChampionBuild, String> {
    get_recommended_build_with_filters_and_endpoints_internal(
        client,
        champion_id,
        queue_id,
        position,
        tier,
        region,
        patch,
        ddragon_cache,
        rest_base,
        mcp_endpoint,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn get_recommended_build_with_filters_and_endpoints_internal(
    client: &reqwest::Client,
    champion_id: i32,
    queue_id: i32,
    position: Option<&str>,
    tier: Option<&str>,
    region: Option<&str>,
    patch: Option<&str>,
    ddragon_cache: &DataDragonCache,
    rest_base: &str,
    mcp_endpoint: &str,
    fast_spells_tx: Option<tokio::sync::oneshot::Sender<FastSpellRecommendation>>,
) -> Result<ChampionBuild, String> {
    let prefs = crate::lcu_injector::get_user_preferences();
    get_recommended_build_with_filters_and_endpoints_with_protection(
        client,
        champion_id,
        queue_id,
        position,
        tier,
        region,
        patch,
        ddragon_cache,
        rest_base,
        mcp_endpoint,
        fast_spells_tx,
        prefs.patch_48h_protection,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn get_recommended_build_with_filters_and_endpoints_with_protection(
    client: &reqwest::Client,
    champion_id: i32,
    queue_id: i32,
    position: Option<&str>,
    tier: Option<&str>,
    region: Option<&str>,
    patch: Option<&str>,
    ddragon_cache: &DataDragonCache,
    rest_base: &str,
    mcp_endpoint: &str,
    fast_spells_tx: Option<tokio::sync::oneshot::Sender<FastSpellRecommendation>>,
    patch_48h_protection: bool,
) -> Result<ChampionBuild, String> {
    let builds_dir = if rest_base == OPGG_REST_API_BASE {
        crate::storage::global_paths().map(|p| p.opgg_builds_dir.as_path())
    } else {
        None
    };
    get_recommended_build_with_filters_and_endpoints_with_dir(
        client,
        champion_id,
        queue_id,
        position,
        tier,
        region,
        patch,
        ddragon_cache,
        rest_base,
        mcp_endpoint,
        fast_spells_tx,
        patch_48h_protection,
        builds_dir,
    )
    .await
}

/// Production entry point to obtain recommended build for champion.
pub async fn get_recommended_build(
    client: &reqwest::Client,
    champion_id: i32,
    queue_id: i32,
) -> Result<ChampionBuild, String> {
    get_recommended_build_with_filters(client, champion_id, queue_id, None, None, None, None).await
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
    let locale = crate::lcu_injector::get_active_locale();
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
