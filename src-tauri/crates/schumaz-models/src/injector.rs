use serde::{Deserialize, Serialize};
use specta::Type;

fn default_true() -> bool {
    true
}

/// Supported application & data locales (`pt_BR` | `en_US`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default, Type)]
pub enum AppLocale {
    #[serde(rename = "pt_BR")]
    PtBr,
    #[serde(rename = "en_US")]
    #[default]
    EnUs,
}

impl AppLocale {
    pub fn as_str(self) -> &'static str {
        match self {
            AppLocale::PtBr => "pt_BR",
            AppLocale::EnUs => "en_US",
        }
    }

    pub fn ddragon_code(self) -> &'static str {
        self.as_str()
    }

    pub fn opgg_aram_mayhem_augments_base(self) -> &'static str {
        match self {
            AppLocale::PtBr => "https://op.gg/pt/lol/modes/aram-mayhem",
            AppLocale::EnUs => "https://op.gg/lol/modes/aram-mayhem",
        }
    }
}

/// User's preferred key for Flash (D vs F).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, Type)]
pub enum FlashKeyPreference {
    #[default]
    D,
    F,
}

/// User preferences synchronized between frontend and backend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct UserPreferences {
    pub auto_inject_runes: bool,
    pub auto_inject_spells: bool,
    pub auto_inject_items: bool,
    pub flash_key: FlashKeyPreference,
    #[serde(default = "default_true", rename = "patch48hProtection")]
    pub patch_48h_protection: bool,
    #[serde(default, rename = "autoMarkDailyMissionsRead")]
    pub auto_mark_daily_missions_read: bool,
    #[serde(default)]
    pub language: AppLocale,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            auto_inject_runes: true,
            auto_inject_spells: true,
            auto_inject_items: true,
            flash_key: FlashKeyPreference::D,
            patch_48h_protection: true,
            auto_mark_daily_missions_read: false,
            language: AppLocale::EnUs,
        }
    }
}

/// Summary report returned after attempting an injection into the LCU.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
pub struct InjectionReport {
    pub champion_name: String,
    pub runes_injected: bool,
    pub spells_injected: bool,
    pub items_injected: bool,
    pub message: String,
}
