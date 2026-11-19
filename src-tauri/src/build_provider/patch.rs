pub const PATCH_PROTECTION_SECS: u64 = 2 * 86_400; // 48 hours = 172,800s
pub const PATCH_HIGH_VOLUME_SAMPLE: u64 = 1000;
pub const MIN_MATURE_SAMPLE: u64 = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchSelectionResult {
    pub effective_patch: String,
    pub fallback_applied: bool,
    pub remaining_hours: Option<u32>,
    pub remaining_seconds: Option<u64>,
}

/// Extracts `(latest_created_at_iso, prev_patch_last_created_at_iso)` from `data.trends.win`
/// (or top-level `trends.win`), matching each entry's `version` field when present so that
/// an unpopulated newly-released patch never inherits the previous patch's frozen timestamp.
pub fn extract_patch_trend_timestamps<'a>(
    analysis_json: &'a serde_json::Value,
    latest_patch: &str,
    prev_patch: Option<&str>,
) -> (Option<&'a str>, Option<&'a str>) {
    let Some(win_trends) = analysis_json
        .pointer("/data/trends/win")
        .or_else(|| analysis_json.pointer("/trends/win"))
        .and_then(|v| v.as_array())
    else {
        return (None, None);
    };

    let has_version_fields = win_trends.iter().any(|entry| {
        entry
            .get("version")
            .and_then(|v| v.as_str())
            .is_some_and(|v| !v.trim().is_empty())
    });

    if has_version_fields {
        let latest_trimmed = latest_patch.trim();
        let prev_trimmed = prev_patch.map(str::trim).filter(|s| !s.is_empty());

        let latest_iso = if latest_trimmed.is_empty() {
            None
        } else {
            win_trends.iter().find_map(|entry| {
                let ver = entry.get("version").and_then(|v| v.as_str())?.trim();
                if ver == latest_trimmed {
                    entry.get("created_at").and_then(|v| v.as_str())
                } else {
                    None
                }
            })
        };

        let prev_iso = prev_trimmed.and_then(|prev| {
            win_trends.iter().find_map(|entry| {
                let ver = entry.get("version").and_then(|v| v.as_str())?.trim();
                if ver == prev {
                    entry.get("created_at").and_then(|v| v.as_str())
                } else {
                    None
                }
            })
        });

        (latest_iso, prev_iso)
    } else {
        let latest_iso = win_trends
            .first()
            .and_then(|entry| entry.get("created_at"))
            .and_then(|v| v.as_str());
        let prev_iso = win_trends
            .get(1)
            .and_then(|entry| entry.get("created_at"))
            .and_then(|v| v.as_str());
        (latest_iso, prev_iso)
    }
}

/// Parses an ISO 8601 timestamp string (e.g. "2026-11-07T22:59:29+09:00" or "2026-11-07T22:59:29Z")
/// into a Unix epoch timestamp (seconds).
pub fn parse_iso8601_to_unix(iso_str: &str) -> Option<u64> {
    if iso_str.len() < 19 {
        return None;
    }
    let year: i32 = iso_str[0..4].parse().ok()?;
    let month: u32 = iso_str[5..7].parse().ok()?;
    let day: u32 = iso_str[8..10].parse().ok()?;
    let hour: u32 = iso_str[11..13].parse().ok()?;
    let min: u32 = iso_str[14..16].parse().ok()?;
    let sec: u32 = iso_str[17..19].parse().ok()?;

    let mut offset_secs: i64 = 0;
    if iso_str.len() > 19 {
        let rest = &iso_str[19..];
        if rest.starts_with('+') || rest.starts_with('-') {
            let sign = if rest.starts_with('-') { -1 } else { 1 };
            let off_str = &rest[1..];
            let parts: Vec<&str> = off_str.split(':').collect();
            if !parts.is_empty() {
                let off_h: i64 = parts[0].parse().unwrap_or(0);
                let off_m: i64 = parts.get(1).and_then(|m| m.parse().ok()).unwrap_or(0);
                offset_secs = sign * (off_h * 3600 + off_m * 60);
            }
        }
    }

    let days_before_month = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let is_leap = |y: i32| (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);

    let mut total_days: i64 = 0;
    for y in 1970..year {
        total_days += if is_leap(y) { 366 } else { 365 };
    }
    total_days += days_before_month[(month - 1) as usize] as i64;
    if month > 2 && is_leap(year) {
        total_days += 1;
    }
    total_days += (day - 1) as i64;

    let total_secs_utc =
        total_days * 86400 + (hour as i64) * 3600 + (min as i64) * 60 + (sec as i64) - offset_secs;
    if total_secs_utc >= 0 {
        Some(total_secs_utc as u64)
    } else {
        None
    }
}

fn fallback_result(prev_patch: &str, age_secs: Option<u64>) -> PatchSelectionResult {
    let (remaining_hours, remaining_seconds) = match age_secs {
        Some(age) if age < PATCH_PROTECTION_SECS => {
            let rem_secs = PATCH_PROTECTION_SECS - age;
            (Some(rem_secs.div_ceil(3600) as u32), Some(rem_secs))
        }
        _ => (None, None),
    };
    PatchSelectionResult {
        effective_patch: prev_patch.to_string(),
        fallback_applied: true,
        remaining_hours,
        remaining_seconds,
    }
}

/// Resolves the effective patch respecting the 48h protection rule (when enabled),
/// manual user overrides, sample volume maturity, and exposes remaining protection time.
pub fn resolve_effective_patch(
    versions: &[String],
    latest_created_at_iso: Option<&str>,
    prev_patch_last_created_at_iso: Option<&str>,
    play_count: Option<u64>,
    now_unix: u64,
    user_override: Option<&str>,
    protection_enabled: bool,
) -> PatchSelectionResult {
    if let Some(user_patch) = user_override
        && !user_patch.trim().is_empty()
    {
        return PatchSelectionResult {
            effective_patch: user_patch.trim().to_string(),
            fallback_applied: false,
            remaining_hours: None,
            remaining_seconds: None,
        };
    }

    if versions.is_empty() {
        return PatchSelectionResult {
            effective_patch: String::new(),
            fallback_applied: false,
            remaining_hours: None,
            remaining_seconds: None,
        };
    }

    let latest = &versions[0];
    if !protection_enabled || versions.len() < 2 {
        return PatchSelectionResult {
            effective_patch: latest.clone(),
            fallback_applied: false,
            remaining_hours: None,
            remaining_seconds: None,
        };
    }

    let previous = &versions[1];

    // High sample volume: if latest patch already has >= 1000 recorded games, it is mature
    if let Some(play) = play_count
        && play >= PATCH_HIGH_VOLUME_SAMPLE
    {
        return PatchSelectionResult {
            effective_patch: latest.clone(),
            fallback_applied: false,
            remaining_hours: None,
            remaining_seconds: None,
        };
    }

    // Priority 1: evaluate latest patch creation timestamp first so a frozen older timestamp
    // from the previous patch can never bypass the 48h protection window.
    if let Some(created_at) = latest_created_at_iso
        && let Some(created_unix) = parse_iso8601_to_unix(created_at)
    {
        let age_secs = now_unix.saturating_sub(created_unix);
        if age_secs < PATCH_PROTECTION_SECS {
            return fallback_result(previous, Some(age_secs));
        }
        // Even past 48h, fall back if explicit sample count is below minimum maturity threshold.
        if let Some(play) = play_count
            && play < MIN_MATURE_SAMPLE
        {
            return fallback_result(previous, None);
        }
        return PatchSelectionResult {
            effective_patch: latest.clone(),
            fallback_applied: false,
            remaining_hours: None,
            remaining_seconds: None,
        };
    }

    // Priority 2: latest patch has no trend timestamp yet (e.g. just launched with 0 or few games).
    // Fall back whenever sample count is below MIN_MATURE_SAMPLE or previous patch ended < 48h ago.
    let prev_age_secs = prev_patch_last_created_at_iso
        .and_then(parse_iso8601_to_unix)
        .map(|prev_unix| now_unix.saturating_sub(prev_unix));

    if play_count.unwrap_or(0) < MIN_MATURE_SAMPLE {
        return fallback_result(previous, prev_age_secs);
    }

    if let Some(age_secs) = prev_age_secs
        && age_secs < PATCH_PROTECTION_SECS
    {
        return fallback_result(previous, Some(age_secs));
    }

    PatchSelectionResult {
        effective_patch: latest.clone(),
        fallback_applied: false,
        remaining_hours: None,
        remaining_seconds: None,
    }
}

/// Selects the effective patch respecting the 2-day maturity rule:
/// If the latest patch was released in the last 2 days (< 48h) with low sample volume,
/// automatically select previous patch to guarantee high sample volume.
pub fn select_effective_patch_with_volume(
    versions: &[String],
    latest_created_at_iso: Option<&str>,
    prev_patch_last_created_at_iso: Option<&str>,
    play_count: Option<u64>,
    now_unix: u64,
    user_override: Option<&str>,
) -> (String, bool) {
    let res = resolve_effective_patch(
        versions,
        latest_created_at_iso,
        prev_patch_last_created_at_iso,
        play_count,
        now_unix,
        user_override,
        true,
    );
    (res.effective_patch, res.fallback_applied)
}

pub fn select_effective_patch(
    versions: &[String],
    latest_created_at_iso: Option<&str>,
    now_unix: u64,
    user_override: Option<&str>,
) -> (String, bool) {
    select_effective_patch_with_volume(
        versions,
        latest_created_at_iso,
        None,
        None,
        now_unix,
        user_override,
    )
}

#[cfg(test)]
#[path = "patch_tests.rs"]
mod tests;
