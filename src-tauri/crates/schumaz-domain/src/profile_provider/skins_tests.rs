use std::collections::HashMap;

use super::*;
use crate::models::ChampionMetadata;

fn sample_cache() -> DataDragonCache {
    let mut champions = HashMap::new();
    champions.insert(
        103,
        ChampionMetadata {
            champion_id: 103,
            key_name: "Ahri".to_string(),
            display_name: "Ahri".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Ahri.png"
                .to_string(),
        },
    );

    DataDragonCache {
        version: "16.22.1".to_string(),
        champions,
        runes: HashMap::new(),
        spells: HashMap::new(),
        ..Default::default()
    }
}

#[test]
fn test_resolve_background_splash_urls_extracts_champion_and_skin_num() {
    let cache = sample_cache();
    let (splash, fallback, label, key, skin_num) =
        resolve_background_splash_urls_for_locale(103015, &cache, AppLocale::PtBr);

    assert_eq!(
        splash,
        "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_15.jpg"
    );
    assert_eq!(
        fallback,
        "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_0.jpg"
    );
    assert_eq!(label, "Ahri #15");
    assert_eq!(key, "Ahri");
    assert_eq!(skin_num, 15);
}

#[test]
fn test_ensure_skins_hydrated_from_dir_reads_disk_once_and_rehydrates_on_eviction() {
    let dir = std::env::temp_dir().join(format!(
        "schumazgg_skins_once_test_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    if let Ok(mut g) = skin_cache_state().write() {
        g.names.remove(&AppLocale::EnUs);
        g.hydrated_dirs.clear();
    }

    let mut initial_map = HashMap::new();
    initial_map.insert(103027, "Spirit Blossom Ahri".to_string());
    let path = skins_file_path(&dir, AppLocale::EnUs);
    crate::storage::write_lz4_envelope(&path, "16.22.1", &initial_map).unwrap();

    // First call hydrates from disk.
    ensure_skins_hydrated_from_dir(&dir, AppLocale::EnUs);
    assert_eq!(
        lookup_cached_skin_name(AppLocale::EnUs, 103027).as_deref(),
        Some("Spirit Blossom Ahri")
    );

    // Replace disk file without clearing RAM: subsequent calls must skip disk I/O.
    let mut updated_map = HashMap::new();
    updated_map.insert(103027, "Changed On Disk Only".to_string());
    updated_map.insert(103028, "New Disk Skin".to_string());
    crate::storage::write_lz4_envelope(&path, "16.22.1", &updated_map).unwrap();

    ensure_skins_hydrated_from_dir(&dir, AppLocale::EnUs);
    assert_eq!(
        lookup_cached_skin_name(AppLocale::EnUs, 103027).as_deref(),
        Some("Spirit Blossom Ahri")
    );
    assert_eq!(lookup_cached_skin_name(AppLocale::EnUs, 103028), None);

    // Evicting the locale from SKIN_NAME_CACHE triggers fresh disk hydration.
    if let Ok(mut g) = skin_cache_state().write() {
        g.names.remove(&AppLocale::EnUs);
        g.hydrated_dirs.clear();
    }
    ensure_skins_hydrated_from_dir(&dir, AppLocale::EnUs);
    assert_eq!(
        lookup_cached_skin_name(AppLocale::EnUs, 103027).as_deref(),
        Some("Changed On Disk Only")
    );
    assert_eq!(
        lookup_cached_skin_name(AppLocale::EnUs, 103028).as_deref(),
        Some("New Disk Skin")
    );

    let _ = std::fs::remove_dir_all(&dir);
}
