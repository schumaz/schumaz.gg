use super::*;

#[test]
fn test_redacts_basic_auth_and_authorization_headers() {
    let input = "req failed Authorization: Basic cmlvdDpzZWNyZXRUb2tlbg==, retry=1";
    let clean = sanitize_log_message(input);
    assert!(!clean.contains("cmlvdDpzZWNyZXRUb2tlbg=="));
    assert!(clean.contains("Authorization: <redacted>"));
    assert!(clean.contains("retry=1"));

    let bare_basic = sanitize_log_message("header=Basic cmlvdDphYmMxMjM0NTY=");
    assert_eq!(bare_basic, "header=Basic <redacted>");

    let bearer = sanitize_log_message("authorization: Bearer eyJhbGciOiJIUzI1NiJ9.secret\nnext=ok");
    assert!(!bearer.contains("eyJhbGci"));
    assert!(bearer.contains("Authorization: <redacted>"));
    assert!(bearer.contains("\\nnext=ok"));
}

#[test]
fn test_redacts_lcu_cli_flags_and_riot_user_token() {
    let cmd = "cmd --app-port=54321 --remoting-auth-token=SuperSecret_999 riot:AnotherSecret@host";
    let clean = sanitize_log_message(cmd);
    assert!(!clean.contains("54321"));
    assert!(!clean.contains("SuperSecret_999"));
    assert!(!clean.contains("AnotherSecret"));
    assert!(clean.contains("--app-port=<redacted>"));
    assert!(clean.contains("--remoting-auth-token=<redacted>"));
    assert!(clean.contains("riot:<redacted>@host"));
}

#[test]
fn test_redacts_lockfile_tuples_and_local_endpoints() {
    let raw = "lockfile=LeagueClient:12345:61234:mySecretToken_abc:https addr=https://127.0.0.1:61234/lol alt=http://localhost:2999/live";
    let clean = sanitize_log_message(raw);
    assert!(!clean.contains("mySecretToken_abc"));
    assert!(!clean.contains("61234"));
    assert!(!clean.contains("2999"));
    assert!(clean.contains("lockfile=<redacted_lockfile>"));
    assert!(clean.contains("127.0.0.1:<redacted>/lol"));
    assert!(clean.contains("localhost:<redacted>/live"));

    let riot_client_lf = sanitize_log_message("Riot Client:999:55111:tok_XYZ:https");
    assert_eq!(riot_client_lf, "<redacted_lockfile>");
}

#[test]
fn test_redacts_windows_user_paths_preserving_structure() {
    let raw = r"io error at C:\Users\Alice\AppData\Local\schumazgg\Logs\schumazgg.log and D:/users/Bob/file.txt";
    let clean = sanitize_log_message(raw);
    assert!(!clean.contains("Alice"));
    assert!(!clean.contains("Bob"));
    assert!(clean.contains(r"C:\Users\<redacted>\AppData\Local\schumazgg\Logs\schumazgg.log"));
    assert!(clean.contains("D:/Users/<redacted>/file.txt"));
}

#[test]
fn test_escapes_crlf_log_injection_and_truncates_to_512_bytes() {
    let injected = "event=ok\r\n2026-01-01T00:00:00.000Z ERROR [fake] spoofed=true\x07";
    let clean = sanitize_log_message(injected);
    assert!(!clean.contains('\r'));
    assert!(!clean.contains('\n'));
    assert!(!clean.contains('\x07'));
    assert!(clean.contains("\\r\\n2026-01-01T00:00:00.000Z"));

    // Multi-byte UTF-8 truncation must stay within 512 bytes on a valid char boundary.
    let long_utf8 = "é".repeat(400);
    let truncated = sanitize_log_message(&long_utf8);
    assert!(truncated.len() <= MAX_LOG_MESSAGE_BYTES);
    assert!(truncated.ends_with("..."));
    assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
}
