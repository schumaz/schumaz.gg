#[path = "assets/fetcher.rs"]
mod fetcher;
#[path = "assets/singleflight.rs"]
mod singleflight;

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bytes::Bytes;

use self::fetcher::fetch_and_cache_asset;
pub use self::singleflight::AssetSingleflight;
use self::singleflight::{FlightStep, global_singleflight};
use super::paths::{force_touch_mtime_at, should_touch_mtime};
use super::{AppPaths, fnv1a_128_hex};

const ALLOWED_ASSET_PREFIXES: [&str; 2] = [
    "https://ddragon.leagueoflegends.com/",
    "https://raw.communitydragon.org/",
];

const B64_URL_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

const MAX_SINGLEFLIGHT_RETRIES: usize = 3;

pub fn encode_base64_url(input: &[u8]) -> String {
    let mut out = String::with_capacity((input.len() * 4).div_ceil(3));
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i];
        let b1 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let b2 = if i + 2 < input.len() { input[i + 2] } else { 0 };

        out.push(B64_URL_ALPHABET[(b0 >> 2) as usize] as char);
        out.push(B64_URL_ALPHABET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if i + 1 < input.len() {
            out.push(B64_URL_ALPHABET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        }
        if i + 2 < input.len() {
            out.push(B64_URL_ALPHABET[(b2 & 0x3f) as usize] as char);
        }
        i += 3;
    }
    out
}

fn decode_b64_char(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'-' | b'+' => Some(62),
        b'_' | b'/' => Some(63),
        _ => None,
    }
}

pub fn decode_base64_url(input: &str) -> Option<Vec<u8>> {
    let trimmed = input.trim_end_matches('=');
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() || bytes.len() % 4 == 1 {
        return None;
    }

    let mut out = Vec::with_capacity((bytes.len() * 3) / 4);
    let mut i = 0;
    while i < bytes.len() {
        let v0 = decode_b64_char(bytes[i])?;
        let v1 = decode_b64_char(*bytes.get(i + 1)?)?;
        out.push((v0 << 2) | (v1 >> 4));

        if let Some(&c2) = bytes.get(i + 2) {
            let v2 = decode_b64_char(c2)?;
            out.push((v1 << 4) | (v2 >> 2));
            if let Some(&c3) = bytes.get(i + 3) {
                let v3 = decode_b64_char(c3)?;
                out.push((v2 << 6) | v3);
            }
        }
        i += 4;
    }
    Some(out)
}

pub fn is_allowed_asset_url(url: &str) -> bool {
    let trimmed = url.trim();
    ALLOWED_ASSET_PREFIXES
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
}

pub fn extract_asset_extension(url: &str) -> &'static str {
    let without_query = url.split('?').next().unwrap_or(url);
    let without_frag = without_query.split('#').next().unwrap_or(without_query);
    let ext = without_frag
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => "png",
        "jpg" => "jpg",
        "jpeg" => "jpeg",
        "webp" => "webp",
        "svg" => "svg",
        "gif" => "gif",
        _ => "bin",
    }
}

pub fn encode_asset_segment(url: &str) -> String {
    let ext = extract_asset_extension(url);
    let b64 = encode_base64_url(url.as_bytes());
    format!("v1_{b64}.{ext}")
}

pub fn decode_asset_request_segment(raw_path: &str) -> Option<String> {
    let clean = raw_path
        .trim()
        .trim_start_matches('/')
        .split('?')
        .next()?
        .split('#')
        .next()?;

    // Grab the first path segment if any trailing path exists
    let segment = clean.split('/').next()?;
    let rest = segment.strip_prefix("v1_")?;
    let b64_part = match rest.rsplit_once('.') {
        Some((b64, _ext)) => b64,
        None => rest,
    };

    let bytes = decode_base64_url(b64_part)?;
    String::from_utf8(bytes).ok()
}

pub fn asset_cache_path(paths: &AppPaths, url: &str) -> PathBuf {
    let hash = fnv1a_128_hex(url);
    let ext = extract_asset_extension(url);
    paths.assets_dir.join(format!("{hash}.{ext}"))
}

pub fn mime_type_for_ext_or_bytes(ext: &str, bytes: &[u8]) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "gif" => "image/gif",
        _ => {
            if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
                "image/png"
            } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
                "image/jpeg"
            } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
                "image/webp"
            } else if bytes.starts_with(b"GIF8") {
                "image/gif"
            } else if bytes
                .windows(4)
                .take(64)
                .any(|w| w.eq_ignore_ascii_case(b"<svg"))
            {
                "image/svg+xml"
            } else {
                "application/octet-stream"
            }
        }
    }
}

fn read_cached_asset_and_touch(cache_path: &Path, ext: &str) -> Option<(Vec<u8>, &'static str)> {
    let mut file = fs::File::open(cache_path).ok()?;
    let meta = file.metadata().ok();
    let cap = meta.as_ref().map(|m| m.len() as usize).unwrap_or(0);
    let mut bytes = Vec::with_capacity(cap);
    file.read_to_end(&mut bytes).ok()?;
    drop(file);
    if bytes.is_empty() {
        return None;
    }
    if let Some(modified) = meta.and_then(|m| m.modified().ok()) {
        let now = SystemTime::now();
        if should_touch_mtime(modified, now) {
            force_touch_mtime_at(cache_path, now);
        }
    }
    let mime = mime_type_for_ext_or_bytes(ext, &bytes);
    Some((bytes, mime))
}

async fn read_cached_asset_async(
    cache_path: PathBuf,
    ext: &'static str,
) -> Option<(Vec<u8>, &'static str)> {
    tokio::task::spawn_blocking(move || read_cached_asset_and_touch(&cache_path, ext))
        .await
        .ok()
        .flatten()
}

pub async fn serve_cached_or_fetch_asset(
    paths: &AppPaths,
    client: &reqwest::Client,
    url: &str,
) -> Result<(Vec<u8>, &'static str), u16> {
    serve_cached_or_fetch_asset_with_policy(paths, client, url, false).await
}

#[cfg(test)]
async fn serve_cached_or_fetch_asset_with_policy(
    paths: &AppPaths,
    client: &reqwest::Client,
    url: &str,
    allow_unlisted_for_tests: bool,
) -> Result<(Vec<u8>, &'static str), u16> {
    serve_cached_or_fetch_asset_internal(
        global_singleflight(),
        paths,
        client,
        url,
        allow_unlisted_for_tests,
    )
    .await
}

#[cfg(not(test))]
async fn serve_cached_or_fetch_asset_with_policy(
    paths: &AppPaths,
    client: &reqwest::Client,
    url: &str,
    allow_unlisted_for_tests: bool,
) -> Result<(Vec<u8>, &'static str), u16> {
    serve_cached_or_fetch_asset_internal(
        global_singleflight(),
        paths,
        client,
        url,
        allow_unlisted_for_tests,
    )
    .await
}

async fn serve_cached_or_fetch_asset_internal(
    flight: &AssetSingleflight,
    paths: &AppPaths,
    client: &reqwest::Client,
    url: &str,
    allow_unlisted_for_tests: bool,
) -> Result<(Vec<u8>, &'static str), u16> {
    if !allow_unlisted_for_tests && !is_allowed_asset_url(url) {
        return Err(403);
    }

    let ext = extract_asset_extension(url);
    let cache_path = asset_cache_path(paths, url);

    if let Some((bytes, mime)) = read_cached_asset_async(cache_path.clone(), ext).await {
        crate::logging::metrics::record_cache_disk_hit();
        return Ok((bytes, mime));
    }

    let key = (cache_path.clone(), url.to_string());

    for _ in 0..MAX_SINGLEFLIGHT_RETRIES {
        match flight.acquire_or_subscribe(&key) {
            FlightStep::Wait(mut rx) => {
                loop {
                    if let Some(result) = rx.borrow_and_update().clone() {
                        return match result {
                            Ok((bytes, mime)) => {
                                crate::logging::metrics::record_cache_ram_hit();
                                Ok((bytes.to_vec(), mime))
                            }
                            Err(status) => Err(status),
                        };
                    }
                    if rx.changed().await.is_err() {
                        break;
                    }
                }
                if let Some((bytes, mime)) = read_cached_asset_async(cache_path.clone(), ext).await {
                    crate::logging::metrics::record_cache_disk_hit();
                    return Ok((bytes, mime));
                }
            }
            FlightStep::Lead(guard) => {
                if let Some((bytes, mime)) = read_cached_asset_async(cache_path.clone(), ext).await {
                    crate::logging::metrics::record_cache_disk_hit();
                    guard.complete(Ok((Bytes::from(bytes.clone()), mime)));
                    return Ok((bytes, mime));
                }

                let result = fetch_and_cache_asset(client, url, &cache_path, ext).await;
                guard.complete(result.clone());
                return result.map(|(bytes, mime)| (bytes.to_vec(), mime));
            }
        }
    }

    Err(502)
}

#[cfg(test)]
#[path = "assets/assets_tests.rs"]
mod tests;
