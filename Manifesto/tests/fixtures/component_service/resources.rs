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
        }
    }
}
