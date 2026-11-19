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
fn test_resolve_effective_patch_exposes_remaining_hours_and_seconds() {
    let versions = vec!["16.22".to_string(), "16.21".to_string()];
    let created_at = "2026-11-07T12:00:00Z";
    let created_unix = parse_iso8601_to_unix(created_at).unwrap();

    // 10h elapsed -> 38h remaining
    let res_10h = resolve_effective_patch(
        &versions,
        Some(created_at),
        None,
        Some(250),
        created_unix + 10 * 3600,
        None,
        true,
    );
    assert_eq!(res_10h.effective_patch, "16.21");
    assert!(res_10h.fallback_applied);
    assert_eq!(res_10h.remaining_hours, Some(38));
    assert_eq!(res_10h.remaining_seconds, Some(38 * 3600));

    // 47h 30m elapsed -> 1800s left -> rounds up to 1h remaining
    let res_30m = resolve_effective_patch(
        &versions,
        Some(created_at),
        None,
        Some(250),
        created_unix + 47 * 3600 + 1800,
        None,
        true,
    );
    assert_eq!(res_30m.effective_patch, "16.21");
    assert!(res_30m.fallback_applied);
    assert_eq!(res_30m.remaining_hours, Some(1));
    assert_eq!(res_30m.remaining_seconds, Some(1800));
}

#[test]
fn test_resolve_effective_patch_when_protection_disabled_selects_latest() {
    let versions = vec!["16.22".to_string(), "16.21".to_string()];
    let created_at = "2026-11-07T12:00:00Z";
    let created_unix = parse_iso8601_to_unix(created_at).unwrap();
    let now_unix = created_unix + 3600; // Only 1h old, low volume

    let res = resolve_effective_patch(
        &versions,
        Some(created_at),
        None,
        Some(100),
        now_unix,
        None,
        false,
    );
    assert_eq!(res.effective_patch, "16.22");
    assert!(!res.fallback_applied);
    assert_eq!(res.remaining_hours, None);
    assert_eq!(res.remaining_seconds, None);
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

    let res = resolve_effective_patch(
        &versions,
        Some(created_at),
        None,
        None,
        now_unix,
        Some("16.22"),
        true,
    );
    assert_eq!(res.effective_patch, "16.22");
    assert!(!res.fallback_applied);
    assert_eq!(res.remaining_hours, None);
    assert_eq!(res.remaining_seconds, None);
}

#[test]
fn test_select_effective_patch_with_volume_keeps_latest_when_play_count_is_high() {
    let versions = vec!["16.22".to_string(), "16.21".to_string()];
    // Even if snapshot created_at was today (under 48h):
    let created_at = "2026-11-07T22:59:29+09:00";
    let created_unix = parse_iso8601_to_unix(created_at).unwrap();
    let now_unix = created_unix + 10000;

    // When play count is >= 1000, 16.22 is kept with fallback = false
    let res = resolve_effective_patch(
        &versions,
        Some(created_at),
        None,
        Some(84510),
        now_unix,
        None,
        true,
    );
    assert_eq!(res.effective_patch, "16.22");
    assert!(!res.fallback_applied);
    assert_eq!(res.remaining_hours, None);
}

#[test]
fn test_irelia_16_23_prioritizes_latest_under_48h_over_frozen_prev_patch_timestamp() {
    // Regression: 16.23 has only 13h age and 79 games while 16.22's frozen snapshot is >72h old.
    let versions = vec!["16.23".to_string(), "16.22".to_string()];
    let latest_created = "2026-11-18T12:00:00Z";
    let prev_frozen = "2026-11-15T12:00:00Z";
    let latest_unix = parse_iso8601_to_unix(latest_created).unwrap();
    let now_unix = latest_unix + 13 * 3600;

    let res = resolve_effective_patch(
        &versions,
        Some(latest_created),
        Some(prev_frozen),
        Some(79),
        now_unix,
        None,
        true,
    );
    assert_eq!(res.effective_patch, "16.22");
    assert!(res.fallback_applied);
    assert_eq!(res.remaining_hours, Some(35));
    assert_eq!(res.remaining_seconds, Some(35 * 3600));
}

#[test]
fn test_extract_patch_trend_timestamps_validates_version_and_handles_zero_game_new_patch() {
    // Patch 16.23 just dropped with 0 games so trends.win only lists 16.22 and 16.21
    let zero_game_payload = serde_json::json!({
        "data": {
            "trends": {
                "win": [
                    { "version": "16.22", "created_at": "2026-11-15T12:00:00Z" },
                    { "version": "16.21", "created_at": "2026-11-01T12:00:00Z" }
                ]
            }
        }
    });

    let (latest_iso, prev_iso) =
        extract_patch_trend_timestamps(&zero_game_payload, "16.23", Some("16.22"));
    assert_eq!(latest_iso, None);
    assert_eq!(prev_iso, Some("2026-11-15T12:00:00Z"));

    let versions = vec!["16.23".to_string(), "16.22".to_string()];
    let now_unix = parse_iso8601_to_unix("2026-11-19T01:00:00Z").unwrap();

    // With 0 games (< MIN_MATURE_SAMPLE), must fall back to 16.22 even though 16.22 is >48h old
    let res = resolve_effective_patch(
        &versions,
        latest_iso,
        prev_iso,
        Some(0),
        now_unix,
        None,
        true,
    );
    assert_eq!(res.effective_patch, "16.22");
    assert!(res.fallback_applied);

    // Same when play_count is None or < MIN_MATURE_SAMPLE (e.g. 18 games)
    let res_low = resolve_effective_patch(
        &versions,
        latest_iso,
        prev_iso,
        Some(MIN_MATURE_SAMPLE - 1),
        now_unix,
        None,
        true,
    );
    assert_eq!(res_low.effective_patch, "16.22");
    assert!(res_low.fallback_applied);
}
