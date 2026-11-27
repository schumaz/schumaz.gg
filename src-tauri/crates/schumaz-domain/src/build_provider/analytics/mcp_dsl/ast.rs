use std::borrow::Cow;
use std::collections::HashMap;

/// A named tuple node in OP.GG MCP's Python-like DSL (e.g. `CoreItems([3118,4645,3157],100,52,0.1)`).
#[derive(Debug, Clone, PartialEq)]
pub struct McpTuple<'a> {
    pub name: &'a str,
    pub args: Vec<McpValue<'a>>,
}

/// Typed value parsed from OP.GG MCP's DSL output without unsafe byte indexing.
#[derive(Debug, Clone, PartialEq)]
pub enum McpValue<'a> {
    Tuple(McpTuple<'a>),
    List(Vec<McpValue<'a>>),
    Str(Cow<'a, str>),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    Raw(&'a str),
}

/// Parsed OP.GG MCP response document containing `class <Name>: <fields>` schemas and tuple AST roots.
#[derive(Debug, Clone, Default)]
pub struct McpDocument<'a> {
    pub schemas: HashMap<&'a str, Vec<&'a str>>,
    pub roots: Vec<McpTuple<'a>>,
}

impl<'a> McpValue<'a> {
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Self::Int(v) => Some(v),
            Self::Float(v) if v.is_finite() => Some(v.round() as i64),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        self.as_i64().and_then(|v| i32::try_from(v).ok())
    }

    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Self::Float(v) if v.is_finite() => Some(v),
            Self::Int(v) => Some(v as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s.as_ref()),
            Self::Raw(s) => Some(*s),
            _ => None,
        }
    }

    pub fn as_tuple(&self) -> Option<&McpTuple<'a>> {
        match self {
            Self::Tuple(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[McpValue<'a>]> {
        match self {
            Self::List(items) => Some(items.as_slice()),
            _ => None,
        }
    }

    pub fn as_i32_list(&self) -> Option<Vec<i32>> {
        let items = self.as_list()?;
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            out.push(item.as_i32()?);
        }
        Some(out)
    }

    pub fn as_str_list(&self) -> Option<Vec<&str>> {
        let items = self.as_list()?;
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            out.push(item.as_str()?);
        }
        Some(out)
    }

    /// Recursively collects all tuples whose name matches any entry in `names` in pre-order.
    pub fn find_tuples(&self, names: &[&str]) -> Vec<&McpTuple<'a>> {
        let mut out = Vec::new();
        collect_tuples_in_value(self, names, &mut out);
        out
    }
}

fn collect_tuples_in_value<'a, 'b>(
    val: &'b McpValue<'a>,
    names: &[&str],
    out: &mut Vec<&'b McpTuple<'a>>,
) {
    match val {
        McpValue::Tuple(tuple) => {
            if names.contains(&tuple.name) {
                out.push(tuple);
            }
            for arg in &tuple.args {
                collect_tuples_in_value(arg, names, out);
            }
        }
        McpValue::List(items) => {
            for item in items {
                collect_tuples_in_value(item, names, out);
            }
        }
        _ => {}
    }
}

impl<'a> McpDocument<'a> {
    /// Returns declared field names for the first matching class name in `names`.
    pub fn fields_for(&self, names: &[&str]) -> Option<&[&'a str]> {
        for &name in names {
            if let Some(fields) = self.schemas.get(name) {
                return Some(fields.as_slice());
            }
        }
        None
    }

    /// Pairs declared schema field names with `tuple.args` iff the schema exists and arity matches.
    pub fn named_args<'b>(&self, tuple: &'b McpTuple<'a>) -> Option<Vec<(&'a str, &'b McpValue<'a>)>> {
        let fields = self.schemas.get(tuple.name)?;
        if fields.len() != tuple.args.len() {
            return None;
        }
        Some(fields.iter().copied().zip(tuple.args.iter()).collect())
    }

    /// Looks up a single field by case-insensitive name using the declared schema for `tuple`.
    pub fn field<'b>(&self, tuple: &'b McpTuple<'a>, field: &str) -> Option<&'b McpValue<'a>> {
        self.named_args(tuple)?
            .into_iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(field))
            .map(|(_, val)| val)
    }

    /// Recursively collects all tuples whose constructor matches any entry in `names` in pre-order.
    pub fn find_tuples(&self, names: &[&str]) -> Vec<&McpTuple<'a>> {
        let mut out = Vec::new();
        for root in &self.roots {
            if names.contains(&root.name) {
                out.push(root);
            }
            for arg in &root.args {
                collect_tuples_in_value(arg, names, &mut out);
            }
        }
        out
    }
}
