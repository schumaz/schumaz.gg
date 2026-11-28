use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::path::Path;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::lz4_cache::{CacheEnvelope, read_lz4_envelope, write_lz4_envelope};
use super::singleflight::Singleflight;

#[derive(Debug, Clone)]
pub struct RamCacheEntry<V> {
    pub cached_at: Instant,
    pub patch: String,
    pub payload: V,
}

/// Generic L1 RAM + L2 `.lz4` disk tiered cache with built-in `Singleflight` request coalescing.
pub struct TieredCache<K, V> {
    ram: RwLock<HashMap<K, RamCacheEntry<V>>>,
    flight: Singleflight<K, Result<V, String>>,
}

impl<K, V> Default for TieredCache<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> TieredCache<K, V> {
    pub fn new() -> Self {
        Self {
            ram: RwLock::new(HashMap::new()),
            flight: Singleflight::new(),
        }
    }
}

impl<K, V> TieredCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// Reads a fresh L1 RAM entry (`ttl == None` means no time-based expiry).
    pub fn get_ram(&self, key: &K, ttl: Option<Duration>) -> Option<RamCacheEntry<V>> {
        self.get_ram_with(key, ttl, |entry| Some(entry.clone()))
    }

    /// Inspects a fresh L1 RAM entry under the read lock without cloning `V` unnecessarily.
    pub fn get_ram_with<F, R>(&self, key: &K, ttl: Option<Duration>, f: F) -> Option<R>
    where
        F: FnOnce(&RamCacheEntry<V>) -> Option<R>,
    {
        let guard = self.ram.read().ok()?;
        let entry = guard.get(key)?;
        if let Some(limit) = ttl
            && entry.cached_at.elapsed() >= limit
        {
            return None;
        }
        f(entry)
    }

    pub fn insert_ram(&self, key: K, patch: impl Into<String>, payload: V) {
        self.insert_ram_at(key, Instant::now(), patch, payload);
    }

    pub fn insert_ram_at(
        &self,
        key: K,
        cached_at: Instant,
        patch: impl Into<String>,
        payload: V,
    ) {
        if let Ok(mut guard) = self.ram.write() {
            guard.insert(
                key,
                RamCacheEntry {
                    cached_at,
                    patch: patch.into(),
                    payload,
                },
            );
        }
    }

    pub fn remove_ram(&self, key: &K) {
        if let Ok(mut guard) = self.ram.write() {
            guard.remove(key);
        }
    }

    pub fn clear_ram(&self) {
        if let Ok(mut guard) = self.ram.write() {
            guard.clear();
        }
    }

    pub async fn coalesce<F, Fut>(&self, key: K, work: F) -> Result<V, String>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<V, String>>,
    {
        self.flight.run(key, work).await
    }
}

impl<K, V> TieredCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone + DeserializeOwned,
{
    pub fn read_disk(&self, path: &Path) -> Option<CacheEnvelope<V>> {
        read_lz4_envelope::<V>(path)
    }
}

impl<K, V> TieredCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone + Serialize,
{
    pub fn write_disk(&self, path: &Path, patch: &str, payload: &V) -> Result<(), String> {
        write_lz4_envelope(path, patch, payload)
    }
}
