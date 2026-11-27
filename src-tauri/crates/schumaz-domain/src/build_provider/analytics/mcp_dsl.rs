mod ast;

pub use ast::{McpDocument, McpTuple, McpValue};

use std::borrow::Cow;
use std::collections::HashMap;

impl<'a> McpDocument<'a> {
    /// Tokenizes and parses an OP.GG MCP DSL payload safely across arbitrary UTF-8 boundaries.
    pub fn parse(text: &'a str) -> Self {
        let mut parser = Parser { src: text, pos: 0 };
        parser.parse_document()
    }
}

struct Parser<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn rest(&self) -> &'a str {
        self.src.get(self.pos..).unwrap_or("")
    }

    fn peek_char(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn advance_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn skip_ws(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
    }

    fn parse_document(&mut self) -> McpDocument<'a> {
        let mut doc = McpDocument::default();
        let mut prev_char: Option<char> = None;

        while let Some(ch) = self.peek_char() {
            if ch == '"' || ch == '\'' {
                let _ = self.parse_quoted_str(ch);
                prev_char = Some(ch);
                continue;
            }

            let at_line_start = prev_char.is_none() || prev_char == Some('\n');
            if at_line_start && self.try_parse_class_header(&mut doc.schemas) {
                prev_char = self.src[..self.pos].chars().next_back();
                continue;
            }

            let boundary_ok = prev_char.is_none_or(|p| !p.is_alphanumeric() && p != '_');
            if boundary_ok && (ch.is_ascii_alphabetic() || ch == '_') {
                let ident_start = self.pos;
                let ident = self.consume_ident();
                if self.peek_char() == Some('(') {
                    let after_open = self.pos + 1;
                    if let Some(tuple) = self.parse_tuple_body(ident) {
                        doc.roots.push(tuple);
                        prev_char = Some(')');
                        continue;
                    }
                    // Recover cleanly past `(` on malformed/unclosed tuples without panicking.
                    self.pos = after_open.min(self.src.len());
                    prev_char = Some('(');
                    continue;
                }
                prev_char = self.src[ident_start..self.pos].chars().next_back();
                continue;
            }

            self.pos += ch.len_utf8();
            prev_char = Some(ch);
        }

        doc
    }

    fn try_parse_class_header(&mut self, schemas: &mut HashMap<&'a str, Vec<&'a str>>) -> bool {
        let saved = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch == ' ' || ch == '\t' {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
        let Some(after_kw) = self.rest().strip_prefix("class ") else {
            self.pos = saved;
            return false;
        };
        let line_len = after_kw.find('\n').unwrap_or(after_kw.len());
        let line = after_kw[..line_len].trim_end_matches('\r');
        let Some(colon_idx) = line.find(':') else {
            self.pos = saved;
            return false;
        };
        let class_name = line[..colon_idx].trim();
        if class_name.is_empty()
            || !class_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.pos = saved;
            return false;
        }
        let fields: Vec<&'a str> = line[colon_idx + 1..]
            .split(',')
            .map(str::trim)
            .filter(|f| !f.is_empty())
            .collect();
        schemas.insert(class_name, fields);
        let consumed_in_line = "class ".len() + line_len;
        self.pos += consumed_in_line;
        if self.peek_char() == Some('\n') {
            self.pos += 1;
        }
        true
    }

    fn consume_ident(&mut self) -> &'a str {
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
        &self.src[start..self.pos]
    }

    fn parse_tuple_body(&mut self, name: &'a str) -> Option<McpTuple<'a>> {
        if self.advance_char() != Some('(') {
            return None;
        }
        let args = self.parse_delimited_items(')')?;
        Some(McpTuple { name, args })
    }

    fn parse_delimited_items(&mut self, close: char) -> Option<Vec<McpValue<'a>>> {
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek_char() == Some(close) {
            self.pos += close.len_utf8();
            return Some(items);
        }

        loop {
            self.skip_ws();
            let val = self.parse_value()?;
            items.push(val);
            self.skip_ws();
            match self.peek_char() {
                Some(',') => {
                    self.pos += 1;
                    self.skip_ws();
                    if self.peek_char() == Some(close) {
                        self.pos += close.len_utf8();
                        return Some(items);
                    }
                }
                Some(ch) if ch == close => {
                    self.pos += close.len_utf8();
                    return Some(items);
                }
                _ => return None,
            }
        }
    }

    fn parse_value(&mut self) -> Option<McpValue<'a>> {
        self.skip_ws();
        let ch = self.peek_char()?;
        match ch {
            '"' | '\'' => self.parse_quoted_str(ch).map(McpValue::Str),
            '[' => {
                self.pos += 1;
                self.parse_delimited_items(']').map(McpValue::List)
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let ident = self.consume_ident();
                if self.peek_char() == Some('(') {
                    self.parse_tuple_body(ident).map(McpValue::Tuple)
                } else {
                    Some(match ident {
                        "true" | "True" => McpValue::Bool(true),
                        "false" | "False" => McpValue::Bool(false),
                        "null" | "None" => McpValue::Null,
                        other => McpValue::Raw(other),
                    })
                }
            }
            c if c.is_ascii_digit() || c == '-' || c == '+' => Some(self.parse_number_or_raw()),
            _ => None,
        }
    }

    fn parse_number_or_raw(&mut self) -> McpValue<'a> {
        let start = self.pos;
        if matches!(self.peek_char(), Some('-' | '+')) {
            self.pos += 1;
        }
        let mut prev_exp = false;
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() || ch == '.' {
                self.pos += 1;
                prev_exp = false;
            } else if ch == 'e' || ch == 'E' {
                self.pos += 1;
                prev_exp = true;
            } else if prev_exp && (ch == '-' || ch == '+') {
                self.pos += 1;
                prev_exp = false;
            } else {
                break;
            }
        }
        let raw = &self.src[start..self.pos];
        if raw.contains(['.', 'e', 'E']) {
            if let Ok(f) = raw.parse::<f64>() {
                return McpValue::Float(f);
            }
        } else if let Ok(i) = raw.parse::<i64>() {
            return McpValue::Int(i);
        } else if let Ok(f) = raw.parse::<f64>() {
            return McpValue::Float(f);
        }
        McpValue::Raw(raw)
    }

    fn parse_quoted_str(&mut self, quote: char) -> Option<Cow<'a, str>> {
        let start = self.pos;
        if self.advance_char() != Some(quote) {
            return None;
        }
        let content_start = self.pos;
        let mut has_escape = false;
        while let Some(ch) = self.peek_char() {
            if ch == '\\' {
                has_escape = true;
                self.pos += 1;
                self.advance_char()?;
                continue;
            }
            if ch == quote {
                let content_end = self.pos;
                self.pos += quote.len_utf8();
                if !has_escape {
                    return Some(Cow::Borrowed(&self.src[content_start..content_end]));
                }
                if quote == '"'
                    && let Ok(decoded) = serde_json::from_str::<String>(&self.src[start..self.pos])
                {
                    return Some(Cow::Owned(decoded));
                }
                return Some(Cow::Owned(unescape_basic(
                    &self.src[content_start..content_end],
                )));
            }
            self.pos += ch.len_utf8();
        }
        None
    }
}

fn unescape_basic(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some(other) => out.push(other),
                None => break,
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
#[path = "mcp_dsl_tests.rs"]
mod tests;
