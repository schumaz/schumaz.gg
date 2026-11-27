use super::super::items::set_dynamic_item_classifications;
use super::super::models::DataDragonCache;
use super::augments::fetch_arena_augments_catalog;
use super::cache::DataDragonDiskBundle;
use super::catalog_parsers::{parse_all_ddragon_champion_profiles, parse_ddragon_rune_trees};
use super::parsers::{
    parse_all_ddragon_champion_abilities, parse_all_ddragon_champions,
    parse_ddragon_item_classifications, parse_ddragon_runes, parse_ddragon_summoners,
    parse_latest_ddragon_version,
};
use super::tooltip_parser::parse_ddragon_items;
use crate::models::AppLocale;
use std::collections::HashMap;
use std::time::Instant;

pub(super) async fn fetch_ddragon_versions(
    client: &reqwest::Client,
    base_url: &str,
) -> Result<String, String> {
    let versions_url = format!("{base_url}/api/versions.json");
    let started = Instant::now();
    let res = client.get(&versions_url).send().await.map_err(|e| {
        crate::logging::record_http_network_error_log(
            "ddragon",
            started.elapsed().as_millis(),
            &e.to_string(),
        );
        format!("Failed to fetch versions.json: {e}")
    })?;
    crate::logging::record_http_status_log(
        "ddragon",
        res.status().as_u16(),
        started.elapsed().as_millis(),
    );
    let json = res
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse versions.json: {e}"))?;
    parse_latest_ddragon_version(&json)
}

pub(super) async fn load_ddragon_bundle_for_version(
    client: &reqwest::Client,
    base_url: &str,
    version: &str,
    locale: AppLocale,
) -> Result<DataDragonDiskBundle, String> {
    let dd_locale = locale.ddragon_code();

    let champion_full_url = format!("{base_url}/cdn/{version}/data/{dd_locale}/championFull.json");
    let champions_url = format!("{base_url}/cdn/{version}/data/{dd_locale}/champion.json");
    let runes_url = format!("{base_url}/cdn/{version}/data/{dd_locale}/runesReforged.json");
    let items_url = format!("{base_url}/cdn/{version}/data/{dd_locale}/item.json");
    let summoners_url = format!("{base_url}/cdn/{version}/data/{dd_locale}/summoner.json");

    let champions_fut = async {
        let t0_full = Instant::now();
        if let Ok(res) = client.get(&champion_full_url).send().await {
            crate::logging::record_http_status_log(
                "ddragon",
                res.status().as_u16(),
                t0_full.elapsed().as_millis(),
            );
            if res.status().is_success()
                && let Ok(full_json) = res.json::<serde_json::Value>().await
                && full_json
                    .get("data")
                    .and_then(|d| d.as_object())
                    .is_some_and(|m| !m.is_empty())
            {
                return Ok(full_json);
            }
        }

        let t0 = Instant::now();
        let res = client.get(&champions_url).send().await.map_err(|e| {
            crate::logging::record_http_network_error_log(
                "ddragon",
                t0.elapsed().as_millis(),
                &e.to_string(),
            );
            format!("Failed to fetch champion.json: {e}")
        })?;
        crate::logging::record_http_status_log(
            "ddragon",
            res.status().as_u16(),
            t0.elapsed().as_millis(),
        );
        res.json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Failed to parse champion.json: {e}"))
    };

    let runes_fut = async {
        let t0 = Instant::now();
        let res = client.get(&runes_url).send().await.map_err(|e| {
            crate::logging::record_http_network_error_log(
                "ddragon",
                t0.elapsed().as_millis(),
                &e.to_string(),
            );
            format!("Failed to fetch runesReforged.json: {e}")
        })?;
        crate::logging::record_http_status_log(
            "ddragon",
            res.status().as_u16(),
            t0.elapsed().as_millis(),
        );
        res.json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Failed to parse runesReforged.json: {e}"))
    };

    let items_fut = async {
        if let Ok(res) = client.get(&items_url).send().await
            && res.status().is_success()
            && let Ok(items_res) = res.json::<serde_json::Value>().await
        {
            let classification = parse_ddragon_item_classifications(&items_res);
            set_dynamic_item_classifications(classification.clone());
            let items = parse_ddragon_items(&items_res);
            return (Some(classification), items);
        }
        (None, HashMap::new())
    };

    let summoners_fut = async {
        if let Ok(res) = client.get(&summoners_url).send().await
            && res.status().is_success()
            && let Ok(summoners_res) = res.json::<serde_json::Value>().await
            && let Ok(parsed_spells) = parse_ddragon_summoners(&summoners_res, version)
        {
            return parsed_spells;
        }
        HashMap::new()
    };

    let arena_augments_fut = fetch_arena_augments_catalog(client, base_url, locale);

    let (champions_res, runes_res, (item_classifications, items), spells, arena_augments) = tokio::join!(
        champions_fut,
        runes_fut,
        items_fut,
        summoners_fut,
        arena_augments_fut
    );

    let champions_json = champions_res?;
    let champions = parse_all_ddragon_champions(&champions_json, version)?;
    let champion_abilities = parse_all_ddragon_champion_abilities(&champions_json, version);
    let champion_profiles = parse_all_ddragon_champion_profiles(&champions_json);
    let runes_json = runes_res?;
    let runes = parse_ddragon_runes(&runes_json)?;
    let rune_trees = parse_ddragon_rune_trees(&runes_json);
    let cached_classifications = item_classifications.clone().unwrap_or_default();

    Ok(DataDragonDiskBundle {
        ddragon: DataDragonCache {
            version: version.to_string(),
            champions,
            runes,
            spells,
            items,
            arena_augments,
            champion_abilities,
            champion_profiles,
            rune_trees,
            item_classifications: cached_classifications,
        },
        item_classifications,
    })
}
