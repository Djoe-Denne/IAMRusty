//! Mesh oracle: echo the method and `x-principal-*` headers Envoy forwarded.

use axum::http::{HeaderMap, Method};
use axum::Json;
use serde_json::{Map, Value};

/// Return the request method and every `x-principal-*` header seen by IAM.
#[must_use = "await the future"]
pub async fn mesh_echo_headers(method: Method, headers: HeaderMap) -> Json<Value> {
    let mut principal = Map::new();
    for (name, value) in &headers {
        let key = name.as_str();
        if key.starts_with("x-principal-") {
            if let Ok(text) = value.to_str() {
                principal.insert(key.to_string(), Value::String(text.to_string()));
            }
        }
    }
    Json(serde_json::json!({
        "method": method.as_str(),
        "x-principal": principal,
    }))
}
