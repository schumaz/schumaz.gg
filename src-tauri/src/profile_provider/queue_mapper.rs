use crate::models::{AppLocale, QUEUE_ARENA, QUEUE_ARENA_1710, QUEUE_ARENA_1750};

/// Maps LCU `queueId` and `gameMode` to normalized filter key and localized mode label.
pub fn map_queue_mode_for_locale(
    queue_id: i32,
    game_mode: &str,
    locale: AppLocale,
) -> (String, String) {
    match queue_id {
        420 => ("ranked_solo".to_string(), "Solo/Duo".to_string()),
        440 => ("ranked_flex".to_string(), "Flex".to_string()),
        400 | 430 | 480 | 490 => ("normal".to_string(), "Normal".to_string()),
        450 => ("aram".to_string(), "ARAM".to_string()),
        2400 => ("aram_mayhem".to_string(), "ARAM Mayhem".to_string()),
        QUEUE_ARENA | QUEUE_ARENA_1710 | QUEUE_ARENA_1750 => {
            ("arena".to_string(), "Arena".to_string())
        }
        900 | 1010 | 1900 => ("urf".to_string(), "URF".to_string()),
        1300 => (
            "nexus_blitz".to_string(),
            match locale {
                AppLocale::PtBr => "Blitz do Nexus",
                AppLocale::EnUs => "Nexus Blitz",
            }
            .to_string(),
        ),
        1400 => (
            "ultbook".to_string(),
            match locale {
                AppLocale::PtBr => "Livro Supremo",
                AppLocale::EnUs => "Ultimate Spellbook",
            }
            .to_string(),
        ),
        830..=890 => (
            "coop_vs_ai".to_string(),
            match locale {
                AppLocale::PtBr => "Co-op vs IA",
                AppLocale::EnUs => "Co-op vs AI",
            }
            .to_string(),
        ),
        _ => {
            let mode_upper = game_mode.trim().to_ascii_uppercase();
            if mode_upper.contains("MAYHEM") || mode_upper == "KIWI" {
                ("aram_mayhem".to_string(), "ARAM Mayhem".to_string())
            } else if mode_upper == "ARAM" {
                ("aram".to_string(), "ARAM".to_string())
            } else if mode_upper == "CHERRY" || mode_upper == "ARENA" {
                ("arena".to_string(), "Arena".to_string())
            } else if mode_upper.contains("URF") {
                ("urf".to_string(), "URF".to_string())
            } else if mode_upper == "CLASSIC" || mode_upper == "SWIFTPLAY" {
                ("normal".to_string(), "Normal".to_string())
            } else if mode_upper.is_empty() {
                (
                    "other".to_string(),
                    match locale {
                        AppLocale::PtBr => "Outro",
                        AppLocale::EnUs => "Other",
                    }
                    .to_string(),
                )
            } else {
                (mode_upper.to_ascii_lowercase(), mode_upper)
            }
        }
    }
}

/// Maps LCU `queueId` and `gameMode` to normalized filter key and localized PT-BR mode label.
pub fn map_queue_mode(queue_id: i32, game_mode: &str) -> (String, String) {
    map_queue_mode_for_locale(queue_id, game_mode, AppLocale::PtBr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_mode_localization_for_en_us() {
        assert_eq!(
            map_queue_mode_for_locale(1400, "", AppLocale::EnUs).1,
            "Ultimate Spellbook"
        );
        assert_eq!(
            map_queue_mode_for_locale(1300, "", AppLocale::EnUs).1,
            "Nexus Blitz"
        );
        assert_eq!(
            map_queue_mode_for_locale(1750, "", AppLocale::EnUs),
            ("arena".to_string(), "Arena".to_string())
        );
    }
}
