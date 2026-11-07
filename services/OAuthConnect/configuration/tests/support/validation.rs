//! Assertions against the public declarative validation API (G-1).

use super::support::assert_redacted;
use oauth_connect_configuration::{validate_config, AppConfig};
use serde_json::Value;

/// Resolve only real public fields; raw serde failures are never diagnostic output.
pub fn parsed(value: &Value) -> AppConfig {
    // Raw serde errors can contain input values: never print them.
    let result = serde_json::from_value(value.clone());
    assert!(
        result.is_ok(),
        "G-1: test declaration must have the real public shape"
    );
    result.unwrap()
}

/// Require semantic acceptance; callers additionally assert the resulting public fields.
pub fn accepted(value: &Value) -> AppConfig {
    let config = parsed(value);
    let result = validate_config(&config);
    assert!(
        result.is_ok(),
        "G-1: valid declarative configuration was refused"
    );
    config
}

/// Reach semantic validation after successful deserialization and require a redacted refusal.
pub fn rejected(value: &Value) {
    let config = parsed(value);
    let result = validate_config(&config);
    assert!(result.is_err(), "G-1: invalid declaration was admitted");
    let error = result.unwrap_err();
    assert_redacted(&format!("{error:?} {error}"));
}

/// Require a structural rejection (not a substitute for the real-loader assertions).
pub fn malformed(value: &Value) {
    let result = serde_json::from_value::<AppConfig>(value.clone());
    assert!(
        result.is_err(),
        "G-1: unknown/missing/mistyped field was accepted"
    );
    // Raw serde diagnostics are not a public error boundary. Loader tests cover sanitization.
}

/// Mutate an existing field; a typo is a fixture failure, not a negative test success.
pub fn change(value: &mut Value, pointer: &str, replacement: Value) {
    *value.pointer_mut(pointer).expect("fixture pointer exists") = replacement;
}

/// Remove a required existing field, failing immediately for a broken seed.
pub fn without(value: &mut Value, pointer: &str, key: &str) {
    let removed = value
        .pointer_mut(pointer)
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove(key);
    assert!(removed.is_some(), "fixture field to remove exists");
}
