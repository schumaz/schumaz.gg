use super::*;
use crate::build_provider::ChampionBuild;
use serde::{Deserialize, Serialize};

pub const PERKS_CURRENT_PAGE_ENDPOINT: &str = "/lol-perks/v1/currentpage";
pub const PERKS_PAGES_ENDPOINT: &str = "/lol-perks/v1/pages";
pub const CHAMP_SELECT_MY_SELECTION_ENDPOINT: &str = "/lol-champ-select/v1/session/my-selection";

/// Summary report returned after attempting an injection into the LCU.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InjectionReport {
    pub champion_name: String,
    pub runes_injected: bool,
    pub spells_injected: bool,
    pub items_injected: bool,
    pub message: String,
}

/// Injects rune page into the LCU. Reuses and overwrites the single schumaz.gg rune page across matches
/// instead of creating a new page every match.
pub async fn inject_rune_payload(
    conn: &crate::lcu_connector::LcuConnection,
    payload: &serde_json::Value,
) -> Result<(), String> {
    // 1. Check all rune pages to see if a schumaz.gg page already exists
    if let Ok(pages) = conn.get_all_rune_pages().await {
        if let Some(existing_page_id) = find_schumaz_page_id(&pages) {
            // Overwrite the existing page in-place via PUT
            if conn.put_rune_page(existing_page_id, payload).await.is_ok() {
                return Ok(());
            }
            // If PUT fails, delete the existing schumaz page and recreate
            let _ = conn.delete_rune_page(existing_page_id).await;
            conn.create_rune_page(payload).await?;
            return Ok(());
        }

        // 2. No schumaz.gg page exists yet. Try creating a new one.
        match conn.create_rune_page(payload).await {
            Ok(_) => return Ok(()),
            Err(_) => {
                // If creation failed (e.g. max page limit reached), overwrite or delete the first custom page
                if let Some(deletable_id) = find_first_deletable_page_id(&pages) {
                    if conn.put_rune_page(deletable_id, payload).await.is_ok() {
                        return Ok(());
                    }
                    let _ = conn.delete_rune_page(deletable_id).await;
                    conn.create_rune_page(payload).await?;
                    return Ok(());
                }
            }
        }
    } else {
        // Fallback: direct create if pages endpoint query failed
        conn.create_rune_page(payload).await?;
    }

    Ok(())
}

/// Injects rune page into the LCU. Reuses and overwrites the single schumaz.gg rune page across matches
/// instead of creating a new page every match.
pub async fn inject_runes(
    conn: &crate::lcu_connector::LcuConnection,
    build: &ChampionBuild,
) -> Result<(), String> {
    let payload = build_rune_page_payload(build)?;
    inject_rune_payload(conn, &payload).await
}

/// Injects a specific chosen rune page directly into the LCU and saves the user choice in ChampionSelections.
pub async fn inject_specific_rune_page(
    conn: &crate::lcu_connector::LcuConnection,
    champion_id: i32,
    champion_name: &str,
    page_index: usize,
    primary_style_id: i32,
    sub_style_id: i32,
    selected_perk_ids: &[i32],
) -> Result<(), String> {
    set_selected_rune_page(champion_id, page_index);
    let payload = build_rune_page_payload_from_parts(
        champion_name,
        primary_style_id,
        sub_style_id,
        selected_perk_ids,
    )?;
    inject_rune_payload(conn, &payload).await
}

/// Injects summoner spells into the active Champion Select session adapting to the Flash key preference
/// and mandatory Jungle Smite rules.
pub async fn inject_spells(
    conn: &crate::lcu_connector::LcuConnection,
    build: &ChampionBuild,
    flash_key: FlashKeyPreference,
) -> Result<(), String> {
    let is_jungle = build.active_position.eq_ignore_ascii_case("jungle");
    let (mut s1, mut s2) = (build.spell1_id, build.spell2_id);

    if let Some(sel) = get_champion_selections(build.champion_id)
        && let Some(sec_spell) = sel.secondary_spell_id
    {
        if s1 == SPELL_FLASH_ID {
            s2 = sec_spell;
        } else if s2 == SPELL_FLASH_ID {
            s1 = sec_spell;
        } else {
            s2 = sec_spell;
        }
    }

    let payload = build_spells_payload_with_jungle_and_preference(s1, s2, flash_key, is_jungle)?;
    conn.patch_summoner_spells(&payload).await?;
    Ok(())
}

/// Injects custom item sets for the summoner into the game client shop.
pub async fn inject_item_sets(
    conn: &crate::lcu_connector::LcuConnection,
    summoner_id: u64,
    build: &ChampionBuild,
) -> Result<(), String> {
    let payload = build_item_sets_payload(summoner_id, build);
    conn.put_item_sets(summoner_id, &payload).await?;
    Ok(())
}

/// High-level orchestrator injecting runes, summoner spells, and item sets respecting user preferences.
/// Produces a comprehensive report of all injection outcomes.
pub async fn inject_champion_build(
    conn: &crate::lcu_connector::LcuConnection,
    summoner_id: u64,
    build: &ChampionBuild,
) -> InjectionReport {
    let prefs = get_user_preferences();

    let runes_res = if prefs.auto_inject_runes && build.supports_runes {
        Some(inject_runes(conn, build).await)
    } else {
        None
    };

    let spells_res = if prefs.auto_inject_spells {
        Some(inject_spells(conn, build, prefs.flash_key).await)
    } else {
        None
    };

    let items_res = if prefs.auto_inject_items {
        Some(inject_item_sets(conn, summoner_id, build).await)
    } else {
        None
    };

    let runes_ok = runes_res.as_ref().is_some_and(|r| r.is_ok());
    let spells_ok = spells_res.as_ref().is_some_and(|r| r.is_ok());
    let items_ok = items_res.as_ref().is_some_and(|r| r.is_ok());

    let mut parts = Vec::new();
    if !build.supports_runes {
        parts.push("Modo ARAM Mayhem (runas desativadas pela Riot)".to_string());
    } else if let Some(r) = runes_res {
        match r {
            Ok(_) => parts.push("Runas injetadas".to_string()),
            Err(e) => parts.push(format!("Falha nas Runas: {}", e)),
        }
    }

    if let Some(s) = spells_res {
        match s {
            Ok(_) => parts.push("Feitiços injetados".to_string()),
            Err(e) => parts.push(format!("Falha nos Feitiços: {}", e)),
        }
    }
    if let Some(i) = items_res {
        match i {
            Ok(_) => parts.push("Itens injetados".to_string()),
            Err(e) => parts.push(format!("Falha nos Itens: {}", e)),
        }
    }

    let message = if parts.is_empty() {
        "Nenhuma injeção configurada nas preferências".to_string()
    } else {
        parts.join(" | ")
    };

    InjectionReport {
        champion_name: build.champion_name.clone(),
        runes_injected: runes_ok,
        spells_injected: spells_ok,
        items_injected: items_ok,
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_injection_report_serialization() {
        let report = InjectionReport {
            champion_name: "Ahri".to_string(),
            runes_injected: true,
            spells_injected: true,
            items_injected: false,
            message: "Runas injetadas | Feitiços injetados".to_string(),
        };

        let json = serde_json::to_string(&report).unwrap();
        let parsed: InjectionReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, parsed);
    }
}
