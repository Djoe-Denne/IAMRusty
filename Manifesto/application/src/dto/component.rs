use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Request to add a component to a project
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AddComponentRequest {
    #[validate(length(min = 1, max = 100))]
    pub component_type: String,
}

/// Request to update a component's status
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateComponentRequest {
    pub status: String, // pending, configured, active, disabled
}

/// Component response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentResponse {
    pub id: Uuid,
    pub component_type: String,
    pub status: String,
    pub endpoint: Option<String>,     // From component service
    pub access_token: Option<String>, // Component-scoped JWT (not implemented yet)
    pub added_at: DateTime<Utc>,
    pub configured_at: Option<DateTime<Utc>>,
    pub activated_at: Option<DateTime<Utc>>,
    pub disabled_at: Option<DateTime<Utc>>,
}

/// List components response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentListResponse {
    pub data: Vec<ComponentResponse>,
}
