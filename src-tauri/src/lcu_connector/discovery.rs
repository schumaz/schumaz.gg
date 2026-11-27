use std::ffi::OsStr;
use std::net::SocketAddrV4;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use http::HeaderValue;
use irelia::rest::LcuClient;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::pem::PemObject;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::lcu_connector::client::{ConnectedLcuClient, DefaultRequestClient};
use crate::logging;
use crate::storage;
use schumaz_domain::lcu_parsers::discovery_manifests as manifests;
use schumaz_domain::lcu_parsers::discovery_parsers as parsers;

#[cfg(target_os = "windows")]
const CLIENT_PROCESS_NAME: &str = "LeagueClientUx.exe";
#[cfg(not(target_os = "windows"))]
const CLIENT_PROCESS_NAME: &str = "LeagueClientUx";

#[cfg(target_os = "windows")]
const LOCKFILE_OWNER_PROCESS_NAME: &str = "LeagueClient.exe";
#[cfg(not(target_os = "windows"))]
const LOCKFILE_OWNER_PROCESS_NAME: &str = "LeagueClient";

const RIOT_GAMES_CA_PEM: &[u8] = include_bytes!("riotgames.pem");

/// Cached connection credentials shared between the LCU REST client and WebSocket listener.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LcuCredentials {
    pub addr: SocketAddrV4,
    pub auth_header: HeaderValue,
}

static SHARED_REQUEST_CLIENT: OnceLock<DefaultRequestClient> = OnceLock::new();
static LCU_TLS_CONFIG: OnceLock<Result<Arc<rustls::ClientConfig>, String>> = OnceLock::new();
static CACHED_CREDENTIALS: OnceLock<RwLock<Option<LcuCredentials>>> = OnceLock::new();
static LAST_LOCKFILE_PATH: OnceLock<RwLock<Option<PathBuf>>> = OnceLock::new();
static PROCESS_SYSTEM: OnceLock<Mutex<System>> = OnceLock::new();
static LAST_DISCOVERY_ERROR: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn credentials_cache() -> &'static RwLock<Option<LcuCredentials>> {
    CACHED_CREDENTIALS.get_or_init(|| RwLock::new(None))
}

fn lockfile_path_cache() -> &'static RwLock<Option<PathBuf>> {
    LAST_LOCKFILE_PATH.get_or_init(|| RwLock::new(None))
}

fn process_system() -> &'static Mutex<System> {
    PROCESS_SYSTEM.get_or_init(|| Mutex::new(System::new()))
}

fn ensure_rustls_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
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
        ensure_rustls_provider();
        irelia::requests::new()
    })
}

/// Returns the memoized `rustls::ClientConfig` trusting Riot Games' local LCU root CA.
pub fn lcu_tls_client_config() -> Result<Arc<rustls::ClientConfig>, String> {
    LCU_TLS_CONFIG
        .get_or_init(|| {
            ensure_rustls_provider();
            let cert = CertificateDer::from_pem_slice(RIOT_GAMES_CA_PEM)
                .map_err(|err| format!("Failed to parse Riot CA PEM: {err}"))?;
            let mut roots = rustls::RootCertStore::empty();
            roots
                .add(cert)
                .map_err(|err| format!("Failed to load Riot CA root certificate: {err}"))?;
            Ok(Arc::new(
                rustls::ClientConfig::builder()
                    .with_root_certificates(roots)
                    .with_no_client_auth(),
            ))
        })
        .clone()
}

/// Returns currently cached LCU credentials without triggering discovery.
pub fn get_cached_lcu_credentials() -> Option<LcuCredentials> {
    credentials_cache().read().ok()?.clone()
}

/// Returns cached LCU credentials or discovers and caches them via lockfile-first resolution.
pub fn get_cached_or_discover_lcu_credentials() -> Result<LcuCredentials, String> {
    if let Some(cached) = get_cached_lcu_credentials() {
        return Ok(cached);
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

    Ok(discovered)
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
    let creds = get_cached_or_discover_lcu_credentials()?;
    Ok(LcuClient::new_with_credentials_with_request_client(
        creds.addr,
        creds.auth_header,
        shared_request_client(),
    ))
}

fn lockfile_path_from_client_exe(exe_path: &Path) -> Option<PathBuf> {
    Some(exe_path.parent()?.join("lockfile"))
}

pub(super) fn is_client_ux_process_name(name: &OsStr) -> bool {
    name.to_str()
        .is_some_and(|s| s.eq_ignore_ascii_case(CLIENT_PROCESS_NAME))
}

pub(super) fn is_valid_lockfile_pid_process(name: &OsStr) -> bool {
    name.to_str().is_some_and(|s| {
        s.eq_ignore_ascii_case(LOCKFILE_OWNER_PROCESS_NAME)
            || s.eq_ignore_ascii_case(CLIENT_PROCESS_NAME)
    })
}

fn load_cached_lockfile_path() -> Option<PathBuf> {
    if let Ok(guard) = lockfile_path_cache().read()
        && let Some(path) = guard.as_ref()
    {
        return Some(path.clone());
    }

    let disk_cache_file = storage::global_paths().map(storage::AppPaths::lcu_lockfile_cache_path)?;
    let persisted = manifests::read_persisted_lockfile_path(&disk_cache_file)?;
    if let Ok(mut guard) = lockfile_path_cache().write() {
        *guard = Some(persisted.clone());
    }
    Some(persisted)
}

fn remember_valid_lockfile_path(path: &Path) {
    let next_key = manifests::normalize_path_key(path);
    let already_cached = lockfile_path_cache()
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|p| manifests::normalize_path_key(p)))
        .is_some_and(|cur_key| cur_key == next_key);

    if !already_cached {
        if let Ok(mut guard) = lockfile_path_cache().write() {
            *guard = Some(path.to_path_buf());
        }
        if let Some(paths) = storage::global_paths() {
            manifests::persist_lockfile_path(&paths.lcu_lockfile_cache_path(), path);
        }
    }
}

pub(super) fn try_discover_from_lockfile_path(
    system: &mut System,
    path: &Path,
) -> Option<LcuCredentials> {
    let (pid_num, port, token) = parsers::read_lockfile_credentials(path)?;
    let pid = Pid::from_u32(pid_num);

    // Verify the PID from lockfile is actually alive using name-only query (zero OpenProcess/PEB).
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing(),
    );
    let proc = system.process(pid)?;
    if !is_valid_lockfile_pid_process(proc.name()) {
        return None;
    }

    let (addr, auth_header) = parsers::build_lcu_credentials(port, &token).ok()?;
    remember_valid_lockfile_path(path);
    Some(LcuCredentials { addr, auth_header })
}

fn discover_lcu_credentials() -> Result<LcuCredentials, String> {
    let mut system = process_system()
        .lock()
        .map_err(|_| "LCU discovery lock poisoned".to_string())?;

    let cached_path = load_cached_lockfile_path();
    if let Some(ref path) = cached_path
        && let Some(creds) = try_discover_from_lockfile_path(&mut system, path)
    {
        return Ok(creds);
    }

    for candidate in manifests::collect_manifest_lockfile_candidates(cached_path.as_deref()) {
        if let Some(creds) = try_discover_from_lockfile_path(&mut system, &candidate) {
            return Ok(creds);
        }
    }

    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());

    let Some(pid) = system
        .processes()
        .iter()
        .find_map(|(pid, proc)| is_client_ux_process_name(proc.name()).then_some(*pid))
    else {
        return Err("LCU process is not running".to_string());
    };

    let exe_kind = ProcessRefreshKind::nothing().with_exe(UpdateKind::Always);
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, exe_kind);

    let lf_path = system
        .process(pid)
        .and_then(|p| p.exe())
        .and_then(lockfile_path_from_client_exe);

    if let Some(ref path) = lf_path
        && let Some(creds) = try_discover_from_lockfile_path(&mut system, path)
    {
        return Ok(creds);
    }

    let cmd_kind = ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always);
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, cmd_kind);

    let process = system
        .process(pid)
        .ok_or_else(|| "LCU process exited during discovery".to_string())?;

    let (port, token) = parsers::extract_credentials_from_cmd(process.cmd())
        .ok_or_else(|| "LCU port/auth token not found in process cmd or lockfile".to_string())?;

    if let Some(ref path) = lf_path {
        remember_valid_lockfile_path(path);
    }

    let (addr, auth_header) = parsers::build_lcu_credentials(port, &token)?;
    Ok(LcuCredentials { addr, auth_header })
}

#[cfg(test)]
#[path = "discovery_tests.rs"]
mod tests;
