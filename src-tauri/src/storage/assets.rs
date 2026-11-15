use std::fs;
use std::path::PathBuf;

use super::{AppPaths, atomic_write, fnv1a_128_hex, touch_mtime};

const ALLOWED_ASSET_PREFIXES: [&str; 3] = [
    "https://ddragon.leagueoflegends.com/",
    "https://raw.communitydragon.org/",
    "https://opgg-static.akamaized.net/",
];

const B64_URL_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

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

pub async fn serve_cached_or_fetch_asset(
    paths: &AppPaths,
    client: &reqwest::Client,
    url: &str,
) -> Result<(Vec<u8>, &'static str), u16> {
    serve_cached_or_fetch_asset_with_policy(paths, client, url, false).await
}

async fn serve_cached_or_fetch_asset_with_policy(
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

    if cache_path.exists()
        && let Ok(bytes) = fs::read(&cache_path)
        && !bytes.is_empty()
    {
        touch_mtime(&cache_path);
        let mime = mime_type_for_ext_or_bytes(ext, &bytes);
        return Ok((bytes, mime));
    }

    let res = client.get(url).send().await.map_err(|_| 502u16)?;
    let status = res.status();
    if !status.is_success() {
        return Err(status.as_u16());
    }

    let bytes = res.bytes().await.map_err(|_| 502u16)?.to_vec();
    if bytes.is_empty() {
        return Err(404);
    }

    let _ = atomic_write(&cache_path, &bytes);
    let mime = mime_type_for_ext_or_bytes(ext, &bytes);
    Ok((bytes, mime))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_asset_segment_roundtrip_and_allowlist() {
        let url = "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_27.jpg?v=1";
        let seg = encode_asset_segment(url);
        assert!(seg.starts_with("v1_"));
        assert!(seg.ends_with(".jpg"));
        assert!(!seg.contains('/'));

        let decoded = decode_asset_request_segment(&format!("/{seg}")).unwrap();
        assert_eq!(decoded, url);

        assert!(is_allowed_asset_url(url));
        assert!(is_allowed_asset_url(
            "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-shared-components/global/default/images/emerald.png"
        ));
        assert!(is_allowed_asset_url(
            "https://opgg-static.akamaized.net/meta/images/lol/latest/perk/8112.png"
        ));
        assert!(!is_allowed_asset_url(
            "https://evil.example.com/payload.png"
        ));
        assert!(!is_allowed_asset_url("file:///C:/Windows/win.ini"));
    }

    #[tokio::test]
    async fn test_serve_cached_or_fetch_asset_caches_on_disk_and_blocks_disallowed() {
        let dir =
            std::env::temp_dir().join(format!("schumazgg_assets_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let paths = AppPaths::from_root(dir.clone());
        paths.ensure_dirs().unwrap();

        let mock_server = MockServer::start().await;
        let fake_png: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3, 4];

        Mock::given(method("GET"))
            .and(path("/cdn/splash/Ahri_0.jpg"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(fake_png.clone()))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        // Public function rejects non-allowlisted hosts
        let forbidden = serve_cached_or_fetch_asset(
            &paths,
            &client,
            &format!("{}/cdn/splash/Ahri_0.jpg", mock_server.uri()),
        )
        .await;
        assert_eq!(forbidden, Err(403));

        // Internal test-policy helper fetches once, caches natively on disk, then hits disk on second call
        let url = format!("{}/cdn/splash/Ahri_0.jpg", mock_server.uri());
        let (b1, m1) = serve_cached_or_fetch_asset_with_policy(&paths, &client, &url, true)
            .await
            .unwrap();
        assert_eq!(b1, fake_png);
        assert_eq!(m1, "image/jpeg");
        assert!(asset_cache_path(&paths, &url).exists());

        let (b2, m2) = serve_cached_or_fetch_asset_with_policy(&paths, &client, &url, true)
            .await
            .unwrap();
        assert_eq!(b2, fake_png);
        assert_eq!(m2, "image/jpeg");

        let _ = fs::remove_dir_all(&dir);
    }
}
