use std::ffi::OsStr;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, RwLock};

use http::HeaderValue;
use irelia::rest::LcuClient;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::lcu_connector::client::{ConnectedLcuClient, DefaultRequestClient};
use crate::logging;

#[cfg(target_os = "windows")]
const CLIENT_PROCESS_NAME: &str = "LeagueClientUx.exe";
#[cfg(not(target_os = "windows"))]
const CLIENT_PROCESS_NAME: &str = "LeagueClientUx";

#[cfg(target_os = "windows")]
const GAME_PROCESS_NAME: &str = "League of Legends.exe";
#[cfg(not(target_os = "windows"))]
const GAME_PROCESS_NAME: &str = "League of Legends";

const BASE64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[derive(Clone, Debug)]
struct CachedLcuCredentials {
    addr: SocketAddrV4,
    auth_header: HeaderValue,
}

static SHARED_REQUEST_CLIENT: OnceLock<DefaultRequestClient> = OnceLock::new();
static CACHED_CREDENTIALS: OnceLock<RwLock<Option<CachedLcuCredentials>>> = OnceLock::new();
static LAST_LOCKFILE_PATH: OnceLock<RwLock<Option<PathBuf>>> = OnceLock::new();
static PROCESS_SYSTEM: OnceLock<Mutex<System>> = OnceLock::new();
static LAST_DISCOVERY_ERROR: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn credentials_cache() -> &'static RwLock<Option<CachedLcuCredentials>> {
    CACHED_CREDENTIALS.get_or_init(|| RwLock::new(None))
}

fn lockfile_path_cache() -> &'static RwLock<Option<PathBuf>> {
    LAST_LOCKFILE_PATH.get_or_init(|| RwLock::new(None))
}

fn process_system() -> &'static Mutex<System> {
    PROCESS_SYSTEM.get_or_init(|| Mutex::new(System::new()))
}

fn set_discovery_error_state(reason: Option<&str>) {
    let lock = LAST_DISCOVERY_ERROR.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = lock.lock()
        && guard.as_deref() != reason
    {
        *guard = reason.map(str::to_string);
        if let Some(msg) = reason {
            logging::log_debug(
                "lcu.discovery",
                format!("event=discovery_failed reason={msg}"),
            );
        }
    }
}

/// Returns the process-wide HTTP/TLS client shared across all LCU REST requests.
pub fn shared_request_client() -> &'static DefaultRequestClient {
    SHARED_REQUEST_CLIENT.get_or_init(|| {
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
        irelia::requests::new()
    })
}

/// Clears cached LCU port and auth header so the next connection re-discovers credentials.
pub fn invalidate_lcu_credentials() {
    if let Ok(mut guard) = credentials_cache().write()
        && guard.take().is_some()
    {
        logging::log_info("lcu.discovery", "event=credentials_invalidated");
    }
}

/// Invalidates cached credentials only when an irelia request error indicates transport failure
/// or stale authorization (never on normal 404/4xx responses such as missing champ select).
pub(super) fn maybe_invalidate_on_error(err: &irelia::requests::Error) {
    match err {
        irelia::requests::Error::ClientError(_) | irelia::requests::Error::Error(_) => {
            invalidate_lcu_credentials();
        }
        irelia::requests::Error::Request(status)
            if status.as_u16() == 401 || status.as_u16() == 403 =>
        {
            invalidate_lcu_credentials();
        }
        _ => {}
    }
}

pub(super) fn connect_lcu_client() -> Result<ConnectedLcuClient, String> {
    if let Ok(guard) = credentials_cache().read()
        && let Some(cached) = guard.as_ref()
    {
        return Ok(LcuClient::new_with_credentials_with_request_client(
            cached.addr,
            cached.auth_header.clone(),
            shared_request_client(),
        ));
    }

    let discovered = match discover_lcu_credentials() {
        Ok(creds) => {
            set_discovery_error_state(None);
            creds
        }
        Err(err) => {
            set_discovery_error_state(Some(&err));
            return Err(err);
        }
    };
    if let Ok(mut guard) = credentials_cache().write() {
        *guard = Some(discovered.clone());
    }

    Ok(LcuClient::new_with_credentials_with_request_client(
        discovered.addr,
        discovered.auth_header,
        shared_request_client(),
    ))
}

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

pub(crate) fn build_lcu_credentials(
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

pub(crate) fn parse_lockfile_contents(contents: &str) -> Option<(u32, u16, String)> {
    let mut parts = contents.trim().split(':');
    let _proc_name = parts.next()?;
    let pid = parts.next()?.trim().parse::<u32>().ok()?;
    let port = parts.next()?.trim().parse::<u16>().ok()?;
    let token = parts.next()?.trim();
    if token.is_empty() {
        return None;
    }
    Some((pid, port, token.to_string()))
}

pub(crate) fn extract_credentials_from_cmd(cmd: &[std::ffi::OsString]) -> Option<(u16, String)> {
    let mut port: Option<u16> = None;
    let mut token: Option<String> = None;

    for arg in cmd {
        let Some(s) = arg.to_str() else {
            continue;
        };
        if port.is_none()
            && let Some(raw_port) = s.strip_prefix("--app-port=")
        {
            port = raw_port.trim().parse::<u16>().ok();
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

fn lockfile_path_from_exe(exe_path: &Path, is_client: bool) -> Option<PathBuf> {
    let mut dir = exe_path.parent()?;
    if !is_client {
        dir = dir.parent()?;
    }
    Some(dir.join("lockfile"))
}

fn read_lockfile_credentials(path: &Path) -> Option<(u32, u16, String)> {
    let contents = std::fs::read_to_string(path).ok()?;
    parse_lockfile_contents(&contents)
}

fn is_league_process_name(name: &OsStr) -> Option<bool> {
    if name == OsStr::new(CLIENT_PROCESS_NAME) {
        Some(true)
    } else if name == OsStr::new(GAME_PROCESS_NAME) {
        Some(false)
    } else {
        None
    }
}

fn try_discover_from_cached_lockfile(system: &mut System) -> Option<CachedLcuCredentials> {
    let path = lockfile_path_cache().read().ok()?.clone()?;
    let (pid_num, port, token) = read_lockfile_credentials(&path)?;
    let pid = Pid::from_u32(pid_num);

    // Verify the PID from lockfile is actually alive using name-only query (zero OpenProcess/PEB).
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        false,
        ProcessRefreshKind::nothing(),
    );
    let proc = system.process(pid)?;
    is_league_process_name(proc.name())?;

    let (addr, auth_header) = build_lcu_credentials(port, &token).ok()?;
    Some(CachedLcuCredentials { addr, auth_header })
}

fn discover_lcu_credentials() -> Result<CachedLcuCredentials, String> {
    let mut system = process_system()
        .lock()
        .map_err(|_| "LCU discovery lock poisoned".to_string())?;

    if let Some(creds) = try_discover_from_cached_lockfile(&mut system) {
        return Ok(creds);
    }

    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());

    let matched = system.processes().iter().find_map(|(pid, proc)| {
        is_league_process_name(proc.name()).map(|is_client| (*pid, is_client))
    });

    let Some((pid, is_client)) = matched else {
        return Err("LCU process is not running".to_string());
    };

    let detail_kind = ProcessRefreshKind::nothing()
        .with_exe(UpdateKind::Always)
        .with_cmd(UpdateKind::Always);
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), false, detail_kind);

    let process = system
        .process(pid)
        .ok_or_else(|| "LCU process exited during discovery".to_string())?;

    if let Some(exe) = process.exe()
        && let Some(lf_path) = lockfile_path_from_exe(exe, is_client)
        && let Ok(mut guard) = lockfile_path_cache().write()
    {
        *guard = Some(lf_path);
    }

    let (port, token) = if is_client {
        extract_credentials_from_cmd(process.cmd())
            .or_else(|| {
                let exe = process.exe()?;
                let lf_path = lockfile_path_from_exe(exe, true)?;
                let (_, port, token) = read_lockfile_credentials(&lf_path)?;
                Some((port, token))
            })
            .ok_or_else(|| "LCU port/auth token not found in process cmd or lockfile".to_string())?
    } else {
        let exe = process
            .exe()
            .ok_or_else(|| "LCU game executable path not found".to_string())?;
        let lf_path = lockfile_path_from_exe(exe, false)
            .ok_or_else(|| "LCU lockfile path could not be resolved".to_string())?;
        let (_, port, token) = read_lockfile_credentials(&lf_path)
            .ok_or_else(|| "LCU lockfile could not be read".to_string())?;
        (port, token)
    };

    let (addr, auth_header) = build_lcu_credentials(port, &token)?;
    Ok(CachedLcuCredentials { addr, auth_header })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn test_parse_lockfile_contents_valid_and_invalid() {
        assert_eq!(
            parse_lockfile_contents("LeagueClient:12345:54321:superSecretToken:https"),
            Some((12345, 54321, "superSecretToken".to_string()))
        );
        assert_eq!(
            parse_lockfile_contents("LeagueClient:bad:54321:tok:https"),
            None
        );
        assert_eq!(parse_lockfile_contents("LeagueClient:1:54321::https"), None);
    }

    #[test]
    fn test_extract_credentials_from_cmd() {
        let args = vec![
            OsString::from("LeagueClientUx.exe"),
            OsString::from("--remoting-auth-token=abc123token"),
            OsString::from("--app-port=61234"),
        ];
        assert_eq!(
            extract_credentials_from_cmd(&args),
            Some((61234, "abc123token".to_string()))
        );
    }

    #[test]
    fn test_build_lcu_credentials_encodes_basic_auth() {
        let (addr, header) = build_lcu_credentials(54321, "token").unwrap();
        assert_eq!(addr, SocketAddrV4::new(Ipv4Addr::LOCALHOST, 54321));
        assert_eq!(header.to_str().unwrap(), "Basic cmlvdDp0b2tlbg==");
    }
}
