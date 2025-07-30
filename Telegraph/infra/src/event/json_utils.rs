//! JSON utility functions for template variable processing

use serde_json::Value;
use std::collections::HashMap;
use telegraph_domain::DomainError;

/// Convert a serde_json::Value to HashMap<String, String>
/// Flattens nested objects using dot notation (e.g., "user.email")
pub fn json_to_string_map(value: &Value) -> Result<HashMap<String, String>, DomainError> {
    let mut map = HashMap::new();

    // If we're skipping the root key, we expect the root to be an object
    // and we want to extract its fields directly (not nested under the root key)
    if let Value::Object(obj) = value {
        for (key, val) in obj {
            flatten_json_value(val, &key, &mut map)?;
        }
    } else {
        // If root is not an object, just flatten normally
        flatten_json_value(value, &"".to_string(), &mut map)?;
    }
    Ok(map)
}

/// Recursively flatten a JSON value into dot-notation string keys
pub fn flatten_json_value(
    value: &Value,
    prefix: &str,
    map: &mut HashMap<String, String>,
) -> Result<(), DomainError> {
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
            if !prefix.is_empty() {
                map.insert(prefix.to_string(), s.clone());
            }
        }
        Value::Number(n) => {
            if !prefix.is_empty() {
                map.insert(prefix.to_string(), n.to_string());
            }
        }
        Value::Bool(b) => {
            if !prefix.is_empty() {
                map.insert(prefix.to_string(), b.to_string());
            }
        }
        Value::Null => {
            if !prefix.is_empty() {
                map.insert(prefix.to_string(), "null".to_string());
            }
        }
    }
    Ok(())
}
