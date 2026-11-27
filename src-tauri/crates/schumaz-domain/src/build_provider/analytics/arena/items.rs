use super::item_options::build_arena_unified_options;
use crate::build_provider::items::{
    ARENA_PRISMATIC_ID_RANGE, is_arena_anvil_or_consumable, is_arena_prismatic_dynamic,
    is_boot_item_dynamic, is_completed_item_dynamic, is_component_item_dynamic,
    is_map30_item_dynamic, is_starter_item, normalize_arena_item_id,
};
use crate::build_provider::models::{ItemBlock, ItemBlockKind, ItemStatEntry, UnifiedBuildOption};
use std::collections::{HashMap, HashSet};

/// Secondary boots below 1.5% share of total boot picks are dropped once a primary boot exists
/// (prevents 0.6% troll boots like Berserker's Greaves on Yuumi).
const MIN_SECONDARY_BOOT_PICK_SHARE: f64 = 0.015;
const MAX_ARENA_BOOTS: usize = 3;
const MAX_ARENA_PRISMATICS: usize = 6;

#[derive(Debug, Clone)]
pub(super) struct AggregatedItemStat {
    pub(super) id: i32,
    pub(super) play: i64,
    pub(super) win: i64,
}

/// Builds the combined set of real Arena Prismatic IDs from OP.GG's `prism_items` array while
/// protecting purchasable `44xxxx` legendaries (`447111`, `2500g`) from being misclassified.
fn collect_known_prismatics(prism_entries: &[ItemStatEntry]) -> HashSet<i32> {
    let mut prismatics = HashSet::new();
    for entry in prism_entries {
        for &raw_id in &entry.ids {
            let norm = normalize_arena_item_id(raw_id);
            if norm <= 0 || is_boot_item_dynamic(norm) || is_arena_anvil_or_consumable(norm) {
                continue;
            }
            if ARENA_PRISMATIC_ID_RANGE.contains(&norm)
                && is_completed_item_dynamic(norm)
                && !is_arena_prismatic_dynamic(norm)
            {
                continue;
            }
            prismatics.insert(norm);
        }
    }
    prismatics
}

fn is_prismatic_item(norm_id: i32, known_prismatics: &HashSet<i32>) -> bool {
    is_arena_prismatic_dynamic(norm_id) || known_prismatics.contains(&norm_id)
}

/// Returns `true` iff `norm_id` is a valid purchasable Map `30` legendary item:
/// accepts Arena `22xxxx` legendaries, 4-digit exclusives (`3430`, `4010`–`4017`), and purchasable
/// `44xxxx` legendaries (`447111` at `2500g`), while strictly rejecting Prismatics, Boots, Anvils,
/// Consumables, Starters, Components, and non-Map-30 SR items.
pub(crate) fn is_valid_arena_legendary(norm_id: i32, known_prismatics: &HashSet<i32>) -> bool {
    norm_id > 0
        && !is_boot_item_dynamic(norm_id)
        && !is_prismatic_item(norm_id, known_prismatics)
        && !is_arena_anvil_or_consumable(norm_id)
        && !is_starter_item(norm_id)
        && !is_component_item_dynamic(norm_id)
        && is_map30_item_dynamic(norm_id)
}

fn aggregate_single_items<F>(entries: &[ItemStatEntry], predicate: F) -> Vec<AggregatedItemStat>
where
    F: Fn(i32) -> bool,
{
    let mut totals: HashMap<i32, (i64, i64)> = HashMap::new();
    for entry in entries {
        let play = entry.play.max(0);
        let win = entry.win.max(0);
        for &raw_id in &entry.ids {
            let norm = normalize_arena_item_id(raw_id);
            if !predicate(norm) {
                continue;
            }
            let slot = totals.entry(norm).or_insert((0, 0));
            slot.0 = slot.0.saturating_add(play);
            slot.1 = slot.1.saturating_add(win);
        }
    }

    let mut ranked: Vec<AggregatedItemStat> = totals
        .into_iter()
        .map(|(id, (play, win))| AggregatedItemStat { id, play, win })
        .collect();
    ranked.sort_by(|a, b| {
        b.play
            .cmp(&a.play)
            .then_with(|| b.win.cmp(&a.win))
            .then_with(|| a.id.cmp(&b.id))
    });
    ranked
}

/// Selects relevant Arena boots without SR fallbacks, dropping residual troll boots (`< 1.5%`
/// pick share) whenever a primary boot exists.
fn select_arena_boots(boot_entries: &[ItemStatEntry]) -> Vec<i32> {
    let ranked = aggregate_single_items(boot_entries, |id| {
        id != 1001
            && id != 2422
            && is_boot_item_dynamic(id)
            && is_map30_item_dynamic(id)
            && !is_arena_anvil_or_consumable(id)
    });
    let total_play: i64 = ranked.iter().map(|b| b.play.max(0)).sum();

    let mut selected = Vec::with_capacity(MAX_ARENA_BOOTS);
    for (idx, boot) in ranked.into_iter().enumerate() {
        if selected.len() >= MAX_ARENA_BOOTS {
            break;
        }
        if idx == 0 {
            selected.push(boot.id);
            continue;
        }
        if total_play > 0 && (boot.play as f64 / total_play as f64) >= MIN_SECONDARY_BOOT_PICK_SHARE
        {
            selected.push(boot.id);
        }
    }
    selected
}

/// Selects the top 5–6 Arena Prismatic items from OP.GG's `prism_items` array.
fn select_arena_prismatics(
    prism_entries: &[ItemStatEntry],
    known_prismatics: &HashSet<i32>,
) -> Vec<i32> {
    aggregate_single_items(prism_entries, |id| {
        is_prismatic_item(id, known_prismatics)
            && is_map30_item_dynamic(id)
            && !is_boot_item_dynamic(id)
            && !is_arena_anvil_or_consumable(id)
    })
    .into_iter()
    .take(MAX_ARENA_PRISMATICS)
    .map(|entry| entry.id)
    .collect()
}

/// Builds the 5 Arena item blocks (`Prismatic`, `Core`, `Boots`, `Final`, `Situational`) and up to
/// 3 distinct archetype-cohesive `UnifiedBuildOption`s without any SR starter/fallback items.
pub fn build_arena_item_blocks_and_options(
    boot_entries: &[ItemStatEntry],
    prism_entries: &[ItemStatEntry],
    core_entries: &[ItemStatEntry],
    last_entries: &[ItemStatEntry],
) -> (Vec<ItemBlock>, Vec<UnifiedBuildOption>) {
    let known_prismatics = collect_known_prismatics(prism_entries);
    let boot_ids = select_arena_boots(boot_entries);
    let prismatic_ids = select_arena_prismatics(prism_entries, &known_prismatics);

    let mut legendary_pool = aggregate_single_items(last_entries, |id| {
        is_valid_arena_legendary(id, &known_prismatics)
    });
    let mut seen_pool: HashSet<i32> = legendary_pool.iter().map(|i| i.id).collect();
    for item in aggregate_single_items(core_entries, |id| {
        is_valid_arena_legendary(id, &known_prismatics)
    }) {
        if seen_pool.insert(item.id) {
            legendary_pool.push(item);
        }
    }

    let build_options =
        build_arena_unified_options(core_entries, &legendary_pool, &known_prismatics);

    let (primary_core, primary_final, primary_situational) = build_options
        .first()
        .map(|opt| {
            (
                opt.core_item_ids.clone(),
                opt.final_item_ids.clone(),
                opt.situational_item_ids.clone(),
            )
        })
        .unwrap_or_default();

    let item_blocks = vec![
        ItemBlock {
            title: "Itens Prismáticos".to_string(),
            kind: ItemBlockKind::Prismatic,
            item_ids: prismatic_ids,
            item_icon_urls: vec![],
        },
        ItemBlock {
            title: "Primeiros 3 Itens".to_string(),
            kind: ItemBlockKind::Core,
            item_ids: primary_core,
            item_icon_urls: vec![],
        },
        ItemBlock {
            title: "3 Botas Mais Feitas".to_string(),
            kind: ItemBlockKind::Boots,
            item_ids: boot_ids,
            item_icon_urls: vec![],
        },
        ItemBlock {
            title: "Build Final".to_string(),
            kind: ItemBlockKind::Final,
            item_ids: primary_final,
            item_icon_urls: vec![],
        },
        ItemBlock {
            title: "Itens Situacionais / Úteis".to_string(),
            kind: ItemBlockKind::Situational,
            item_ids: primary_situational,
            item_icon_urls: vec![],
        },
    ];

    (item_blocks, build_options)
}
