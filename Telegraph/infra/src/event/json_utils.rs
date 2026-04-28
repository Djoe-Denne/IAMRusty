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
        Value::Object(obj) => flatten_object(obj, prefix, map)?,
        Value::Array(arr) => flatten_array(arr, prefix, map)?,
        Value::String(s) => insert_prefixed(map, prefix, s.clone()),
        Value::Number(n) => insert_prefixed(map, prefix, n.to_string()),
        Value::Bool(b) => insert_prefixed(map, prefix, b.to_string()),
        Value::Null => insert_prefixed(map, prefix, "null".to_string()),
    }
    Ok(())
}

fn flatten_object(
    obj: &serde_json::Map<String, Value>,
    prefix: &str,
    map: &mut HashMap<String, String>,
) -> Result<(), DomainError> {
    for (key, val) in obj {
        flatten_json_value(val, &join_path(prefix, key), map)?;
    }
    Ok(())
}

fn flatten_array(
    arr: &[Value],
    prefix: &str,
    map: &mut HashMap<String, String>,
) -> Result<(), DomainError> {
    for (index, val) in arr.iter().enumerate() {
        flatten_json_value(val, &join_path(prefix, &index.to_string()), map)?;
    }
    Ok(())
}

fn join_path(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{}.{}", prefix, key)
    }
}

fn insert_prefixed(map: &mut HashMap<String, String>, prefix: &str, value: String) {
    if !prefix.is_empty() {
        map.insert(prefix.to_string(), value);
    }
}
