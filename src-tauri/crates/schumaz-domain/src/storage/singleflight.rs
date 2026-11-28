use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::watch;

const MAX_SINGLEFLIGHT_RETRIES: usize = 4;

struct InFlightEntry<V> {
    id: u64,
    tx: watch::Sender<Option<V>>,
}

/// Coalesces concurrent asynchronous requests for the same key `K` into a single execution.
pub struct Singleflight<K, V> {
    next_id: AtomicU64,
    in_flight: Mutex<HashMap<K, InFlightEntry<V>>>,
}

impl<K, V> Default for Singleflight<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

pub enum FlightStep<'a, K, V>
where
    K: Eq + Hash + Clone,
{
    Lead(InFlightGuard<'a, K, V>),
    Wait(watch::Receiver<Option<V>>),
}

pub struct InFlightGuard<'a, K, V>
where
    K: Eq + Hash + Clone,
{
    flight: &'a Singleflight<K, V>,
    key: K,
    id: u64,
    tx: watch::Sender<Option<V>>,
    completed: bool,
}

impl<K, V> Drop for InFlightGuard<'_, K, V>
where
    K: Eq + Hash + Clone,
{
    fn drop(&mut self) {
        if !self.completed {
            self.flight.remove_if_matches(&self.key, self.id);
        }
    }
}

impl<K, V> InFlightGuard<'_, K, V>
where
    K: Eq + Hash + Clone,
{
    pub fn complete(mut self, value: V) {
        self.flight.remove_if_matches(&self.key, self.id);
        let _ = self.tx.send(Some(value));
        self.completed = true;
    }
}

impl<K, V> Singleflight<K, V> {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(0),
            in_flight: Mutex::new(HashMap::new()),
        }
    }
}

impl<K, V> Singleflight<K, V>
where
    K: Eq + Hash + Clone,
{
    fn remove_if_matches(&self, key: &K, id: u64) {
        let mut guard = self
            .in_flight
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.get(key).is_some_and(|entry| entry.id == id) {
            guard.remove(key);
        }
    }
}

impl<K, V> Singleflight<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    pub fn acquire_or_subscribe(&self, key: &K) -> FlightStep<'_, K, V> {
        let mut guard = self
            .in_flight
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(existing) = guard.get(key) {
            return FlightStep::Wait(existing.tx.subscribe());
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, _rx) = watch::channel(None);
        guard.insert(key.clone(), InFlightEntry { id, tx: tx.clone() });
        FlightStep::Lead(InFlightGuard {
            flight: self,
            key: key.clone(),
            id,
            tx,
            completed: false,
        })
    }

    /// Executes `work` as the singleflight leader for `key`, or awaits the in-flight leader's result.
    pub async fn run<F, Fut>(&self, key: K, mut work: F) -> V
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = V>,
    {
        for _ in 0..MAX_SINGLEFLIGHT_RETRIES {
            match self.acquire_or_subscribe(&key) {
                FlightStep::Lead(guard) => {
                    let result = work().await;
                    guard.complete(result.clone());
                    return result;
                }
                FlightStep::Wait(mut rx) => loop {
                    if let Some(val) = rx.borrow_and_update().clone() {
                        return val;
                    }
                    if rx.changed().await.is_err() {
                        break;
                    }
                },
            }
        }
        work().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;

    #[tokio::test]
    async fn test_singleflight_coalesces_concurrent_same_key_and_separates_distinct_keys() {
        let flight = Arc::new(Singleflight::<String, Result<i32, String>>::new());
        let calls_a = Arc::new(AtomicUsize::new(0));
        let calls_b = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..5 {
            let f = Arc::clone(&flight);
            let c = Arc::clone(&calls_a);
            handles.push(tokio::spawn(async move {
                f.run("key-a".to_string(), || {
                    let c = Arc::clone(&c);
                    async move {
                        c.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                        Ok(42)
                    }
                })
                .await
            }));
        }
        {
            let f = Arc::clone(&flight);
            let c = Arc::clone(&calls_b);
            handles.push(tokio::spawn(async move {
                f.run("key-b".to_string(), || {
                    let c = Arc::clone(&c);
                    async move {
                        c.fetch_add(1, Ordering::SeqCst);
                        Ok(99)
                    }
                })
                .await
            }));
        }

        let mut results = Vec::new();
        for h in handles {
            results.push(h.await.unwrap());
        }
        assert_eq!(calls_a.load(Ordering::SeqCst), 1);
        assert_eq!(calls_b.load(Ordering::SeqCst), 1);
        assert_eq!(results[..5], vec![Ok(42); 5]);
        assert_eq!(results[5], Ok(99));
    }
}
