use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::Path;

use http::HeaderValue;

const BASE64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn encode_base64(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(BASE64_TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(BASE64_TABLE[((triple >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            BASE64_TABLE[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            BASE64_TABLE[(triple & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

pub fn build_lcu_credentials(
    port: u16,
    auth_token: &str,
) -> Result<(SocketAddrV4, HeaderValue), String> {
    let raw_auth = format!("riot:{}", auth_token.trim());
    let encoded = format!("Basic {}", encode_base64(raw_auth.as_bytes()));
    let auth_header = HeaderValue::from_str(&encoded)
        .map_err(|err| format!("Invalid LCU auth header: {}", err))?;
    let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
    Ok((addr, auth_header))
}

pub fn parse_lockfile_contents(contents: &str) -> Option<(u32, u16, String)> {
    let mut parts = contents.trim().split(':');
    let proc_name = parts.next()?.trim();
    if !proc_name.eq_ignore_ascii_case("LeagueClient")
        && !proc_name.eq_ignore_ascii_case("LeagueClientUx")
    {
        return None;
    }
    let pid = parts.next()?.trim().parse::<u32>().ok().filter(|&p| p > 0)?;
    let port = parts.next()?.trim().parse::<u16>().ok().filter(|&p| p > 0)?;
    let token = parts.next()?.trim();
    if token.is_empty() {
        return None;
    }
    Some((pid, port, token.to_string()))
}

pub fn extract_credentials_from_cmd(cmd: &[std::ffi::OsString]) -> Option<(u16, String)> {
    let mut port: Option<u16> = None;
    let mut token: Option<String> = None;

    for arg in cmd {
        let Some(s) = arg.to_str() else {
            continue;
        };
        if port.is_none()
            && let Some(raw_port) = s.strip_prefix("--app-port=")
        {
            port = raw_port.trim().parse::<u16>().ok().filter(|&p| p > 0);
        }
        if token.is_none()
            && let Some(raw_token) = s.strip_prefix("--remoting-auth-token=")
        {
            let trimmed = raw_token.trim();
            if !trimmed.is_empty() {
                token = Some(trimmed.to_string());
            }
        }
        if port.is_some() && token.is_some() {
            break;
        }
    }

    Some((port?, token?))
}

pub fn read_lockfile_credentials(path: &Path) -> Option<(u32, u16, String)> {
    let contents = std::fs::read_to_string(path).ok()?;
    parse_lockfile_contents(&contents)
}
