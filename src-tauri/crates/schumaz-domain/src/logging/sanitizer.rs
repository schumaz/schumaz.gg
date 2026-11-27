/// Maximum byte length for an individual sanitized log message.
pub const MAX_LOG_MESSAGE_BYTES: usize = 512;

fn starts_with_ignore_ascii_case(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.len() >= needle.len() && haystack[..needle.len()].eq_ignore_ascii_case(needle)
}

fn is_word_boundary_before(bytes: &[u8], idx: usize) -> bool {
    idx == 0 || !bytes[idx - 1].is_ascii_alphanumeric()
}

fn is_token_delimiter(b: u8) -> bool {
    b.is_ascii_whitespace()
        || matches!(
            b,
            b'"' | b'\'' | b',' | b';' | b')' | b']' | b'}' | b'<' | b'>'
        )
}

fn is_header_delimiter(b: u8) -> bool {
    matches!(b, b'\r' | b'\n' | b'"' | b'\'' | b',' | b';' | b'}' | b']')
}

fn is_base64_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'=' | b'_' | b'-')
}

fn match_authorization_header(bytes: &[u8], idx: usize) -> Option<usize> {
    if !is_word_boundary_before(bytes, idx) {
        return None;
    }
    let prefix = b"authorization:";
    if !starts_with_ignore_ascii_case(&bytes[idx..], prefix) {
        return None;
    }
    let mut end = idx + prefix.len();
    while end < bytes.len() && !is_header_delimiter(bytes[end]) {
        end += 1;
    }
    Some(end)
}

fn match_basic_auth(bytes: &[u8], idx: usize) -> Option<usize> {
    if !is_word_boundary_before(bytes, idx) {
        return None;
    }
    let prefix = b"basic ";
    if !starts_with_ignore_ascii_case(&bytes[idx..], prefix) {
        return None;
    }
    let token_start = idx + prefix.len();
    let mut end = token_start;
    while end < bytes.len() && is_base64_char(bytes[end]) {
        end += 1;
    }
    if end.saturating_sub(token_start) < 4 {
        return None;
    }
    Some(end)
}

fn match_cli_flag_value(bytes: &[u8], idx: usize, flag: &[u8]) -> Option<usize> {
    if !starts_with_ignore_ascii_case(&bytes[idx..], flag) {
        return None;
    }
    let mut end = idx + flag.len();
    while end < bytes.len() && !is_token_delimiter(bytes[end]) {
        end += 1;
    }
    Some(end)
}

fn match_riot_user_token(bytes: &[u8], idx: usize) -> Option<usize> {
    if !is_word_boundary_before(bytes, idx) {
        return None;
    }
    let prefix = b"riot:";
    if !starts_with_ignore_ascii_case(&bytes[idx..], prefix) {
        return None;
    }
    let token_start = idx + prefix.len();
    let mut end = token_start;
    while end < bytes.len() && !is_token_delimiter(bytes[end]) && bytes[end] != b'@' {
        end += 1;
    }
    if end == token_start {
        return None;
    }
    Some(end)
}

fn match_local_endpoint(bytes: &[u8], idx: usize) -> Option<(&'static str, usize)> {
    if !is_word_boundary_before(bytes, idx) {
        return None;
    }
    let (label, prefix): (&'static str, &[u8]) =
        if starts_with_ignore_ascii_case(&bytes[idx..], b"127.0.0.1:") {
            ("127.0.0.1:<redacted>", b"127.0.0.1:")
        } else if starts_with_ignore_ascii_case(&bytes[idx..], b"localhost:") {
            ("localhost:<redacted>", b"localhost:")
        } else {
            return None;
        };

    let port_start = idx + prefix.len();
    let mut end = port_start;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    let port_len = end.saturating_sub(port_start);
    if (1..=5).contains(&port_len) {
        Some((label, end))
    } else {
        None
    }
}

fn match_windows_user_path(bytes: &[u8], idx: usize) -> Option<(String, usize)> {
    if idx + 9 >= bytes.len() {
        return None;
    }
    if !bytes[idx].is_ascii_alphabetic() || bytes[idx + 1] != b':' {
        return None;
    }
    let sep1 = bytes[idx + 2];
    if sep1 != b'\\' && sep1 != b'/' {
        return None;
    }
    if !starts_with_ignore_ascii_case(&bytes[idx + 3..], b"users") {
        return None;
    }
    let sep2 = bytes[idx + 8];
    if sep2 != b'\\' && sep2 != b'/' {
        return None;
    }
    let user_start = idx + 9;
    let mut end = user_start;
    while end < bytes.len()
        && bytes[end] != b'\\'
        && bytes[end] != b'/'
        && !is_token_delimiter(bytes[end])
    {
        end += 1;
    }
    if end == user_start {
        return None;
    }
    let drive = bytes[idx] as char;
    let sep1_ch = sep1 as char;
    let sep2_ch = sep2 as char;
    Some((format!("{drive}:{sep1_ch}Users{sep2_ch}<redacted>"), end))
}

fn is_lockfile_proc_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.')
}

fn match_lockfile_tuple(bytes: &[u8], idx: usize) -> Option<usize> {
    if !is_word_boundary_before(bytes, idx) {
        return None;
    }
    let mut p = idx;
    if starts_with_ignore_ascii_case(&bytes[p..], b"Riot Client:") {
        p += b"Riot Client".len();
    } else {
        while p < bytes.len() && p - idx < 40 && is_lockfile_proc_char(bytes[p]) {
            p += 1;
        }
    }
    if p == idx || p >= bytes.len() || bytes[p] != b':' {
        return None;
    }
    p += 1;

    let pid_start = p;
    while p < bytes.len() && bytes[p].is_ascii_digit() {
        p += 1;
    }
    if !(1..=10).contains(&(p - pid_start)) || p >= bytes.len() || bytes[p] != b':' {
        return None;
    }
    p += 1;

    let port_start = p;
    while p < bytes.len() && bytes[p].is_ascii_digit() {
        p += 1;
    }
    if !(1..=5).contains(&(p - port_start)) || p >= bytes.len() || bytes[p] != b':' {
        return None;
    }
    p += 1;

    let tok_start = p;
    while p < bytes.len()
        && p - tok_start < 128
        && bytes[p] != b':'
        && !is_token_delimiter(bytes[p])
    {
        p += 1;
    }
    if p == tok_start || p >= bytes.len() || bytes[p] != b':' {
        return None;
    }
    p += 1;

    let proto_len = if starts_with_ignore_ascii_case(&bytes[p..], b"https") {
        5
    } else if starts_with_ignore_ascii_case(&bytes[p..], b"http") {
        4
    } else {
        return None;
    };
    let end = p + proto_len;
    if end < bytes.len() && bytes[end].is_ascii_alphanumeric() {
        return None;
    }
    Some(end)
}

fn redact_sensitive_patterns(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < bytes.len() {
        if let Some(end) = match_authorization_header(bytes, i) {
            out.push_str("Authorization: <redacted>");
            i = end;
            continue;
        }
        if let Some(end) = match_basic_auth(bytes, i) {
            out.push_str("Basic <redacted>");
            i = end;
            continue;
        }
        if let Some(end) = match_cli_flag_value(bytes, i, b"--remoting-auth-token=") {
            out.push_str("--remoting-auth-token=<redacted>");
            i = end;
            continue;
        }
        if let Some(end) = match_cli_flag_value(bytes, i, b"--app-port=") {
            out.push_str("--app-port=<redacted>");
            i = end;
            continue;
        }
        if let Some(end) = match_riot_user_token(bytes, i) {
            out.push_str("riot:<redacted>");
            i = end;
            continue;
        }
        if let Some(end) = match_lockfile_tuple(bytes, i) {
            out.push_str("<redacted_lockfile>");
            i = end;
            continue;
        }
        if let Some((replacement, end)) = match_local_endpoint(bytes, i) {
            out.push_str(replacement);
            i = end;
            continue;
        }
        if let Some((replacement, end)) = match_windows_user_path(bytes, i) {
            out.push_str(&replacement);
            i = end;
            continue;
        }

        if let Some(ch) = input[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            break;
        }
    }

    out
}

fn escape_control_chars(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push(' '),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

fn truncate_utf8_bytes(input: String, max_bytes: usize) -> String {
    if input.len() <= max_bytes {
        return input;
    }
    const SUFFIX: &str = "...";
    if max_bytes <= SUFFIX.len() {
        return SUFFIX[..max_bytes].to_string();
    }
    let mut cut = max_bytes - SUFFIX.len();
    while cut > 0 && !input.is_char_boundary(cut) {
        cut -= 1;
    }
    let mut truncated = String::with_capacity(max_bytes);
    truncated.push_str(&input[..cut]);
    truncated.push_str(SUFFIX);
    truncated
}

/// Applies mandatory linear redaction, anti-log-injection escaping, and byte truncation.
pub fn sanitize_log_message(input: &str) -> String {
    let redacted = redact_sensitive_patterns(input);
    let escaped = escape_control_chars(&redacted);
    truncate_utf8_bytes(escaped, MAX_LOG_MESSAGE_BYTES)
}

#[cfg(test)]
#[path = "sanitizer_tests.rs"]
mod tests;
