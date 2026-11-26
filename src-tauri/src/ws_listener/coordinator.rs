use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

#[derive(Default)]
struct FinalizationTimerSlot {
    deadline: Option<tokio::time::Instant>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

/// Coordinates asynchronous Champ Select build/injection tasks so rapid champion switches,
/// finalization timers, or session exits immediately cancel stale work and prevent out-of-order LCU writes.
#[derive(Clone, Default)]
pub struct ChampSelectCoordinator {
    selection_seq: Arc<AtomicU64>,
    active_inject_task: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
    finalization_timer: Arc<tokio::sync::Mutex<FinalizationTimerSlot>>,
    inject_lock: Arc<tokio::sync::Mutex<()>>,
}

impl ChampSelectCoordinator {
    #[cfg(test)]
    pub(crate) fn current_seq(&self) -> u64 {
        self.selection_seq.load(Ordering::SeqCst)
    }

    pub(crate) fn is_current(&self, seq: u64) -> bool {
        self.selection_seq.load(Ordering::SeqCst) == seq
    }

    pub(crate) async fn claim_next_seq(&self) -> u64 {
        let seq = self.selection_seq.fetch_add(1, Ordering::SeqCst) + 1;
        let mut guard = self.active_inject_task.lock().await;
        if let Some(prev_handle) = guard.take() {
            prev_handle.abort();
        }
        seq
    }

    pub(crate) async fn lock_injection(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.inject_lock.lock().await
    }

    pub(crate) async fn cancel_pending(&self) {
        self.selection_seq.fetch_add(1, Ordering::SeqCst);
        let mut guard = self.active_inject_task.lock().await;
        if let Some(handle) = guard.take() {
            handle.abort();
        }
    }

    pub(crate) async fn cancel_finalization_timer(&self) {
        let mut slot = self.finalization_timer.lock().await;
        slot.deadline = None;
        if let Some(handle) = slot.handle.take() {
            handle.abort();
        }
    }

    pub(crate) async fn schedule_finalization_timer<F, Fut>(&self, delay: Duration, make_task: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let desired_deadline = tokio::time::Instant::now() + delay;
        let mut slot = self.finalization_timer.lock().await;

        if let (Some(existing_deadline), Some(existing_handle)) =
            (slot.deadline, slot.handle.as_ref())
            && !existing_handle.is_finished()
        {
            let diff = desired_deadline.saturating_duration_since(existing_deadline)
                + existing_deadline.saturating_duration_since(desired_deadline);
            if diff <= Duration::from_millis(1200) {
                return;
            }
        }

        if let Some(prev) = slot.handle.take() {
            prev.abort();
        }

        slot.deadline = Some(desired_deadline);
        let handle = tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            make_task().await;
        });
        slot.handle = Some(handle);
    }

    pub(crate) async fn spawn_latest<F, Fut>(&self, make_task: F) -> u64
    where
        F: FnOnce(u64, ChampSelectCoordinator) -> Fut,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let seq = self.selection_seq.fetch_add(1, Ordering::SeqCst) + 1;
        let mut guard = self.active_inject_task.lock().await;
        if let Some(prev_handle) = guard.take() {
            prev_handle.abort();
        }
        let coordinator = self.clone();
        let handle = tokio::spawn(make_task(seq, coordinator));
        *guard = Some(handle);
        seq
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coordinator_cancels_stale_task_on_rapid_champion_switch() {
        let coord = ChampSelectCoordinator::default();
        let completed = Arc::new(tokio::sync::Mutex::new(Vec::<i32>::new()));

        let c1 = Arc::clone(&completed);
        let seq1 = coord
            .spawn_latest(move |my_seq, seq_guard| async move {
                tokio::time::sleep(Duration::from_millis(80)).await;
                if seq_guard.is_current(my_seq) {
                    c1.lock().await.push(18);
                }
            })
            .await;

        let c2 = Arc::clone(&completed);
        let seq2 = coord
            .spawn_latest(move |my_seq, seq_guard| async move {
                tokio::time::sleep(Duration::from_millis(15)).await;
                if seq_guard.is_current(my_seq) {
                    c2.lock().await.push(103);
                }
            })
            .await;

        assert_ne!(seq1, seq2);
        assert!(!coord.is_current(seq1));
        assert!(coord.is_current(seq2));

        tokio::time::sleep(Duration::from_millis(120)).await;
        let finished = completed.lock().await.clone();
        assert_eq!(finished, vec![103]);
    }

    #[tokio::test]
    async fn test_coordinator_cancel_pending_invalidates_sequence_on_champ_select_end() {
        let coord = ChampSelectCoordinator::default();
        let completed = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let c1 = Arc::clone(&completed);
        let seq1 = coord
            .spawn_latest(move |my_seq, seq_guard| async move {
                tokio::time::sleep(Duration::from_millis(60)).await;
                if seq_guard.is_current(my_seq) {
                    c1.store(true, Ordering::SeqCst);
                }
            })
            .await;

        assert!(coord.is_current(seq1));
        coord.cancel_pending().await;
        assert!(!coord.is_current(seq1));
        assert_ne!(coord.current_seq(), seq1);

        tokio::time::sleep(Duration::from_millis(80)).await;
        assert!(!completed.load(Ordering::SeqCst));
    }
}
