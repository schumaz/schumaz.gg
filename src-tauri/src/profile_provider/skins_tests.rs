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
