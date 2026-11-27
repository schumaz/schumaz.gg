use std::cmp::Ordering;

/// Default cooldown between automatic startup update checks (24 hours).
pub const AUTO_UPDATE_CHECK_COOLDOWN_SECS: u64 = 24 * 60 * 60;

/// Returns `true` when an automatic update check is due.
///
/// Treats missing timestamps or backwards system clock skew (`now < last`) as expired
/// so the updater recovers gracefully instead of staying stuck indefinitely.
pub fn should_run_auto_check(
    last_check_epoch_secs: Option<u64>,
    now_epoch_secs: u64,
    cooldown_secs: u64,
) -> bool {
    let Some(last) = last_check_epoch_secs else {
        return true;
    };
    if now_epoch_secs < last {
        return true;
    }
    now_epoch_secs.saturating_sub(last) >= cooldown_secs
}

/// Extracts the release tag segment from a GitHub `Location` redirect header.
///
/// Supports both absolute (`https://github.com/<owner>/<repo>/releases/tag/v0.5.0`)
/// and relative (`/<owner>/<repo>/releases/tag/v0.5.0`) paths.
pub fn extract_release_tag_from_location(location: &str) -> Option<String> {
    let without_fragment = location.trim().split('#').next().unwrap_or("");
    let without_query = without_fragment.split('?').next().unwrap_or("");
    let trimmed = without_query.trim_end_matches('/');
    let (_, after_tag) = trimmed.rsplit_once("/releases/tag/")?;
    let tag = after_tag.trim();
    if tag.is_empty() || tag.contains('/') {
        return None;
    }
    Some(tag.to_string())
}

/// Normalizes a release tag (e.g. `v0.5.0` or `V0.5.0-beta.1`) into a bare SemVer string.
pub fn normalize_semver_tag(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let bare = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    if bare.is_empty() {
        return None;
    }
    parse_semver(bare)?;
    Some(bare.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedSemver {
    major: u64,
    minor: u64,
    patch: u64,
    pre: Option<String>,
}

fn parse_semver(raw: &str) -> Option<ParsedSemver> {
    let trimmed = raw.trim();
    let bare = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    let without_build = bare.split('+').next().unwrap_or("");
    let (core, pre) = match without_build.split_once('-') {
        Some((c, p)) if !p.is_empty() => (c, Some(p.to_string())),
        Some(_) => return None,
        None => (without_build, None),
    };

    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }

    Some(ParsedSemver {
        major,
        minor,
        patch,
        pre,
    })
}

fn compare_prerelease(left: &str, right: &str) -> Ordering {
    let mut left_ids = left.split('.');
    let mut right_ids = right.split('.');

    loop {
        match (left_ids.next(), right_ids.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(l), Some(r)) => {
                let ord = match (l.parse::<u64>(), r.parse::<u64>()) {
                    (Ok(ln), Ok(rn)) => ln.cmp(&rn),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => l.cmp(r),
                };
                if ord != Ordering::Equal {
                    return ord;
                }
            }
        }
    }
}

fn compare_semver(left: &ParsedSemver, right: &ParsedSemver) -> Ordering {
    (left.major, left.minor, left.patch)
        .cmp(&(right.major, right.minor, right.patch))
        .then_with(|| match (&left.pre, &right.pre) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(l), Some(r)) => compare_prerelease(l, r),
        })
}

/// Returns `Some(true)` when `remote` is strictly newer than `current` per SemVer 2.0.0.
pub fn is_remote_version_newer(current: &str, remote: &str) -> Option<bool> {
    let curr = parse_semver(current)?;
    let rem = parse_semver(remote)?;
    Some(compare_semver(&rem, &curr) == Ordering::Greater)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_run_auto_check_cooldown_and_clock_skew() {
        let cooldown = AUTO_UPDATE_CHECK_COOLDOWN_SECS;
        assert!(should_run_auto_check(None, 1_000_000, cooldown));
        assert!(!should_run_auto_check(
            Some(1_000_000),
            1_000_000 + cooldown - 1,
            cooldown
        ));
        assert!(should_run_auto_check(
            Some(1_000_000),
            1_000_000 + cooldown,
            cooldown
        ));
        // Backwards clock skew resets cooldown safely
        assert!(should_run_auto_check(Some(2_000_000), 1_000_000, cooldown));
    }

    #[test]
    fn test_extract_release_tag_from_location() {
        assert_eq!(
            extract_release_tag_from_location(
                "https://github.com/schumaz/schumaz.gg/releases/tag/v0.5.0"
            )
            .as_deref(),
            Some("v0.5.0")
        );
        assert_eq!(
            extract_release_tag_from_location(
                "/schumaz/schumaz.gg/releases/tag/v0.5.0-beta.2/?ref=1#top"
            )
            .as_deref(),
            Some("v0.5.0-beta.2")
        );
        assert_eq!(
            extract_release_tag_from_location("https://github.com/schumaz/schumaz.gg/releases"),
            None
        );
    }

    #[test]
    fn test_semver_comparison_and_normalization() {
        assert_eq!(normalize_semver_tag("v0.5.0").as_deref(), Some("0.5.0"));
        assert_eq!(normalize_semver_tag("invalid"), None);
        assert_eq!(is_remote_version_newer("0.4.0", "v0.4.1"), Some(true));
        assert_eq!(is_remote_version_newer("0.4.0", "v0.4.0"), Some(false));
        assert_eq!(is_remote_version_newer("0.5.0", "v0.4.0"), Some(false));
        assert_eq!(
            is_remote_version_newer("0.4.0-beta.1", "v0.4.0"),
            Some(true)
        );
        assert_eq!(
            is_remote_version_newer("0.4.0-beta.1", "v0.4.0-beta.2"),
            Some(true)
        );
        assert_eq!(
            is_remote_version_newer("0.4.0", "v0.4.0-beta.1"),
            Some(false)
        );
    }
}
