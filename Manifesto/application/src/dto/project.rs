use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::dto::ComponentResponse;

/// Request to create a new project
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateProjectRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    
    pub owner_type: String,  // personal or organization
    
    pub owner_id: Option<Uuid>,  // Required for organization projects
    
    pub visibility: Option<String>,  // private, internal, public
    
    pub external_collaboration_enabled: Option<bool>,
    
    pub data_classification: Option<String>,  // public, internal, confidential, restricted
}

/// Request to update a project
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateProjectRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    
    pub visibility: Option<String>,
    
    pub external_collaboration_enabled: Option<bool>,
    
    pub data_classification: Option<String>,
}

/// Project response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub owner_type: String,
    pub owner_id: Uuid,
    pub created_by: Uuid,
    pub visibility: String,
    pub external_collaboration_enabled: bool,
    pub data_classification: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

/// Detailed project response with components and member count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDetailResponse {
    #[serde(flatten)]
    pub project: ProjectResponse,
    pub components: Vec<ComponentResponse>,
    pub member_count: i64,
}

/// List projects response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectListResponse {
    pub data: Vec<ProjectResponse>,
    pub pagination: crate::dto::PaginationResponse,
}

