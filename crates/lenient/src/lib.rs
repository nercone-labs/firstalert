//! Lenient reading of JSON values.
//!
//! A relay of public information may change a field from a number to a string, send `null` for
//! an unknown value, or add fields. A strict schema would then reject the whole message; for
//! information that must not be lost, such as an earthquake early warning, that is the worst
//! outcome. Reading each field leniently loses only the field that changed.
//!
//! Every accessor takes a path of object keys and returns `None` when a key is missing or the
//! value cannot be read as the type asked for.

use serde_json::Value;

/// Lenient accessors of a JSON value.
pub trait Lenient {
    /// The value at `path` (keys of nested objects).
    fn at(&self, path: &[&str]) -> Option<&Value>;

    /// A string; a number is written as JSON writes it. Empty strings are kept.
    fn string(&self, path: &[&str]) -> Option<String> {
        match self.at(path)? {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }

    /// A string with surrounding whitespace removed, or `None` when it is then empty.
    fn text(&self, path: &[&str]) -> Option<String> {
        self.string(path).map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
    }

    /// A finite number; a string holding a number (`"3.7"`, `" 10 "`) is read too.
    fn number(&self, path: &[&str]) -> Option<f64> {
        let value = match self.at(path)? {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        }?;
        value.is_finite().then_some(value)
    }

    /// An integer: a number without a fractional part, or a string holding one.
    fn integer(&self, path: &[&str]) -> Option<i64> {
        match self.at(path)? {
            Value::Number(n) => n.as_i64().or_else(|| n.as_f64().filter(|f| f.fract() == 0.0 && f.abs() < 9.0e15).map(|f| f as i64)),
            Value::String(s) => s.trim().parse::<i64>().ok(),
            _ => None,
        }
    }

    /// A boolean; the strings `"true"` and `"false"` and the numbers 1 and 0 are read too.
    fn boolean(&self, path: &[&str]) -> Option<bool> {
        match self.at(path)? {
            Value::Bool(b) => Some(*b),
            Value::String(s) => match s.trim() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            },
            Value::Number(n) => match n.as_i64() {
                Some(1) => Some(true),
                Some(0) => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    /// The elements of an array; anything else is no elements.
    fn elements(&self, path: &[&str]) -> &[Value] {
        match self.at(path) {
            Some(Value::Array(items)) => items,
            _ => &[],
        }
    }
}

impl Lenient for Value {
    fn at(&self, path: &[&str]) -> Option<&Value> {
        let mut value = self;
        for key in path {
            value = value.as_object()?.get(*key)?;
        }
        (!value.is_null()).then_some(value)
    }
}
