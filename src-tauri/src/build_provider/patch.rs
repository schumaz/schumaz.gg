
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
    if let Some(user_patch) = user_override
        && !user_patch.trim().is_empty()
    {
        return (user_patch.trim().to_string(), false);
    }

    if versions.is_empty() {
        return ("".to_string(), false);
    }

    let latest = &versions[0];
    if versions.len() < 2 {
        return (latest.clone(), false);
    }

    // High sample volume: if latest patch already has >= 1000 recorded games, it is mature
    if let Some(play) = play_count
        && play >= 1000
    {
        return (latest.clone(), false);
    }

    // If previous patch ended more than 48 hours ago, current patch has been active > 48h
    if let Some(prev_iso) = prev_patch_last_created_at_iso
        && let Some(prev_unix) = parse_iso8601_to_unix(prev_iso)
    {
        let age_secs = now_unix.saturating_sub(prev_unix);
        let two_days_secs = 2 * 86_400;
        if age_secs >= two_days_secs {
            return (latest.clone(), false);
        }
    }

    // If latest_created_at_iso indicates < 48h age and sample volume is low, fall back
    if let Some(created_at) = latest_created_at_iso
        && let Some(created_unix) = parse_iso8601_to_unix(created_at)
    {
        let age_secs = now_unix.saturating_sub(created_unix);
        let two_days_secs = 2 * 86_400; // 172,800s
        if age_secs < two_days_secs {
            return (versions[1].clone(), true);
        }
    }

    (latest.clone(), false)
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
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_iso8601_to_unix() {
        let ts = parse_iso8601_to_unix("2026-11-07T22:59:29+09:00");
        assert!(ts.is_some());
        let unix_secs = ts.unwrap();
        assert!(unix_secs > 1_700_000_000);
    }

    #[test]
    fn test_select_effective_patch_under_48h_falls_back() {
        let versions = vec!["16.22".to_string(), "16.21".to_string()];
        let created_at = "2026-11-07T22:59:29+09:00";
        let created_unix = parse_iso8601_to_unix(created_at).unwrap();
        let now_unix = created_unix + 36000; // 10h later

        let (patch, fallback) = select_effective_patch(&versions, Some(created_at), now_unix, None);
        assert_eq!(patch, "16.21");
        assert!(fallback);
    }

    #[test]
    fn test_select_effective_patch_over_48h_keeps_latest() {
        let versions = vec!["16.22".to_string(), "16.21".to_string()];
        let created_at = "2026-11-05T12:00:00Z";
        let created_unix = parse_iso8601_to_unix(created_at).unwrap();
        let now_unix = created_unix + (3 * 86400); // 3 days later

        let (patch, fallback) = select_effective_patch(&versions, Some(created_at), now_unix, None);
        assert_eq!(patch, "16.22");
        assert!(!fallback);
    }

    #[test]
    fn test_select_effective_patch_respects_user_override() {
        let versions = vec!["16.22".to_string(), "16.21".to_string()];
        let created_at = "2026-11-07T22:59:29+09:00";
        let created_unix = parse_iso8601_to_unix(created_at).unwrap();
        let now_unix = created_unix + 1000;

        let (patch, fallback) =
            select_effective_patch(&versions, Some(created_at), now_unix, Some("16.22"));
        assert_eq!(patch, "16.22");
        assert!(!fallback);
    }

    #[test]
    fn test_select_effective_patch_with_volume_keeps_latest_when_play_count_is_high() {
        let versions = vec!["16.22".to_string(), "16.21".to_string()];
        // Even if snapshot created_at was today (under 48h):
        let created_at = "2026-11-07T22:59:29+09:00";
        let created_unix = parse_iso8601_to_unix(created_at).unwrap();
        let now_unix = created_unix + 10000;

        // When play count is >= 1000, 16.22 is kept with fallback = false
        let (patch, fallback) = select_effective_patch_with_volume(
            &versions,
            Some(created_at),
            None,
            Some(84510),
            now_unix,
            None,
        );
        assert_eq!(patch, "16.22");
        assert!(!fallback);
    }
}
