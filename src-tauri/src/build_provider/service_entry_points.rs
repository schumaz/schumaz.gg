use super::analytics::{
    OPGG_MCP_ENDPOINT, OPGG_REST_API_BASE, fetch_aram_mayhem_augments_for_locale,
    fetch_aram_tier_list, fetch_ranked_tier_list, shared_http_client,
};
use super::ddragon::get_or_init_ddragon_cache;
use super::models::{ChampionBuild, DataDragonCache};
use super::service::get_recommended_build_with_filters_and_endpoints_with_dir;
use super::service_helpers::FastSpellRecommendation;
use crate::models::{
    AramAugmentEntry, AramChampionRank, AramMode, QUEUE_SOLO_DUO, RankedTierListResponse,
};

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
    fetch_aram_mayhem_augments_for_locale(http_client, champion_key, locale).await
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
