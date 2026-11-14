pub use crate::models::{FlashKeyPreference, UserPreferences};
use std::sync::{OnceLock, RwLock};

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
            patch_48h_protection: false,
        };

        set_user_preferences(prefs.clone());
        assert_eq!(get_user_preferences(), prefs);

        // Reset to default
        set_user_preferences(UserPreferences::default());
        assert_eq!(get_user_preferences(), UserPreferences::default());
        assert!(get_user_preferences().patch_48h_protection);
    }
}
