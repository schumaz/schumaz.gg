use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const MAX_AUTO_MARK_ATTEMPTS_PER_ID: u8 = 2;
const AUTO_MARK_PUT_COOLDOWN: Duration = Duration::from_millis(800);

static AUTO_MARK_TRACKER: OnceLock<Mutex<AutoMarkTracker>> = OnceLock::new();

#[derive(Default)]
pub(super) struct AutoMarkTracker {
    /// Maps mission or series ID -> (attempt_count, last_is_new_flag) for the current LCU session.
    attempts: HashMap<String, (u8, bool)>,
    last_put_at: Option<Instant>,
}

pub(super) fn auto_mark_tracker() -> &'static Mutex<AutoMarkTracker> {
    AUTO_MARK_TRACKER.get_or_init(|| Mutex::new(AutoMarkTracker::default()))
}

/// Resets per-session mission auto-mark retry tracking (called on LCU disconnect or manual re-enable).
pub fn reset_auto_mark_session_state() {
    if let Ok(mut guard) = auto_mark_tracker().lock() {
        guard.attempts.clear();
        guard.last_put_at = None;
    }
}

pub(super) fn filter_candidates_with_tracker(
    tracker: &mut AutoMarkTracker,
    mission_candidates: &[(String, bool)],
    series_candidates: &[String],
    now: Instant,
) -> (Vec<(String, bool)>, Vec<String>) {
    if let Some(last_put) = tracker.last_put_at
        && now.saturating_duration_since(last_put) < AUTO_MARK_PUT_COOLDOWN
    {
        return (Vec::new(), Vec::new());
    }

    let active_keys: HashSet<&str> = mission_candidates
        .iter()
        .map(|(id, _)| id.as_str())
        .chain(series_candidates.iter().map(String::as_str))
        .collect();
    // Prune IDs that are now cleared in LCU so future reappearances are handled cleanly.
    tracker
        .attempts
        .retain(|key, _| active_keys.contains(key.as_str()));

    let mut missions_to_send = Vec::new();
    for (id, is_new) in mission_candidates {
        match tracker.attempts.get_mut(id) {
            Some((count, prev_is_new)) => {
                if *is_new && !*prev_is_new {
                    *count = 0;
                    *prev_is_new = true;
                    missions_to_send.push((id.clone(), *is_new));
                } else if *count < MAX_AUTO_MARK_ATTEMPTS_PER_ID {
                    missions_to_send.push((id.clone(), *is_new));
                }
            }
            None => {
                missions_to_send.push((id.clone(), *is_new));
            }
        }
    }

    let mut series_to_send = Vec::new();
    for id in series_candidates {
        match tracker.attempts.get(id) {
            Some((count, _)) if *count >= MAX_AUTO_MARK_ATTEMPTS_PER_ID => {}
            _ => series_to_send.push(id.clone()),
        }
    }

    (missions_to_send, series_to_send)
}

pub(super) fn record_sent_with_tracker(
    tracker: &mut AutoMarkTracker,
    missions_sent: &[(String, bool)],
    series_sent: &[String],
    now: Instant,
) {
    for (id, is_new) in missions_sent {
        let entry = tracker.attempts.entry(id.clone()).or_insert((0, *is_new));
        entry.0 = entry.0.saturating_add(1);
        entry.1 = *is_new;
    }
    for id in series_sent {
        let entry = tracker.attempts.entry(id.clone()).or_insert((0, false));
        entry.0 = entry.0.saturating_add(1);
    }
    tracker.last_put_at = Some(now);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_mark_tracker_prevents_infinite_loops_and_resets_when_cleared() {
        let mut tracker = AutoMarkTracker::default();
        let t0 = Instant::now();

        let candidates = vec![("m1".to_string(), true), ("m2".to_string(), false)];
        let series = vec!["s1".to_string()];

        // Attempt 1
        let (m_send, s_send) =
            filter_candidates_with_tracker(&mut tracker, &candidates, &series, t0);
        assert_eq!(m_send.len(), 2);
        assert_eq!(s_send.len(), 1);
        record_sent_with_tracker(&mut tracker, &m_send, &s_send, t0);

        // Within cooldown -> blocked
        let (m_cool, s_cool) = filter_candidates_with_tracker(
            &mut tracker,
            &candidates,
            &series,
            t0 + Duration::from_millis(200),
        );
        assert!(m_cool.is_empty() && s_cool.is_empty());

        // Attempt 2 after cooldown
        let t1 = t0 + Duration::from_secs(2);
        let (m_send2, s_send2) =
            filter_candidates_with_tracker(&mut tracker, &candidates, &series, t1);
        assert_eq!(m_send2.len(), 2);
        assert_eq!(s_send2.len(), 1);
        record_sent_with_tracker(&mut tracker, &m_send2, &s_send2, t1);

        // Attempt 3 -> capped to avoid PUT loop if LCU refuses to flip an optional item
        let t2 = t1 + Duration::from_secs(2);
        let (m_send3, s_send3) =
            filter_candidates_with_tracker(&mut tracker, &candidates, &series, t2);
        assert!(m_send3.is_empty() && s_send3.is_empty());

        // Once m2 clears (disappears from unread list), it is pruned; if it returns with isNew=true later, it runs again
        let t3 = t2 + Duration::from_secs(2);
        let (m_send4, _) =
            filter_candidates_with_tracker(&mut tracker, &[("m1".to_string(), true)], &series, t3);
        assert!(m_send4.is_empty());
        assert!(!tracker.attempts.contains_key("m2"));

        let t4 = t3 + Duration::from_secs(2);
        let (m_send5, _) = filter_candidates_with_tracker(
            &mut tracker,
            &[("m1".to_string(), true), ("m2".to_string(), true)],
            &series,
            t4,
        );
        assert_eq!(m_send5, vec![("m2".to_string(), true)]);
    }
}
