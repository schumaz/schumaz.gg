use serde::{Deserialize, Serialize};
use std::sync::{OnceLock, RwLock};

/// Represents the user's preferred key for Flash (D vs F).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum FlashKeyPreference {
    #[default]
    D,
    F,
}

/// User preferences synchronized between frontend and backend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserPreferences {
    pub auto_inject_runes: bool,
    pub auto_inject_spells: bool,
    pub auto_inject_items: bool,
    pub flash_key: FlashKeyPreference,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            auto_inject_runes: true,
            auto_inject_spells: true,
            auto_inject_items: true,
            flash_key: FlashKeyPreference::D,
        }
    }
}

static USER_PREFERENCES: OnceLock<RwLock<UserPreferences>> = OnceLock::new();

fn get_preferences_lock() -> &'static RwLock<UserPreferences> {
    USER_PREFERENCES.get_or_init(|| RwLock::new(UserPreferences::default()))
}

/// Retrieves a clone of the current global user preferences.
pub fn get_user_preferences() -> UserPreferences {
    get_preferences_lock()
        .read()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

/// Updates the global user preferences.
pub fn set_user_preferences(prefs: UserPreferences) {
    if let Ok(mut guard) = get_preferences_lock().write() {
        *guard = prefs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_preferences_global_sync() {
        let prefs = UserPreferences {
            auto_inject_runes: false,
            auto_inject_spells: true,
            auto_inject_items: false,
            flash_key: FlashKeyPreference::F,
        };

        set_user_preferences(prefs.clone());
        assert_eq!(get_user_preferences(), prefs);

        // Reset to default
        set_user_preferences(UserPreferences::default());
        assert_eq!(get_user_preferences(), UserPreferences::default());
    }
}
