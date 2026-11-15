pub use crate::models::{AppLocale, FlashKeyPreference, UserPreferences};
use std::sync::{
    OnceLock, RwLock,
    atomic::{AtomicU8, Ordering},
};

static USER_PREFERENCES: OnceLock<RwLock<UserPreferences>> = OnceLock::new();
static ACTIVE_LOCALE: AtomicU8 = AtomicU8::new(1); // 0 = pt_BR, 1 = en_US

fn locale_to_u8(locale: AppLocale) -> u8 {
    match locale {
        AppLocale::PtBr => 0,
        AppLocale::EnUs => 1,
    }
}

fn u8_to_locale(val: u8) -> AppLocale {
    match val {
        0 => AppLocale::PtBr,
        _ => AppLocale::EnUs,
    }
}

fn get_preferences_lock() -> &'static RwLock<UserPreferences> {
    USER_PREFERENCES.get_or_init(|| RwLock::new(UserPreferences::default()))
}

/// Returns the active backend locale (`pt_BR` | `en_US`).
pub fn get_active_locale() -> AppLocale {
    u8_to_locale(ACTIVE_LOCALE.load(Ordering::Relaxed))
}

/// Updates the active backend locale (`pt_BR` | `en_US`).
pub fn set_active_locale(locale: AppLocale) {
    ACTIVE_LOCALE.store(locale_to_u8(locale), Ordering::Relaxed);
    if let Ok(mut guard) = get_preferences_lock().write() {
        guard.language = locale;
    }
}

/// Retrieves a clone of the current global user preferences.
pub fn get_user_preferences() -> UserPreferences {
    get_preferences_lock()
        .read()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

/// Updates the global user preferences and active locale.
pub fn set_user_preferences(prefs: UserPreferences) {
    ACTIVE_LOCALE.store(locale_to_u8(prefs.language), Ordering::Relaxed);
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
            auto_mark_daily_missions_read: true,
            language: AppLocale::PtBr,
        };

        set_user_preferences(prefs.clone());
        assert_eq!(get_user_preferences(), prefs);
        assert_eq!(get_active_locale(), AppLocale::PtBr);

        set_active_locale(AppLocale::EnUs);
        assert_eq!(get_active_locale(), AppLocale::EnUs);
        assert_eq!(get_user_preferences().language, AppLocale::EnUs);

        // Reset to default
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
}
