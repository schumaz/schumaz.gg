use crate::build_provider::models::DataDragonCache;
use crate::models::{AramAugmentEntry, AramChampionRank, AramMode};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

pub const OPGG_ARAM_REST_URL: &str = "https://lol-api-champion.op.gg/api/global/champions/aram";
pub const OPGG_ARAM_MAYHEM_WEB_URL: &str = "https://op.gg/lol/modes/aram-mayhem";
pub const OPGG_ARAM_MAYHEM_AUGMENTS_WEB_BASE: &str = "https://op.gg/pt/lol/modes/aram-mayhem";

const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";

const ARAM_CACHE_TTL: Duration = Duration::from_secs(900);

type TierCacheMap = HashMap<String, (Instant, Vec<AramChampionRank>)>;
type AugmentsCacheMap = HashMap<String, (Instant, Vec<AramAugmentEntry>)>;

static ARAM_TIER_CACHE: OnceLock<RwLock<TierCacheMap>> = OnceLock::new();
static ARAM_AUGMENTS_CACHE: OnceLock<RwLock<AugmentsCacheMap>> = OnceLock::new();

fn get_tier_cache() -> &'static RwLock<TierCacheMap> {
    ARAM_TIER_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn get_augments_cache() -> &'static RwLock<AugmentsCacheMap> {
    ARAM_AUGMENTS_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn normalize_opgg_champion_key(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

pub fn clean_augment_description(raw: &str) -> String {
    let normalized_breaks = raw
        .replace("<br />", " ")
        .replace("<br/>", " ")
        .replace("<br>", " ")
        .replace("<BR />", " ")
        .replace("<BR/>", " ")
        .replace("<BR>", " ");

    let mut out = String::with_capacity(normalized_breaks.len());
    let mut in_tag = false;
    for ch in normalized_breaks.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }

    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn round_pct(val: f64) -> f64 {
    let pct = if val <= 1.0 { val * 100.0 } else { val };
    (pct * 10.0).round() / 10.0
}

#[derive(Debug, Deserialize)]
struct RestAramChampionResponse {
    data: Vec<RestAramChampionItem>,
}

#[derive(Debug, Deserialize)]
struct RestAramChampionItem {
    id: i32,
    #[serde(default)]
    average_stats: Option<RestAramAverageStats>,
}

#[derive(Debug, Deserialize)]
struct RestAramAverageStats {
    #[serde(default)]
    win_rate: Option<f64>,
    #[serde(default)]
    pick_rate: Option<f64>,
    #[serde(default)]
    tier: Option<i32>,
    #[serde(default)]
    rank: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct WebMayhemChampionItem {
    id: i32,
    key: String,
    name: String,
    image_url: String,
    #[serde(default)]
    tier: i32,
    #[serde(default)]
    rank: i32,
}

#[derive(Debug, Deserialize)]
struct WebMayhemAugmentItem {
    id: i32,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    tier: i32,
    #[serde(default)]
    rarity: Option<i32>,
    #[serde(default)]
    performance: f64,
    #[serde(default)]
    popular: f64,
    #[serde(rename = "smallIcon", default)]
    small_icon: Option<String>,
    #[serde(rename = "largeIcon", default)]
    large_icon: Option<String>,
    #[serde(default)]
    desc: Option<String>,
}

/// Decodes all `self.__next_f.push([1,"..."])` string payloads in Next.js App Router HTML.
fn decode_next_f_stream(html: &str) -> String {
    const PREFIX: &str = "self.__next_f.push([1,";
    let mut combined = String::new();
    let mut cursor = 0;

    while let Some(rel_idx) = html[cursor..].find(PREFIX) {
        let start = cursor + rel_idx + PREFIX.len();
        let bytes = html.as_bytes();
        if start >= bytes.len() || bytes[start] != b'"' {
            cursor = start;
            continue;
        }

        let mut i = start + 1;
        let mut escaped = false;
        while i < bytes.len() {
            let b = bytes[i];
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                break;
            }
            i += 1;
        }

        if i < bytes.len() && bytes[i] == b'"' {
            let literal = &html[start..=i];
            if let Ok(decoded) = serde_json::from_str::<String>(literal) {
                combined.push_str(&decoded);
            }
            cursor = i + 1;
        } else {
            break;
        }
    }

    combined
}

/// Extracts a balanced JSON `[...]` slice starting at `array_start_idx`.
fn extract_balanced_json_array(source: &str, array_start_idx: usize) -> Option<&str> {
    let bytes = source.as_bytes();
    if bytes.get(array_start_idx) != Some(&b'[') {
        return None;
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, &b) in bytes[array_start_idx..].iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
        } else {
            match b {
                b'"' => in_string = true,
                b'[' => depth += 1,
                b']' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(&source[array_start_idx..=array_start_idx + offset]);
                    }
                }
                _ => {}
            }
        }
    }

    None
}

pub fn parse_classic_aram_champions(
    json: &serde_json::Value,
    ddragon: &DataDragonCache,
) -> Result<Vec<AramChampionRank>, String> {
    let parsed: RestAramChampionResponse = serde_json::from_value(json.clone())
        .map_err(|e| format!("Failed to parse classic ARAM response: {}", e))?;

    let mut champions: Vec<AramChampionRank> = parsed
        .data
        .into_iter()
        .filter_map(|item| {
            let stats = item.average_stats?;
            let meta = ddragon.champions.get(&item.id);
            let name = meta
                .map(|m| m.display_name.clone())
                .unwrap_or_else(|| format!("Champion {}", item.id));
            let key = meta
                .map(|m| normalize_opgg_champion_key(&m.key_name))
                .unwrap_or_else(|| item.id.to_string());
            let icon_url = meta.map(|m| m.icon_url.clone()).unwrap_or_default();

            Some(AramChampionRank {
                id: item.id,
                key,
                name,
                icon_url,
                tier: stats.tier.unwrap_or(5),
                rank: stats.rank.unwrap_or(999),
                win_rate: stats.win_rate.map(round_pct),
                pick_rate: stats.pick_rate.map(round_pct),
            })
        })
        .collect();

    champions.sort_by(|a, b| {
        let a_valid = if a.rank > 0 { 0 } else { 1 };
        let b_valid = if b.rank > 0 { 0 } else { 1 };
        a_valid
            .cmp(&b_valid)
            .then_with(|| a.rank.cmp(&b.rank))
            .then_with(|| a.tier.cmp(&b.tier))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(champions)
}

pub fn parse_aram_mayhem_champions_html(
    html: &str,
    ddragon: &DataDragonCache,
) -> Result<Vec<AramChampionRank>, String> {
    let rsc = decode_next_f_stream(html);
    let source = if rsc.is_empty() { html } else { &rsc };

    let marker = "\"champions\":[{\"key\":";
    let marker_idx = source
        .find(marker)
        .ok_or_else(|| "ARAM Mayhem champions payload not found in HTML".to_string())?;
    let array_start = marker_idx + "\"champions\":".len();
    let array_json = extract_balanced_json_array(source, array_start)
        .ok_or_else(|| "Malformed ARAM Mayhem champions array".to_string())?;

    let raw_items: Vec<WebMayhemChampionItem> = serde_json::from_str(array_json)
        .map_err(|e| format!("Failed to deserialize ARAM Mayhem champions: {}", e))?;

    let mut champions: Vec<AramChampionRank> = raw_items
        .into_iter()
        .map(|item| {
            let dd_meta = ddragon.champions.get(&item.id);
            let icon_url = if !item.image_url.is_empty() {
                item.image_url
            } else {
                dd_meta.map(|m| m.icon_url.clone()).unwrap_or_default()
            };
            let name = if !item.name.is_empty() {
                item.name
            } else {
                dd_meta
                    .map(|m| m.display_name.clone())
                    .unwrap_or_else(|| format!("Champion {}", item.id))
            };

            AramChampionRank {
                id: item.id,
                key: normalize_opgg_champion_key(&item.key),
                name,
                icon_url,
                // OP.GG Mayhem tier starts at 1 (1 = OP, 2 = Tier 1, 3 = Tier 2, ...).
                // Normalize to 0-based canonical tiers (0 = OP, 1 = Tier 1, ..., 5 = Tier 5).
                tier: item.tier.saturating_sub(1),
                rank: item.rank,
                win_rate: None,
                pick_rate: None,
            }
        })
        .collect();

    champions.sort_by(|a, b| {
        let a_valid = if a.rank > 0 { 0 } else { 1 };
        let b_valid = if b.rank > 0 { 0 } else { 1 };
        a_valid
            .cmp(&b_valid)
            .then_with(|| a.rank.cmp(&b.rank))
            .then_with(|| a.tier.cmp(&b.tier))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(champions)
}

pub fn parse_aram_mayhem_augments_html(html: &str) -> Result<Vec<AramAugmentEntry>, String> {
    let rsc = decode_next_f_stream(html);
    let source = if rsc.is_empty() { html } else { &rsc };

    // Locate the `"data":[{"id":...,"tier":...,"performance":...}]` array inside the RSC stream.
    let mut search_from = 0usize;
    let mut matched_array: Option<&str> = None;

    while let Some(rel_idx) = source[search_from..].find("\"data\":[{\"id\":") {
        let abs_idx = search_from + rel_idx;
        let array_start = abs_idx + "\"data\":".len();
        if let Some(candidate) = extract_balanced_json_array(source, array_start)
            && candidate.contains("\"performance\":")
            && candidate.contains("\"popular\":")
        {
            matched_array = Some(candidate);
            break;
        }
        search_from = abs_idx + 8;
    }

    let array_json = matched_array
        .ok_or_else(|| "ARAM Mayhem augments payload not found in HTML".to_string())?;

    let raw_items: Vec<WebMayhemAugmentItem> = serde_json::from_str(array_json)
        .map_err(|e| format!("Failed to deserialize ARAM Mayhem augments: {}", e))?;

    let mut augments: Vec<AramAugmentEntry> = raw_items
        .into_iter()
        .filter_map(|item| {
            let name = item.name?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            let small_icon = item.small_icon.unwrap_or_default();
            let large_icon = item.large_icon.unwrap_or_default();
            let icon_url = if !small_icon.is_empty() {
                small_icon
            } else {
                large_icon
            };
            let desc = item.desc.unwrap_or_default();

            Some(AramAugmentEntry {
                id: item.id,
                name,
                tier: item.tier,
                rarity: item.rarity,
                performance: (item.performance * 100.0).round() / 100.0,
                popular: (item.popular * 100.0).round() / 100.0,
                icon_url,
                desc: clean_augment_description(&desc),
            })
        })
        .collect();

    augments.sort_by(|a, b| {
        a.tier
            .cmp(&b.tier)
            .then_with(|| b.popular.total_cmp(&a.popular))
            .then_with(|| b.performance.total_cmp(&a.performance))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(augments)
}

pub async fn fetch_aram_tier_list_with_urls(
    client: &reqwest::Client,
    mode: AramMode,
    ddragon: &DataDragonCache,
    aram_rest_url: &str,
    mayhem_web_url: &str,
) -> Result<Vec<AramChampionRank>, String> {
    let cache_key = match mode {
        AramMode::Aram => format!("aram:{}", aram_rest_url),
        AramMode::AramMayhem => format!("mayhem:{}", mayhem_web_url),
    };

    if let Ok(guard) = get_tier_cache().read()
        && let Some((ts, cached)) = guard.get(&cache_key)
        && ts.elapsed() < ARAM_CACHE_TTL
    {
        return Ok(cached.clone());
    }

    let list = match mode {
        AramMode::Aram => {
            let res = client
                .get(aram_rest_url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch ARAM tier list: {}", e))?;
            if !res.status().is_success() {
                return Err(format!("ARAM tier list HTTP error: {}", res.status()));
            }
            let json: serde_json::Value = res
                .json()
                .await
                .map_err(|e| format!("Failed to parse ARAM tier list JSON: {}", e))?;
            parse_classic_aram_champions(&json, ddragon)?
        }
        AramMode::AramMayhem => {
            let res = client
                .get(mayhem_web_url)
                .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch ARAM Mayhem tier list: {}", e))?;
            if !res.status().is_success() {
                return Err(format!(
                    "ARAM Mayhem tier list HTTP error: {}",
                    res.status()
                ));
            }
            let html = res
                .text()
                .await
                .map_err(|e| format!("Failed to read ARAM Mayhem HTML: {}", e))?;
            parse_aram_mayhem_champions_html(&html, ddragon)?
        }
    };

    if !list.is_empty()
        && let Ok(mut guard) = get_tier_cache().write()
    {
        guard.insert(cache_key, (Instant::now(), list.clone()));
    }

    Ok(list)
}

pub async fn fetch_aram_tier_list(
    client: &reqwest::Client,
    mode: AramMode,
    ddragon: &DataDragonCache,
) -> Result<Vec<AramChampionRank>, String> {
    fetch_aram_tier_list_with_urls(
        client,
        mode,
        ddragon,
        OPGG_ARAM_REST_URL,
        OPGG_ARAM_MAYHEM_WEB_URL,
    )
    .await
}

pub async fn fetch_aram_mayhem_augments_with_base(
    client: &reqwest::Client,
    champion_key: &str,
    web_base: &str,
) -> Result<Vec<AramAugmentEntry>, String> {
    let normalized = normalize_opgg_champion_key(champion_key);
    if normalized.is_empty() {
        return Err("Champion key cannot be empty".to_string());
    }

    let cache_key = format!("{}:{}", web_base, normalized);
    if let Ok(guard) = get_augments_cache().read()
        && let Some((ts, cached)) = guard.get(&cache_key)
        && ts.elapsed() < ARAM_CACHE_TTL
    {
        return Ok(cached.clone());
    }

    let url = format!("{}/{}/augments", web_base.trim_end_matches('/'), normalized);
    let res = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch ARAM Mayhem augments: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("ARAM Mayhem augments HTTP error: {}", res.status()));
    }

    let html = res
        .text()
        .await
        .map_err(|e| format!("Failed to read ARAM Mayhem augments HTML: {}", e))?;
    let augments = parse_aram_mayhem_augments_html(&html)?;

    if !augments.is_empty()
        && let Ok(mut guard) = get_augments_cache().write()
    {
        guard.insert(cache_key, (Instant::now(), augments.clone()));
    }

    Ok(augments)
}

pub async fn fetch_aram_mayhem_augments(
    client: &reqwest::Client,
    champion_key: &str,
) -> Result<Vec<AramAugmentEntry>, String> {
    fetch_aram_mayhem_augments_with_base(client, champion_key, OPGG_ARAM_MAYHEM_AUGMENTS_WEB_BASE)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ChampionMetadata;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_ddragon() -> DataDragonCache {
        let mut champions = HashMap::new();
        champions.insert(
            4,
            ChampionMetadata {
                champion_id: 4,
                key_name: "TwistedFate".to_string(),
                display_name: "Twisted Fate".to_string(),
                icon_url: "https://ddragon.example/TwistedFate.png".to_string(),
            },
        );
        champions.insert(
            50,
            ChampionMetadata {
                champion_id: 50,
                key_name: "Swain".to_string(),
                display_name: "Swain".to_string(),
                icon_url: "https://ddragon.example/Swain.png".to_string(),
            },
        );
        DataDragonCache {
            version: "16.22.1".to_string(),
            champions,
            runes: HashMap::new(),
            spells: HashMap::new(),
        }
    }

    #[test]
    fn test_parse_classic_aram_champions_sorts_by_rank_and_normalizes_rates() {
        let json = serde_json::json!({
            "data": [
                { "id": 4, "average_stats": { "win_rate": 0.522, "pick_rate": 0.078, "tier": 2, "rank": 16 } },
                { "id": 50, "average_stats": { "win_rate": 0.633, "pick_rate": 0.091, "tier": 0, "rank": 1 } }
            ]
        });

        let list = parse_classic_aram_champions(&json, &test_ddragon()).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, 50);
        assert_eq!(list[0].name, "Swain");
        assert_eq!(list[0].rank, 1);
        assert_eq!(list[0].tier, 0);
        assert_eq!(list[0].win_rate, Some(63.3));
        assert_eq!(list[1].id, 4);
        assert_eq!(list[1].key, "twistedfate");
    }

    #[test]
    fn test_parse_aram_mayhem_champions_from_next_f_html_normalizes_tiers() {
        let inner = r#"1a:["$","$L32",null,{"champions":[{"key":"ahri","name":"Ahri","image_url":"https://img/Ahri.png","champion_id":103,"id":103,"tier":2,"rank":14},{"key":"twistedfate","name":"Twisted Fate","image_url":"https://img/TF.png","champion_id":4,"id":4,"tier":1,"rank":1},{"key":"ashe","name":"Ashe","image_url":"https://img/Ashe.png","champion_id":22,"id":22,"tier":5,"rank":173}]}]"#;
        let escaped = serde_json::to_string(inner).unwrap();
        let html = format!(r#"<html><script>self.__next_f.push([1,{escaped}])</script></html>"#);

        let list = parse_aram_mayhem_champions_html(&html, &test_ddragon()).unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].id, 4);
        assert_eq!(list[0].key, "twistedfate");
        assert_eq!(list[0].rank, 1);
        assert_eq!(list[0].tier, 0); // OP
        assert_eq!(list[1].id, 103);
        assert_eq!(list[1].rank, 14);
        assert_eq!(list[1].tier, 1); // Tier 1
        assert_eq!(list[2].id, 22);
        assert_eq!(list[2].tier, 4); // Tier 4
    }

    #[test]
    fn test_parse_aram_mayhem_augments_skips_entries_missing_name() {
        let inner = r#"6c:["$","$L6d",null,{"data":[{"id":1032,"tier":5,"performance":97.14,"popular":0},{"id":1033,"name":null,"tier":5,"performance":50.0,"popular":0,"desc":null},{"id":1074,"tier":3,"performance":76.82,"popular":0.23,"name":"Raio Encolhedor","smallIcon":"https://img/shrink.png","rarity":4,"desc":"<OnHit>Ao contato</OnHit><br />Reduz dano."},{"id":3075,"tier":0,"performance":99.62,"popular":3.8,"name":"Coice","smallIcon":"https://img/recoil.png","rarity":4,"desc":"<font color='#F0C200'>?</font> causa dano."},{"id":2062,"tier":0,"performance":114.24,"popular":3.2,"name":"Conjuração Ecoante","smallIcon":"https://img/echo.png","rarity":8,"desc":"Conjura duas vezes."}]}]"#;
        let escaped = serde_json::to_string(inner).unwrap();
        let html = format!(r#"<html><script>self.__next_f.push([1,{escaped}])</script></html>"#);

        let augments = parse_aram_mayhem_augments_html(&html).unwrap();
        assert_eq!(augments.len(), 3);
        assert_eq!(augments[0].name, "Coice");
        assert_eq!(augments[0].tier, 0);
        assert_eq!(augments[0].rarity, Some(4));
        assert_eq!(augments[0].desc, "? causa dano.");
        assert_eq!(augments[1].name, "Conjuração Ecoante");
        assert_eq!(augments[1].rarity, Some(8));
        assert_eq!(augments[2].name, "Raio Encolhedor");
        assert_eq!(augments[2].desc, "Ao contato Reduz dano.");
    }

    #[tokio::test]
    async fn test_fetch_aram_mayhem_augments_with_mock_server() {
        let mock_server = MockServer::start().await;
        let inner = r#"6c:["$","$L6d",null,{"data":[{"id":3075,"tier":0,"performance":99.62,"popular":3.8,"name":"Coice","smallIcon":"https://img/recoil.png","rarity":4,"desc":"Dano extra."}]}]"#;
        let escaped = serde_json::to_string(inner).unwrap();
        let html = format!(r#"<html><script>self.__next_f.push([1,{escaped}])</script></html>"#);

        Mock::given(method("GET"))
            .and(path("/twistedfate/augments"))
            .respond_with(ResponseTemplate::new(200).set_body_string(html))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let augments =
            fetch_aram_mayhem_augments_with_base(&client, "Twisted Fate", &mock_server.uri())
                .await
                .unwrap();

        assert_eq!(augments.len(), 1);
        assert_eq!(augments[0].id, 3075);
        assert_eq!(augments[0].name, "Coice");
    }
}
