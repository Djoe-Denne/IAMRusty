//! Wire types for the upstream component-catalog service.
//!
//! Mirrors `manifesto_domain::port::ComponentInfo` so the wiremock fake's
//! response body deserializes cleanly through the production
//! `ComponentServiceClient`.

use serde::{Deserialize, Serialize};

/// Component metadata returned by `GET /api/components`.
///
/// Field-by-field equivalent of `manifesto_domain::port::ComponentInfo` so
/// that `set_body_json` produces a payload the production client can decode
/// without any glue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInfoBody {
    pub component_type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub version: String,
    pub endpoint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared_capabilities: Option<Vec<String>>,
}

impl ComponentInfoBody {
    /// Convenience constructor — the only fields tests actually care about
    /// are `component_type` and a non-empty endpoint, but we keep the full
    /// shape so changes to `ComponentInfo` surface here at compile time.
    pub fn new(component_type: impl Into<String>) -> Self {
        let component_type = component_type.into();
        Self {
            component_type: component_type.clone(),
            name: format!("{component_type} component"),
            description: Some(format!("Test fixture component for {component_type}")),
            version: "1.0.0".to_string(),
            endpoint: format!("http://127.0.0.1:0/components/{component_type}"),
            digest: None,
            declared_capabilities: None,
        }
    }

    /// Pin gold-path `io.aiforall.reference-kv` (digest descripteur 0002 + requires).
    #[must_use]
    pub fn reference_kv(digest: impl Into<String>) -> Self {
        Self {
            component_type: "io.aiforall.reference-kv".to_owned(),
            name: "Reference KV".to_owned(),
            description: Some("Apparatus KV de référence".to_owned()),
            version: "1.0.0".to_owned(),
            endpoint: "http://127.0.0.1:8080/lazaret".to_owned(),
            digest: Some(digest.into()),
            declared_capabilities: Some(vec![
                "project.read".to_owned(),
                "storage.kv.read".to_owned(),
                "storage.kv.write".to_owned(),
            ]),
        }
    }
}
