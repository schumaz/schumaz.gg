pub use crate::models::{AppLocale, FlashKeyPreference, UserPreferences};

/// Returns the active backend locale (`pt_BR` | `en_US`) from the unified `SettingsStore`.
pub fn get_active_locale() -> AppLocale {
    crate::storage::get_active_locale()
}

/// Updates the active backend locale (`pt_BR` | `en_US`) in the unified `SettingsStore`.
pub fn set_active_locale(locale: AppLocale) {
    crate::storage::set_active_locale(locale);
}

/// Retrieves a clone of the current global user preferences from the unified `SettingsStore`.
pub fn get_user_preferences() -> UserPreferences {
    crate::storage::get_user_preferences()
}

/// Updates the global user preferences and active locale in the unified `SettingsStore`.
pub fn set_user_preferences(prefs: UserPreferences) {
    crate::storage::set_user_preferences(prefs);
}

/// Marks Flash key onboarding completed, persists `flashKey` to `UserSettings.ini`, and syncs `SettingsStore`.
pub fn complete_flash_onboarding_service(
    flash_key: FlashKeyPreference,
) -> Result<crate::models::AppSettings, String> {
    crate::storage::update_global_settings(|s| {
        s.general.flash_onboarding_completed = true;
        s.automation.flash_key = flash_key;
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    static PREFS_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_user_preferences_global_sync() {
        let _guard = PREFS_TEST_LOCK.lock().unwrap();
        let prefs = UserPreferences {
            auto_inject_runes: false,
            auto_inject_spells: true,
            auto_inject_items: false,
            flash_key: FlashKeyPreference::F,
            patch_48h_protection: false,
            auto_mark_daily_missions_read: true,
            language: AppLocale::PtBr,
        };

        set_user_preferences(prefs.clone());
        assert_eq!(get_user_preferences(), prefs);
        assert_eq!(get_active_locale(), AppLocale::PtBr);

        set_active_locale(AppLocale::EnUs);
        assert_eq!(get_active_locale(), AppLocale::EnUs);
        assert_eq!(get_user_preferences().language, AppLocale::EnUs);

        set_user_preferences(UserPreferences::default());
        assert_eq!(get_user_preferences(), UserPreferences::default());
        assert!(get_user_preferences().patch_48h_protection);
        assert!(!get_user_preferences().auto_mark_daily_missions_read);
        assert_eq!(get_active_locale(), AppLocale::EnUs);
    }

    #[test]
    fn test_user_preferences_deserialize_defaults_missing_flags() {
        let raw = r#"{
            "autoInjectRunes": true,
            "autoInjectSpells": true,
            "autoInjectItems": true,
            "flashKey": "D"
        }"#;
        let parsed: UserPreferences = serde_json::from_str(raw).unwrap();
        assert!(parsed.patch_48h_protection);
        assert!(!parsed.auto_mark_daily_missions_read);
        assert_eq!(parsed.language, AppLocale::EnUs);
    }

    #[test]
    fn test_complete_flash_onboarding_service_persists_flash_key_and_flag() {
        let _guard = PREFS_TEST_LOCK.lock().unwrap();
        let updated = complete_flash_onboarding_service(FlashKeyPreference::F).unwrap();
        assert!(updated.general.flash_onboarding_completed);
        assert_eq!(updated.automation.flash_key, FlashKeyPreference::F);
        assert_eq!(get_user_preferences().flash_key, FlashKeyPreference::F);
        set_user_preferences(UserPreferences::default());
    }
}
