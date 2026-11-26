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

fn parse_ascii_uint(bytes: &[u8]) -> Option<u32> {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let mut acc: u32 = 0;
    for &b in bytes {
        acc = acc.checked_mul(10)?.checked_add(u32::from(b - b'0'))?;
    }
    Some(acc)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) { 29 } else { 28 },
        _ => 0,
    }
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400) as u64;
    let (m, d) = (u64::from(month), u64::from(day));
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

fn parse_iso8601_offset(mut rest: &[u8]) -> Option<i64> {
    if let Some(after_dot) = rest.strip_prefix(b".").or_else(|| rest.strip_prefix(b",")) {
        let frac_digits = after_dot.iter().take_while(|b| b.is_ascii_digit()).count();
        if frac_digits == 0 {
            return None;
        }
        rest = &after_dot[frac_digits..];
    }

    match rest {
        b"" | b"Z" | b"z" => Some(0),
        [sign_byte @ (b'+' | b'-'), digits @ ..] => {
            let sign: i64 = if *sign_byte == b'-' { -1 } else { 1 };
            let (off_h, off_m) = match digits {
                [h1, h2, b':', m1, m2] | [h1, h2, m1, m2] => (
                    parse_ascii_uint(&[*h1, *h2])?,
                    parse_ascii_uint(&[*m1, *m2])?,
                ),
                [h1, h2] => (parse_ascii_uint(&[*h1, *h2])?, 0),
                _ => return None,
            };
            if off_h > 23 || off_m > 59 {
                return None;
            }
            Some(sign * (i64::from(off_h) * 3600 + i64::from(off_m) * 60))
        }
        _ => None,
    }
}

/// Parses an ISO 8601 timestamp string (e.g. "2026-11-07T22:59:29+09:00",
/// "2026-11-07T22:59:29.123+09:00", or "2026-11-07T22:59:29Z") into a Unix epoch timestamp (seconds).
pub fn parse_iso8601_to_unix(iso_str: &str) -> Option<u64> {
    let iso = iso_str.trim();
    if !iso.is_ascii() || iso.len() < 19 {
        return None;
    }
    let b = iso.as_bytes();
    if b[4] != b'-'
        || b[7] != b'-'
        || !matches!(b[10], b'T' | b't' | b' ')
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }

    let year = parse_ascii_uint(&b[0..4])? as i32;
    let month = parse_ascii_uint(&b[5..7])?;
    let day = parse_ascii_uint(&b[8..10])?;
    let hour = parse_ascii_uint(&b[11..13])?;
    let min = parse_ascii_uint(&b[14..16])?;
    let sec = parse_ascii_uint(&b[17..19])?;

    let max_day = days_in_month(year, month);
    if max_day == 0 || !(1..=max_day).contains(&day) || hour > 23 || min > 59 || sec > 59 {
        return None;
    }

    let offset_secs = parse_iso8601_offset(&b[19..])?;
    let total_days = days_from_civil(i64::from(year), month, day);
    let total_secs_utc =
        total_days * 86_400 + i64::from(hour) * 3_600 + i64::from(min) * 60 + i64::from(sec)
            - offset_secs;

    u64::try_from(total_secs_utc).ok()
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
