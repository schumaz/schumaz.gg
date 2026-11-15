use crate::models::{AppLocale, AramMode, FlashKeyPreference, UserPreferences};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use super::atomic_write;

pub const MIN_WINDOW_WIDTH: f64 = 840.0;
pub const MIN_WINDOW_HEIGHT: f64 = 600.0;
pub const DEFAULT_WINDOW_WIDTH: f64 = 980.0;
pub const DEFAULT_WINDOW_HEIGHT: f64 = 720.0;
pub const DEFAULT_PROFILE_BLUR_PERCENT: u32 = 10;

const SEC_GENERAL: &str = "/Script/SchumazGG.GeneralSettings";
const SEC_AUTOMATION: &str = "/Script/SchumazGG.AutomationSettings";
const SEC_PROFILE_APPEARANCE: &str = "/Script/SchumazGG.ProfileAppearanceSettings";
const SEC_GRAPHICS: &str = "/Script/SchumazGG.GraphicsSettings";
const SEC_WINDOW: &str = "/Script/SchumazGG.WindowSettings";
const SEC_NAVIGATION: &str = "/Script/SchumazGG.NavigationState";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSettings {
    pub language: Option<AppLocale>,
    pub language_onboarding_completed: bool,
    pub legacy_storage_migrated: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            language: None,
            language_onboarding_completed: false,
            legacy_storage_migrated: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProfileAppearanceSettings {
    pub profile_blur_percent: u32,
    pub wallpaper_visible: bool,
}

impl Default for ProfileAppearanceSettings {
    fn default() -> Self {
        Self {
            profile_blur_percent: DEFAULT_PROFILE_BLUR_PERCENT,
            wallpaper_visible: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphicsSettings {
    pub hardware_acceleration: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            hardware_acceleration: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowSettings {
    pub window_width: f64,
    pub window_height: f64,
    pub window_x: Option<f64>,
    pub window_y: Option<f64>,
    pub window_maximized: bool,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            window_width: DEFAULT_WINDOW_WIDTH,
            window_height: DEFAULT_WINDOW_HEIGHT,
            window_x: None,
            window_y: None,
            window_maximized: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct NavigationState {
    pub saved_aram_mode: Option<AramMode>,
    pub profile_match_mode_filter: String,
    pub aram_mayhem_rarity_filter: String,
    pub aram_mayhem_last_champion: Option<String>,
    pub ranked_filter_region: String,
    pub ranked_filter_tier: String,
    pub ranked_filter_lane: String,
    pub ranked_filter_patch: Option<String>,
    pub ranked_champion_tier_filter: String,
    pub last_summoner_id: Option<u64>,
}

impl Default for NavigationState {
    fn default() -> Self {
        Self {
            saved_aram_mode: None,
            profile_match_mode_filter: "all".to_string(),
            aram_mayhem_rarity_filter: "all".to_string(),
            aram_mayhem_last_champion: None,
            ranked_filter_region: "global".to_string(),
            ranked_filter_tier: "emerald_plus".to_string(),
            ranked_filter_lane: "all".to_string(),
            ranked_filter_patch: None,
            ranked_champion_tier_filter: "all".to_string(),
            last_summoner_id: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub general: GeneralSettings,
    pub automation: UserPreferences,
    pub profile_appearance: ProfileAppearanceSettings,
    pub graphics: GraphicsSettings,
    pub window: WindowSettings,
    pub navigation: NavigationState,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NavigationStatePatch {
    pub saved_aram_mode: Option<AramMode>,
    pub profile_match_mode_filter: Option<String>,
    pub aram_mayhem_rarity_filter: Option<String>,
    /// Empty string clears `aram_mayhem_last_champion` to `None`.
    pub aram_mayhem_last_champion: Option<String>,
    pub ranked_filter_region: Option<String>,
    pub ranked_filter_tier: Option<String>,
    pub ranked_filter_lane: Option<String>,
    /// Empty string clears `ranked_filter_patch` to `None`.
    pub ranked_filter_patch: Option<String>,
    pub ranked_champion_tier_filter: Option<String>,
    pub last_summoner_id: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LegacyLocalStoragePayload {
    pub schumaz_user_preferences: Option<String>,
    pub schumaz_language: Option<String>,
    pub schumaz_profile_ui_blur: Option<String>,
    pub schumaz_profile_wallpaper_visible: Option<String>,
    pub schumaz_aram_submode: Option<String>,
    pub schumaz_ranked_filters: Option<String>,
}

fn parse_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn fmt_bool(v: bool) -> &'static str {
    if v { "True" } else { "False" }
}

fn parse_locale(raw: &str) -> Option<AppLocale> {
    match raw.trim() {
        "pt_BR" | "pt-BR" | "pt_br" => Some(AppLocale::PtBr),
        "en_US" | "en-US" | "en_us" => Some(AppLocale::EnUs),
        _ => None,
    }
}

fn parse_flash_key(raw: &str) -> Option<FlashKeyPreference> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "D" => Some(FlashKeyPreference::D),
        "F" => Some(FlashKeyPreference::F),
        _ => None,
    }
}

fn parse_aram_mode(raw: &str) -> Option<AramMode> {
    match raw.trim() {
        "aram" => Some(AramMode::Aram),
        "aram_mayhem" => Some(AramMode::AramMayhem),
        _ => None,
    }
}

fn format_aram_mode(mode: AramMode) -> &'static str {
    match mode {
        AramMode::Aram => "aram",
        AramMode::AramMayhem => "aram_mayhem",
    }
}

fn clamp_profile_blur(val: u32) -> u32 {
    val.min(100)
}

fn validate_rarity_filter(raw: &str) -> String {
    match raw.trim() {
        "all" | "1" | "4" | "8" => raw.trim().to_string(),
        _ => "all".to_string(),
    }
}

fn validate_ranked_lane(raw: &str) -> String {
    match raw.trim() {
        "all" | "top" | "jungle" | "mid" | "adc" | "support" => raw.trim().to_string(),
        _ => "all".to_string(),
    }
}

fn validate_champion_tier_filter(raw: &str) -> String {
    match raw.trim() {
        "all" | "0" | "1" | "2" | "3" | "4" | "5" => raw.trim().to_string(),
        _ => "all".to_string(),
    }
}

/// Parses `UserSettings.ini` tolerantly: unknown sections/keys are ignored,
/// and malformed values fall back per-key to defaults.
pub fn parse_user_settings_ini(content: &str) -> AppSettings {
    let mut settings = AppSettings::default();
    let mut current_section = String::new();
    let mut saw_explicit_onboarding = false;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') && line.len() >= 2 {
            current_section = line[1..line.len() - 1].trim().to_string();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let key = k.trim();
        let val = v.trim();

        match current_section.as_str() {
            SEC_GENERAL => match key {
                "Language" => {
                    if let Some(loc) = parse_locale(val) {
                        settings.general.language = Some(loc);
                        settings.automation.language = loc;
                    }
                }
                "bLanguageOnboardingCompleted" => {
                    if let Some(b) = parse_bool(val) {
                        settings.general.language_onboarding_completed = b;
                        saw_explicit_onboarding = true;
                    }
                }
                "bLegacyStorageMigrated" => {
                    if let Some(b) = parse_bool(val) {
                        settings.general.legacy_storage_migrated = b;
                    }
                }
                _ => {}
            },
            SEC_AUTOMATION => match key {
                "bAutoInjectRunes" => {
                    if let Some(b) = parse_bool(val) {
                        settings.automation.auto_inject_runes = b;
                    }
                }
                "bAutoInjectSpells" => {
                    if let Some(b) = parse_bool(val) {
                        settings.automation.auto_inject_spells = b;
                    }
                }
                "bAutoInjectItems" => {
                    if let Some(b) = parse_bool(val) {
                        settings.automation.auto_inject_items = b;
                    }
                }
                "FlashKey" => {
                    if let Some(fk) = parse_flash_key(val) {
                        settings.automation.flash_key = fk;
                    }
                }
                "bPatch48hProtection" => {
                    if let Some(b) = parse_bool(val) {
                        settings.automation.patch_48h_protection = b;
                    }
                }
                "bAutoMarkDailyMissionsRead" => {
                    if let Some(b) = parse_bool(val) {
                        settings.automation.auto_mark_daily_missions_read = b;
                    }
                }
                _ => {}
            },
            SEC_PROFILE_APPEARANCE => match key {
                "ProfileBlurPercent" => {
                    if let Ok(n) = val.parse::<u32>() {
                        settings.profile_appearance.profile_blur_percent = clamp_profile_blur(n);
                    }
                }
                "bWallpaperVisible" => {
                    if let Some(b) = parse_bool(val) {
                        settings.profile_appearance.wallpaper_visible = b;
                    }
                }
                _ => {}
            },
            SEC_GRAPHICS => match key {
                "bHardwareAcceleration" => {
                    if let Some(b) = parse_bool(val) {
                        settings.graphics.hardware_acceleration = b;
                    }
                }
                _ => {}
            },
            SEC_WINDOW => match key {
                "WindowWidth" => {
                    if let Ok(w) = val.parse::<f64>()
                        && w.is_finite()
                        && w >= MIN_WINDOW_WIDTH
                    {
                        settings.window.window_width = w;
                    }
                }
                "WindowHeight" => {
                    if let Ok(h) = val.parse::<f64>()
                        && h.is_finite()
                        && h >= MIN_WINDOW_HEIGHT
                    {
                        settings.window.window_height = h;
                    }
                }
                "WindowX" => {
                    if let Ok(x) = val.parse::<f64>()
                        && x.is_finite()
                    {
                        settings.window.window_x = Some(x);
                    }
                }
                "WindowY" => {
                    if let Ok(y) = val.parse::<f64>()
                        && y.is_finite()
                    {
                        settings.window.window_y = Some(y);
                    }
                }
                "bWindowMaximized" => {
                    if let Some(b) = parse_bool(val) {
                        settings.window.window_maximized = b;
                    }
                }
                _ => {}
            },
            SEC_NAVIGATION => match key {
                "SavedAramMode" => {
                    settings.navigation.saved_aram_mode = parse_aram_mode(val);
                }
                "ProfileMatchModeFilter" => {
                    if !val.is_empty() {
                        settings.navigation.profile_match_mode_filter = val.to_string();
                    }
                }
                "AramMayhemRarityFilter" => {
                    settings.navigation.aram_mayhem_rarity_filter = validate_rarity_filter(val);
                }
                "AramMayhemLastChampion" => {
                    if !val.is_empty() {
                        settings.navigation.aram_mayhem_last_champion =
                            Some(val.to_ascii_lowercase());
                    }
                }
                "RankedFilterRegion" => {
                    if !val.is_empty() {
                        settings.navigation.ranked_filter_region = val.to_string();
                    }
                }
                "RankedFilterTier" => {
                    if !val.is_empty() {
                        settings.navigation.ranked_filter_tier = val.to_string();
                    }
                }
                "RankedFilterLane" => {
                    settings.navigation.ranked_filter_lane = validate_ranked_lane(val);
                }
                "RankedFilterPatch" => {
                    if !val.is_empty() {
                        settings.navigation.ranked_filter_patch = Some(val.to_string());
                    }
                }
                "RankedChampionTierFilter" => {
                    settings.navigation.ranked_champion_tier_filter =
                        validate_champion_tier_filter(val);
                }
                "LastSummonerId" => {
                    if let Ok(id) = val.parse::<u64>()
                        && id > 0
                    {
                        settings.navigation.last_summoner_id = Some(id);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    if !saw_explicit_onboarding && settings.general.language.is_some() {
        settings.general.language_onboarding_completed = true;
    }

    settings
}

pub fn serialize_user_settings_ini(s: &AppSettings) -> String {
    let mut out = String::with_capacity(1024);

    out.push_str(&format!("[{SEC_GENERAL}]\n"));
    if let Some(lang) = s.general.language {
        out.push_str(&format!("Language={}\n", lang.as_str()));
    } else {
        out.push_str("Language=\n");
    }
    out.push_str(&format!(
        "bLanguageOnboardingCompleted={}\n",
        fmt_bool(s.general.language_onboarding_completed)
    ));
    out.push_str(&format!(
        "bLegacyStorageMigrated={}\n\n",
        fmt_bool(s.general.legacy_storage_migrated)
    ));

    out.push_str(&format!("[{SEC_AUTOMATION}]\n"));
    out.push_str(&format!(
        "bAutoInjectRunes={}\n",
        fmt_bool(s.automation.auto_inject_runes)
    ));
    out.push_str(&format!(
        "bAutoInjectSpells={}\n",
        fmt_bool(s.automation.auto_inject_spells)
    ));
    out.push_str(&format!(
        "bAutoInjectItems={}\n",
        fmt_bool(s.automation.auto_inject_items)
    ));
    out.push_str(&format!(
        "FlashKey={}\n",
        match s.automation.flash_key {
            FlashKeyPreference::D => "D",
            FlashKeyPreference::F => "F",
        }
    ));
    out.push_str(&format!(
        "bPatch48hProtection={}\n",
        fmt_bool(s.automation.patch_48h_protection)
    ));
    out.push_str(&format!(
        "bAutoMarkDailyMissionsRead={}\n\n",
        fmt_bool(s.automation.auto_mark_daily_missions_read)
    ));

    out.push_str(&format!("[{SEC_PROFILE_APPEARANCE}]\n"));
    out.push_str(&format!(
        "ProfileBlurPercent={}\n",
        clamp_profile_blur(s.profile_appearance.profile_blur_percent)
    ));
    out.push_str(&format!(
        "bWallpaperVisible={}\n\n",
        fmt_bool(s.profile_appearance.wallpaper_visible)
    ));

    out.push_str(&format!("[{SEC_GRAPHICS}]\n"));
    out.push_str(&format!(
        "bHardwareAcceleration={}\n\n",
        fmt_bool(s.graphics.hardware_acceleration)
    ));

    out.push_str(&format!("[{SEC_WINDOW}]\n"));
    out.push_str(&format!(
        "WindowWidth={}\n",
        s.window.window_width.round() as i64
    ));
    out.push_str(&format!(
        "WindowHeight={}\n",
        s.window.window_height.round() as i64
    ));
    match s.window.window_x {
        Some(x) => out.push_str(&format!("WindowX={}\n", x.round() as i64)),
        None => out.push_str("WindowX=\n"),
    }
    match s.window.window_y {
        Some(y) => out.push_str(&format!("WindowY={}\n", y.round() as i64)),
        None => out.push_str("WindowY=\n"),
    }
    out.push_str(&format!(
        "bWindowMaximized={}\n\n",
        fmt_bool(s.window.window_maximized)
    ));

    out.push_str(&format!("[{SEC_NAVIGATION}]\n"));
    match s.navigation.saved_aram_mode {
        Some(m) => out.push_str(&format!("SavedAramMode={}\n", format_aram_mode(m))),
        None => out.push_str("SavedAramMode=\n"),
    }
    out.push_str(&format!(
        "ProfileMatchModeFilter={}\n",
        s.navigation.profile_match_mode_filter
    ));
    out.push_str(&format!(
        "AramMayhemRarityFilter={}\n",
        s.navigation.aram_mayhem_rarity_filter
    ));
    out.push_str(&format!(
        "AramMayhemLastChampion={}\n",
        s.navigation
            .aram_mayhem_last_champion
            .as_deref()
            .unwrap_or("")
    ));
    out.push_str(&format!(
        "RankedFilterRegion={}\n",
        s.navigation.ranked_filter_region
    ));
    out.push_str(&format!(
        "RankedFilterTier={}\n",
        s.navigation.ranked_filter_tier
    ));
    out.push_str(&format!(
        "RankedFilterLane={}\n",
        s.navigation.ranked_filter_lane
    ));
    out.push_str(&format!(
        "RankedFilterPatch={}\n",
        s.navigation.ranked_filter_patch.as_deref().unwrap_or("")
    ));
    out.push_str(&format!(
        "RankedChampionTierFilter={}\n",
        s.navigation.ranked_champion_tier_filter
    ));
    match s.navigation.last_summoner_id {
        Some(id) => out.push_str(&format!("LastSummonerId={id}\n")),
        None => out.push_str("LastSummonerId=\n"),
    }

    out
}

pub fn load_or_migrate_user_settings(
    ini_path: &Path,
    legacy_graphics_json: Option<&Path>,
) -> AppSettings {
    if ini_path.exists()
        && let Ok(raw) = fs::read_to_string(ini_path)
    {
        return parse_user_settings_ini(&raw);
    }

    let mut settings = AppSettings::default();
    let mut migrated_any = false;

    if let Some(legacy_path) = legacy_graphics_json
        && legacy_path.exists()
        && let Ok(raw) = fs::read_to_string(legacy_path)
        && let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw)
        && let Some(enabled) = json.get("hardware_acceleration").and_then(|v| v.as_bool())
    {
        settings.graphics.hardware_acceleration = enabled;
        migrated_any = true;
    }

    if migrated_any {
        let _ = save_user_settings_to_path(ini_path, &settings);
    }

    settings
}

pub fn save_user_settings_to_path(path: &Path, settings: &AppSettings) -> Result<(), String> {
    let serialized = serialize_user_settings_ini(settings);
    atomic_write(path, serialized.as_bytes())
}

pub fn apply_legacy_local_storage_migration(
    settings: &mut AppSettings,
    payload: &LegacyLocalStoragePayload,
) -> bool {
    if settings.general.legacy_storage_migrated {
        return false;
    }

    let mut changed = false;

    if let Some(raw_lang) = payload.schumaz_language.as_deref()
        && let Some(loc) = parse_locale(raw_lang)
    {
        settings.general.language = Some(loc);
        settings.general.language_onboarding_completed = true;
        settings.automation.language = loc;
        changed = true;
    }

    if let Some(raw_prefs) = payload.schumaz_user_preferences.as_deref()
        && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw_prefs)
    {
        if let Some(v) = parsed.get("autoInjectRunes").and_then(|x| x.as_bool()) {
            settings.automation.auto_inject_runes = v;
        }
        if let Some(v) = parsed.get("autoInjectSpells").and_then(|x| x.as_bool()) {
            settings.automation.auto_inject_spells = v;
        }
        if let Some(v) = parsed.get("autoInjectItems").and_then(|x| x.as_bool()) {
            settings.automation.auto_inject_items = v;
        }
        if let Some(fk) = parsed
            .get("flashKey")
            .and_then(|x| x.as_str())
            .and_then(parse_flash_key)
        {
            settings.automation.flash_key = fk;
        }
        if let Some(v) = parsed.get("patch48hProtection").and_then(|x| x.as_bool()) {
            settings.automation.patch_48h_protection = v;
        }
        if let Some(v) = parsed
            .get("autoMarkDailyMissionsRead")
            .and_then(|x| x.as_bool())
        {
            settings.automation.auto_mark_daily_missions_read = v;
        }
        if let Some(loc) = parsed
            .get("language")
            .and_then(|x| x.as_str())
            .and_then(parse_locale)
            && settings.general.language.is_none()
        {
            settings.general.language = Some(loc);
            settings.automation.language = loc;
        }
        changed = true;
    }

    if let Some(raw_blur) = payload.schumaz_profile_ui_blur.as_deref()
        && let Ok(val) = raw_blur.trim().parse::<u32>()
    {
        settings.profile_appearance.profile_blur_percent = clamp_profile_blur(val);
        changed = true;
    }

    if let Some(raw_vis) = payload.schumaz_profile_wallpaper_visible.as_deref()
        && let Some(b) = parse_bool(raw_vis)
    {
        settings.profile_appearance.wallpaper_visible = b;
        changed = true;
    }

    if let Some(raw_aram) = payload.schumaz_aram_submode.as_deref()
        && let Some(mode) = parse_aram_mode(raw_aram)
    {
        settings.navigation.saved_aram_mode = Some(mode);
        changed = true;
    }

    if let Some(raw_ranked) = payload.schumaz_ranked_filters.as_deref()
        && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw_ranked)
    {
        if let Some(r) = parsed.get("region").and_then(|x| x.as_str()).map(str::trim)
            && !r.is_empty()
        {
            settings.navigation.ranked_filter_region = r.to_string();
        }
        if let Some(t) = parsed.get("tier").and_then(|x| x.as_str()).map(str::trim)
            && !t.is_empty()
        {
            settings.navigation.ranked_filter_tier = t.to_string();
        }
        if let Some(l) = parsed.get("lane").and_then(|x| x.as_str()) {
            settings.navigation.ranked_filter_lane = validate_ranked_lane(l);
        }
        if let Some(p) = parsed.get("patch").and_then(|x| x.as_str()).map(str::trim)
            && !p.is_empty()
        {
            settings.navigation.ranked_filter_patch = Some(p.to_string());
        }
        changed = true;
    }

    settings.general.legacy_storage_migrated = true;
    changed || true
}

pub fn apply_navigation_patch(nav: &mut NavigationState, patch: NavigationStatePatch) {
    if let Some(mode) = patch.saved_aram_mode {
        nav.saved_aram_mode = Some(mode);
    }
    if let Some(mode_filter) = patch.profile_match_mode_filter {
        let trimmed = mode_filter.trim();
        nav.profile_match_mode_filter = if trimmed.is_empty() {
            "all".to_string()
        } else {
            trimmed.to_string()
        };
    }
    if let Some(rarity) = patch.aram_mayhem_rarity_filter {
        nav.aram_mayhem_rarity_filter = validate_rarity_filter(&rarity);
    }
    if let Some(champ) = patch.aram_mayhem_last_champion {
        let trimmed = champ.trim().to_ascii_lowercase();
        nav.aram_mayhem_last_champion = (!trimmed.is_empty()).then_some(trimmed);
    }
    if let Some(region) = patch.ranked_filter_region {
        let trimmed = region.trim();
        if !trimmed.is_empty() {
            nav.ranked_filter_region = trimmed.to_string();
        }
    }
    if let Some(tier) = patch.ranked_filter_tier {
        let trimmed = tier.trim();
        if !trimmed.is_empty() {
            nav.ranked_filter_tier = trimmed.to_string();
        }
    }
    if let Some(lane) = patch.ranked_filter_lane {
        nav.ranked_filter_lane = validate_ranked_lane(&lane);
    }
    if let Some(p) = patch.ranked_filter_patch {
        let trimmed = p.trim();
        nav.ranked_filter_patch = (!trimmed.is_empty()).then(|| trimmed.to_string());
    }
    if let Some(ct) = patch.ranked_champion_tier_filter {
        nav.ranked_champion_tier_filter = validate_champion_tier_filter(&ct);
    }
    if let Some(id) = patch.last_summoner_id
        && id > 0
    {
        nav.last_summoner_id = Some(id);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValidatedWindowBounds {
    pub width: f64,
    pub height: f64,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub maximized: bool,
    pub center: bool,
}

/// Validates saved window coordinates and dimensions against currently connected monitors
/// in logical coordinates so disconnected displays never spawn an off-screen window.
pub fn validate_window_bounds(
    win: &WindowSettings,
    monitors: &[MonitorRect],
) -> ValidatedWindowBounds {
    let mut width = if win.window_width.is_finite() {
        win.window_width.max(MIN_WINDOW_WIDTH)
    } else {
        DEFAULT_WINDOW_WIDTH
    };
    let mut height = if win.window_height.is_finite() {
        win.window_height.max(MIN_WINDOW_HEIGHT)
    } else {
        DEFAULT_WINDOW_HEIGHT
    };

    if monitors.is_empty() {
        return ValidatedWindowBounds {
            width,
            height,
            x: None,
            y: None,
            maximized: win.window_maximized,
            center: true,
        };
    }

    if let (Some(wx), Some(wy)) = (win.window_x, win.window_y)
        && wx.is_finite()
        && wy.is_finite()
    {
        // Check if at least 120x60 logical pixels intersect any active monitor
        let mut best_monitor: Option<&MonitorRect> = None;
        let mut best_overlap_area = 0.0f64;

        for m in monitors {
            let ix1 = wx.max(m.x);
            let iy1 = wy.max(m.y);
            let ix2 = (wx + width).min(m.x + m.width);
            let iy2 = (wy + height).min(m.y + m.height);
            let iw = (ix2 - ix1).max(0.0);
            let ih = (iy2 - iy1).max(0.0);
            if iw >= 120.0 && ih >= 60.0 {
                let area = iw * ih;
                if area > best_overlap_area {
                    best_overlap_area = area;
                    best_monitor = Some(m);
                }
            }
        }

        if let Some(m) = best_monitor {
            width = width.min(m.width.max(MIN_WINDOW_WIDTH));
            height = height.min(m.height.max(MIN_WINDOW_HEIGHT));
            // Keep titlebar reachable inside monitor top edge
            let clamped_y = wy.max(m.y);
            return ValidatedWindowBounds {
                width,
                height,
                x: Some(wx),
                y: Some(clamped_y),
                maximized: win.window_maximized,
                center: false,
            };
        }
    }

    let primary = &monitors[0];
    width = width.min(primary.width.max(MIN_WINDOW_WIDTH));
    height = height.min(primary.height.max(MIN_WINDOW_HEIGHT));

    ValidatedWindowBounds {
        width,
        height,
        x: None,
        y: None,
        maximized: win.window_maximized,
        center: true,
    }
}

#[derive(Default)]
struct SettingsStore {
    settings: AppSettings,
    ini_path: Option<PathBuf>,
}

static SETTINGS_STORE: OnceLock<RwLock<SettingsStore>> = OnceLock::new();

fn settings_lock() -> &'static RwLock<SettingsStore> {
    SETTINGS_STORE.get_or_init(|| RwLock::new(SettingsStore::default()))
}

pub fn init_global_settings(ini_path: PathBuf, settings: AppSettings) {
    if let Ok(mut guard) = settings_lock().write() {
        guard.ini_path = Some(ini_path);
        guard.settings = settings;
    }
    if let Some(lang) = get_global_settings().general.language {
        let mut prefs = get_global_settings().automation;
        prefs.language = lang;
        crate::lcu_injector::set_user_preferences(prefs);
    } else {
        crate::lcu_injector::set_user_preferences(get_global_settings().automation);
    }
}

pub fn get_global_settings() -> AppSettings {
    settings_lock()
        .read()
        .map(|g| g.settings.clone())
        .unwrap_or_default()
}

pub fn update_global_settings<F>(mutator: F) -> Result<AppSettings, String>
where
    F: FnOnce(&mut AppSettings),
{
    let (updated, path_opt) = {
        let mut guard = settings_lock()
            .write()
            .map_err(|_| "Settings lock poisoned".to_string())?;
        mutator(&mut guard.settings);
        (guard.settings.clone(), guard.ini_path.clone())
    };

    if let Some(path) = path_opt {
        save_user_settings_to_path(&path, &updated)?;
    }

    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ini_roundtrip_and_tolerant_manual_edit_fallback() {
        let mut s = AppSettings::default();
        s.general.language = Some(AppLocale::PtBr);
        s.general.language_onboarding_completed = true;
        s.general.legacy_storage_migrated = true;
        s.automation.auto_inject_runes = false;
        s.automation.flash_key = FlashKeyPreference::F;
        s.automation.auto_mark_daily_missions_read = true;
        s.automation.language = AppLocale::PtBr;
        s.profile_appearance.profile_blur_percent = 45;
        s.profile_appearance.wallpaper_visible = false;
        s.graphics.hardware_acceleration = false;
        s.window = WindowSettings {
            window_width: 1280.0,
            window_height: 800.0,
            window_x: Some(150.0),
            window_y: Some(80.0),
            window_maximized: true,
        };
        s.navigation = NavigationState {
            saved_aram_mode: Some(AramMode::AramMayhem),
            profile_match_mode_filter: "ranked_solo".to_string(),
            aram_mayhem_rarity_filter: "8".to_string(),
            aram_mayhem_last_champion: Some("ahri".to_string()),
            ranked_filter_region: "kr".to_string(),
            ranked_filter_tier: "master_plus".to_string(),
            ranked_filter_lane: "mid".to_string(),
            ranked_filter_patch: Some("16.21".to_string()),
            ranked_champion_tier_filter: "1".to_string(),
            last_summoner_id: Some(987654321),
        };

        let ini = serialize_user_settings_ini(&s);
        let parsed = parse_user_settings_ini(&ini);
        assert_eq!(parsed, s);

        // Corrupt / malformed manual edits fall back per-key without losing valid keys
        let edited = format!(
            "{ini}\n; user comment\n[/Script/SchumazGG.ProfileAppearanceSettings]\nProfileBlurPercent=notanumber\nbWallpaperVisible=True\n[/Script/SchumazGG.AutomationSettings]\nFlashKey=INVALID\n"
        );
        let parsed_edited = parse_user_settings_ini(&edited);
        assert_eq!(parsed_edited.profile_appearance.profile_blur_percent, 45);
        assert!(parsed_edited.profile_appearance.wallpaper_visible);
        assert_eq!(parsed_edited.automation.flash_key, FlashKeyPreference::F);

        let partial_corrupt = "
[/Script/SchumazGG.ProfileAppearanceSettings]
ProfileBlurPercent=abc
bWallpaperVisible=False
";
        let fallback_test = parse_user_settings_ini(partial_corrupt);
        assert_eq!(
            fallback_test.profile_appearance.profile_blur_percent,
            DEFAULT_PROFILE_BLUR_PERCENT
        );
        assert!(!fallback_test.profile_appearance.wallpaper_visible);
    }

    #[test]
    fn test_legacy_graphics_json_and_local_storage_migration() {
        let dir =
            std::env::temp_dir().join(format!("schumazgg_ini_migrate_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let ini_path = dir.join("UserSettings.ini");
        let legacy_gpu_path = dir.join("graphics_settings.json");
        fs::write(&legacy_gpu_path, r#"{ "hardware_acceleration": false }"#).unwrap();

        let mut settings = load_or_migrate_user_settings(&ini_path, Some(&legacy_gpu_path));
        assert!(!settings.graphics.hardware_acceleration);
        assert!(ini_path.exists());

        let legacy_ls = LegacyLocalStoragePayload {
            schumaz_user_preferences: Some(
                r#"{"autoInjectRunes":false,"autoInjectSpells":true,"autoInjectItems":false,"flashKey":"F","patch48hProtection":false,"autoMarkDailyMissionsRead":true,"language":"pt_BR"}"#
                    .to_string(),
            ),
            schumaz_language: Some("pt_BR".to_string()),
            schumaz_profile_ui_blur: Some("75".to_string()),
            schumaz_profile_wallpaper_visible: Some("false".to_string()),
            schumaz_aram_submode: Some("aram_mayhem".to_string()),
            schumaz_ranked_filters: Some(
                r#"{"region":"br","tier":"diamond_plus","lane":"jungle","patch":"16.21"}"#
                    .to_string(),
            ),
        };

        assert!(apply_legacy_local_storage_migration(
            &mut settings,
            &legacy_ls
        ));
        assert_eq!(settings.general.language, Some(AppLocale::PtBr));
        assert!(settings.general.language_onboarding_completed);
        assert!(settings.general.legacy_storage_migrated);
        assert!(!settings.automation.auto_inject_runes);
        assert_eq!(settings.automation.flash_key, FlashKeyPreference::F);
        assert!(!settings.automation.patch_48h_protection);
        assert!(settings.automation.auto_mark_daily_missions_read);
        assert_eq!(settings.profile_appearance.profile_blur_percent, 75);
        assert!(!settings.profile_appearance.wallpaper_visible);
        assert_eq!(
            settings.navigation.saved_aram_mode,
            Some(AramMode::AramMayhem)
        );
        assert_eq!(settings.navigation.ranked_filter_region, "br");
        assert_eq!(settings.navigation.ranked_filter_tier, "diamond_plus");
        assert_eq!(settings.navigation.ranked_filter_lane, "jungle");
        assert_eq!(
            settings.navigation.ranked_filter_patch.as_deref(),
            Some("16.21")
        );

        // Second migration call is a no-op
        assert!(!apply_legacy_local_storage_migration(
            &mut settings,
            &legacy_ls
        ));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_validate_window_bounds_multi_monitor_and_disconnected_fallback() {
        let monitors = vec![
            MonitorRect {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            },
            MonitorRect {
                x: 1920.0,
                y: 0.0,
                width: 1440.0,
                height: 900.0,
            },
        ];

        // Valid position on second monitor
        let on_second = WindowSettings {
            window_width: 1100.0,
            window_height: 760.0,
            window_x: Some(2050.0),
            window_y: Some(60.0),
            window_maximized: false,
        };
        let res = validate_window_bounds(&on_second, &monitors);
        assert_eq!(res.x, Some(2050.0));
        assert_eq!(res.y, Some(60.0));
        assert_eq!(res.width, 1100.0);
        assert_eq!(res.height, 760.0);
        assert!(!res.center);

        // Disconnected third monitor at x = 4000 -> centers on primary
        let offscreen = WindowSettings {
            window_width: 1050.0,
            window_height: 700.0,
            window_x: Some(4000.0),
            window_y: Some(200.0),
            window_maximized: true,
        };
        let res_off = validate_window_bounds(&offscreen, &monitors);
        assert_eq!(res_off.x, None);
        assert_eq!(res_off.y, None);
        assert!(res_off.center);
        assert!(res_off.maximized);
    }
}
