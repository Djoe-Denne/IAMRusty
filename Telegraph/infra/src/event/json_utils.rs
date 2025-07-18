//! JSON utility functions for template variable processing

use std::collections::HashMap;
use serde_json::Value;
use telegraph_domain::DomainError;

/// Convert a serde_json::Value to HashMap<String, String>
/// Flattens nested objects using dot notation (e.g., "user.email")
pub fn json_to_string_map(value: &Value) -> Result<HashMap<String, String>, DomainError> {
    let mut map = HashMap::new();
    flatten_json_value(value, "", &mut map)?;
    Ok(map)
}

/// Recursively flatten a JSON value into dot-notation string keys
pub fn flatten_json_value(value: &Value, prefix: &str, map: &mut HashMap<String, String>) -> Result<(), DomainError> {
    match value {
        Value::Object(obj) => {
            for (key, val) in obj {
                let new_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                flatten_json_value(val, &new_key, map)?;
            }
        }
        Value::Array(arr) => {
            for (index, val) in arr.iter().enumerate() {
                let new_key = if prefix.is_empty() {
                    index.to_string()
                } else {
                    format!("{}.{}", prefix, index)
                };
                flatten_json_value(val, &new_key, map)?;
            }
        }
        Value::String(s) => {
            map.insert(prefix.to_string(), s.clone());
        }
        Value::Number(n) => {
            map.insert(prefix.to_string(), n.to_string());
        }
        Value::Bool(b) => {
            map.insert(prefix.to_string(), b.to_string());
        }
        Value::Null => {
            map.insert(prefix.to_string(), "null".to_string());
        }
    }
    Ok(())
}
