use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use bytes::Bytes;
use tokio::sync::watch;

pub(super) type AssetSharedResult = Result<(Bytes, &'static str), u16>;
pub(super) type SingleflightKey = (PathBuf, String);

struct InFlightEntry {
    id: u64,
    tx: watch::Sender<Option<AssetSharedResult>>,
}

/// Coalesces concurrent cold-cache asset downloads for the same destination and URL.
#[derive(Default)]
pub struct AssetSingleflight {
    next_id: AtomicU64,
    in_flight: Mutex<HashMap<SingleflightKey, InFlightEntry>>,
}

pub(super) enum FlightStep<'a> {
    Lead(InFlightGuard<'a>),
    Wait(watch::Receiver<Option<AssetSharedResult>>),
}

pub(super) struct InFlightGuard<'a> {
    flight: &'a AssetSingleflight,
    key: SingleflightKey,
    id: u64,
    tx: watch::Sender<Option<AssetSharedResult>>,
    completed: bool,
}

impl Drop for InFlightGuard<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.flight.remove_if_matches(&self.key, self.id);
        }
    }
}

impl InFlightGuard<'_> {
    pub(super) fn complete(mut self, result: AssetSharedResult) {
        self.flight.remove_if_matches(&self.key, self.id);
        let _ = self.tx.send(Some(result));
        self.completed = true;
    }
}

impl AssetSingleflight {
    pub(super) fn acquire_or_subscribe(&self, key: &SingleflightKey) -> FlightStep<'_> {
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

    fn remove_if_matches(&self, key: &SingleflightKey, id: u64) {
        let mut guard = self
            .in_flight
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.get(key).is_some_and(|entry| entry.id == id) {
            guard.remove(key);
        }
    }
}

static ASSET_SINGLEFLIGHT: OnceLock<AssetSingleflight> = OnceLock::new();

pub(super) fn global_singleflight() -> &'static AssetSingleflight {
    ASSET_SINGLEFLIGHT.get_or_init(AssetSingleflight::default)
}
