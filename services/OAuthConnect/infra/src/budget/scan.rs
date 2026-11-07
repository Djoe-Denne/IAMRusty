//! Bounded structural scan with one borrowed iterator per container level.

use serde_json::Value;

use super::{capped_add, length, MAX_JSON_DEPTH, MAX_SCAN_UNITS_PER_API};
use crate::{invalid_mapping, DomainError};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ApiStats {
    pub nodes: u64,
    pub bytes: u64,
    pub units: u64,
    pub max_array: u64,
    pub max_object: u64,
    pub max_key: u64,
}

impl ApiStats {
    pub fn fanout(&self) -> u64 {
        self.max_array.max(self.max_object).max(1)
    }

    fn add_bytes(&mut self, bytes: usize) -> Result<(), DomainError> {
        let bytes = length(bytes);
        self.bytes = capped_add(self.bytes, bytes, MAX_SCAN_UNITS_PER_API);
        self.units = capped_add(self.units, bytes, MAX_SCAN_UNITS_PER_API);
        self.check()
    }

    fn add_node(&mut self) -> Result<(), DomainError> {
        self.nodes = capped_add(self.nodes, 1, MAX_SCAN_UNITS_PER_API);
        self.units = capped_add(self.units, 1, MAX_SCAN_UNITS_PER_API);
        self.check()
    }

    fn check(&self) -> Result<(), DomainError> {
        if self.units > MAX_SCAN_UNITS_PER_API {
            return Err(invalid_mapping());
        }
        Ok(())
    }
}

enum Frame<'a> {
    Array(std::slice::Iter<'a, Value>),
    Object(serde_json::map::Iter<'a>),
}

impl<'a> Frame<'a> {
    fn next(&mut self) -> Option<(Option<&'a str>, &'a Value)> {
        match self {
            Self::Array(items) => items.next().map(|value| (None, value)),
            Self::Object(items) => items.next().map(|(key, value)| (Some(key.as_str()), value)),
        }
    }
}

/// # Errors
/// Refuses logical volume above `1_048_576` units or depth above 128 (root = 0).
pub fn scan_api(value: &Value) -> Result<ApiStats, DomainError> {
    let mut stats = ApiStats::default();
    let mut frames = Vec::with_capacity(MAX_JSON_DEPTH + 1);
    visit(value, &mut stats, &mut frames)?;
    while let Some(frame) = frames.last_mut() {
        if let Some((key, child)) = frame.next() {
            // No recursive descent and no wide collection of pending children.
            // An iterator at depth 128 may exist, but its depth-129 child may not.
            if frames.len() > MAX_JSON_DEPTH {
                return Err(invalid_mapping());
            }
            if let Some(key) = key {
                stats.max_key = stats.max_key.max(length(key.len()));
                stats.add_bytes(key.len())?;
            }
            visit(child, &mut stats, &mut frames)?;
        } else {
            let _ = frames.pop();
        }
    }
    Ok(stats)
}

fn visit<'a>(
    value: &'a Value,
    stats: &mut ApiStats,
    frames: &mut Vec<Frame<'a>>,
) -> Result<(), DomainError> {
    stats.add_node()?;
    match value {
        Value::String(value) => stats.add_bytes(value.len())?,
        Value::Array(items) => {
            stats.max_array = stats.max_array.max(length(items.len()));
            frames.push(Frame::Array(items.iter()));
        }
        Value::Object(items) => {
            stats.max_object = stats.max_object.max(length(items.len()));
            frames.push(Frame::Object(items.iter()));
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
    Ok(())
}
