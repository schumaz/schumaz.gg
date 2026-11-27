use std::path::Path;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use bytes::Bytes;
use tokio::sync::Semaphore;

use super::mime_type_for_ext_or_bytes;
use super::singleflight::AssetSharedResult;
use crate::storage::atomic_write;

const MAX_CONCURRENT_ASSET_FETCHES: usize = 8;
const MAX_ASSET_FETCH_ATTEMPTS: usize = 3;
const ASSET_RETRY_DELAYS_MS: [u64; 2] = [150, 350];

static ASSET_FETCH_SEMAPHORE: OnceLock<Semaphore> = OnceLock::new();

fn asset_fetch_semaphore() -> &'static Semaphore {
    ASSET_FETCH_SEMAPHORE.get_or_init(|| Semaphore::new(MAX_CONCURRENT_ASSET_FETCHES))
}

fn is_retryable_asset_status(status: u16) -> bool {
    status == 408 || status == 429 || (500..=599).contains(&status)
}

enum AssetAttemptError {
    HttpStatus(u16, u128),
    Transport(String, u128),
    EmptyBody(u16),
}

async fn try_fetch_asset_once(
    client: &reqwest::Client,
    url: &str,
) -> Result<(Bytes, u16), AssetAttemptError> {
    let Ok(_permit) = asset_fetch_semaphore().acquire().await else {
        return Err(AssetAttemptError::HttpStatus(503, 0));
    };
    let started = Instant::now();
    let res = client.get(url).send().await.map_err(|err| {
        AssetAttemptError::Transport(err.to_string(), started.elapsed().as_millis())
    })?;
    let code = res.status().as_u16();
    if !res.status().is_success() {
        return Err(AssetAttemptError::HttpStatus(
            code,
            started.elapsed().as_millis(),
        ));
    }
    let bytes = res.bytes().await.map_err(|err| {
        AssetAttemptError::Transport(err.to_string(), started.elapsed().as_millis())
    })?;
    if bytes.is_empty() {
        return Err(AssetAttemptError::EmptyBody(code));
    }
    Ok((bytes, code))
}

/// Downloads a remote asset with bounded concurrency and transient-failure retries, writing
/// non-empty payloads atomically into the disk cache.
pub(super) async fn fetch_and_cache_asset(
    client: &reqwest::Client,
    url: &str,
    cache_path: &Path,
    ext: &str,
) -> AssetSharedResult {
    crate::logging::metrics::record_cache_miss();

    for attempt in 0..MAX_ASSET_FETCH_ATTEMPTS {
        match try_fetch_asset_once(client, url).await {
            Ok((bytes, code)) => {
                crate::logging::metrics::record_http_status(code);
                let path_buf = cache_path.to_path_buf();
                let bytes_for_write = bytes.clone();
                let write_res = tokio::task::spawn_blocking(move || {
                    atomic_write(&path_buf, &bytes_for_write)
                })
                .await;
                match write_res {
                    Ok(Ok(())) => {}
                    Ok(Err(err)) => {
                        crate::logging::log_warn(
                            "storage.assets",
                            format!("event=asset_io_error err={err}"),
                        );
                    }
                    Err(err) => {
                        crate::logging::log_warn(
                            "storage.assets",
                            format!("event=asset_io_error err={err}"),
                        );
                    }
                }
                let mime = mime_type_for_ext_or_bytes(ext, &bytes);
                return Ok((bytes, mime));
            }
            Err(AssetAttemptError::EmptyBody(code)) => {
                crate::logging::metrics::record_http_status(code);
                return Err(404);
            }
            Err(AssetAttemptError::HttpStatus(code, elapsed_ms)) => {
                crate::logging::metrics::record_http_status(code);
                if is_retryable_asset_status(code)
                    && let Some(&delay_ms) = ASSET_RETRY_DELAYS_MS.get(attempt)
                {
                    crate::logging::log_debug(
                        "storage.assets",
                        format!(
                            "event=asset_http_retry target=cdn status={code} attempt={} elapsed_ms={elapsed_ms}",
                            attempt + 1
                        ),
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    continue;
                }
                crate::logging::log_warn(
                    "storage.assets",
                    format!(
                        "event=asset_http_error target=cdn status={code} elapsed_ms={elapsed_ms}"
                    ),
                );
                return Err(code);
            }
            Err(AssetAttemptError::Transport(err_msg, elapsed_ms)) => {
                if let Some(&delay_ms) = ASSET_RETRY_DELAYS_MS.get(attempt) {
                    crate::logging::log_debug(
                        "storage.assets",
                        format!(
                            "event=asset_transport_retry target=cdn attempt={} elapsed_ms={elapsed_ms}",
                            attempt + 1
                        ),
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    continue;
                }
                crate::logging::record_http_network_error_log("cdn", elapsed_ms, &err_msg);
                return Err(502);
            }
        }
    }

    Err(502)
}
