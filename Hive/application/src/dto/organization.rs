use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use validator::Validate;

use crate::Organization;

// Regex for validating organization slugs
lazy_static::lazy_static! {
    static ref RE_SLUG: Regex = Regex::new(r"^[a-z0-9-]+$").unwrap();
}

/// DTO for creating a new organization
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateOrganizationRequest {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Name must be between 1 and 255 characters"
    ))]
    pub name: String,

    #[validate(
        length(
            min = 1,
            max = 100,
            message = "Slug must be between 1 and 100 characters"
        ),
        regex(
            path = "RE_SLUG",
            message = "Slug must contain only lowercase letters, numbers, and hyphens"
        )
    )]
    pub slug: String,

    #[validate(length(max = 1000, message = "Description cannot exceed 1000 characters"))]
    pub description: Option<String>,

    #[validate(url(message = "Avatar URL must be a valid URL"))]
    pub avatar_url: Option<String>,
}

/// DTO for updating an organization
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateOrganizationRequest {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Name must be between 1 and 255 characters"
    ))]
    pub name: Option<String>,

    #[validate(length(max = 1000, message = "Description cannot exceed 1000 characters"))]
    pub description: Option<String>,

    #[validate(url(message = "Avatar URL must be a valid URL"))]
    pub avatar_url: Option<String>,

    pub settings: Option<Value>,
}

/// DTO for organization response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub owner_user_id: Uuid,
    pub settings: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Additional computed fields
    pub member_count: Option<i64>,
    pub role_count: Option<i64>,
    pub is_owner: Option<bool>,
    pub user_role: Option<String>,
}

/// DTO for organization list response with pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationListResponse {
    pub organizations: Vec<OrganizationResponse>,
    pub pagination: super::PaginationResponse,
}

/// DTO for organization search request
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct OrganizationSearchRequest {
    #[validate(length(min = 1, message = "Search term cannot be empty"))]
    pub query: String,

    #[validate(range(min = 1, max = 100, message = "Page size must be between 1 and 100"))]
    pub page_size: Option<u32>,

    #[validate(range(min = 1, message = "Page must be at least 1"))]
    pub page: Option<u32>,

    pub role_filter: Option<String>,
}

/// DTO for organization statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationStatsResponse {
    pub organization_id: Uuid,
    pub total_members: i64,
    pub active_members: i64,
    pub pending_members: i64,
    pub suspended_members: i64,
    pub total_roles: i64,
    pub custom_roles: i64,
    pub pending_invitations: i64,
    pub external_links: i64,
    pub active_sync_jobs: i64,
}

impl From<Organization> for OrganizationResponse {
    fn from(org: Organization) -> Self {
        Self {
            id: org.id,
            name: org.name,
            slug: org.slug,
            description: org.description,
            avatar_url: org.avatar_url,
            owner_user_id: org.owner_user_id,
            settings: org.settings,
            created_at: org.created_at,
            updated_at: org.updated_at,
            member_count: None,
            role_count: None,
            is_owner: None,
            user_role: None,
        }
    }
}

impl OrganizationResponse {
    /// Create response with additional computed fields
    pub fn with_details(
        organization: Organization,
        member_count: Option<i64>,
        role_count: Option<i64>,
        is_owner: Option<bool>,
        user_role: Option<String>,
    ) -> Self {
        Self {
            id: organization.id,
            name: organization.name,
            slug: organization.slug,
            description: organization.description,
            avatar_url: organization.avatar_url,
            owner_user_id: organization.owner_user_id,
            settings: organization.settings,
            created_at: organization.created_at,
            updated_at: organization.updated_at,
            member_count,
            role_count,
            is_owner,
            user_role,
        }
    }
}

impl OrganizationListResponse {
    /// Create a new list response
    pub fn new(
        organizations: Vec<OrganizationResponse>,
        pagination: super::PaginationResponse,
    ) -> Self {
        Self {
            organizations,
            pagination,
        }
    }
}

impl Default for OrganizationSearchRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            page_size: Some(20),
            page: Some(1),
            role_filter: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_organization_request_validation() {
        let valid_request = CreateOrganizationRequest {
            name: "Test Org".to_string(),
            slug: "test-org".to_string(),
            description: Some("Test Description".to_string()),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = CreateOrganizationRequest {
            name: "".to_string(),                      // Empty name should fail
            slug: "Test@Org".to_string(),              // Invalid slug format
            description: Some("x".repeat(1001)),       // Too long description
            avatar_url: Some("not-a-url".to_string()), // Invalid URL
        };
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_organization_response_conversion() {
        let owner_id = Uuid::new_v4();
        let org = Organization::new(
            "Test Org".to_string(),
            "test-org".to_string(),
            Some("Test Description".to_string()),
            owner_id,
        )
        .unwrap();

        let response: OrganizationResponse = org.into();
        assert_eq!(response.name, "Test Org");
        assert_eq!(response.slug, "test-org");
        assert_eq!(response.owner_user_id, owner_id);
    }

    #[test]
    fn test_organization_search_request_validation() {
        let valid_request = OrganizationSearchRequest {
            query: "test".to_string(),
            page_size: Some(10),
            page: Some(1),
            role_filter: Some("admin".to_string()),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = OrganizationSearchRequest {
            query: "".to_string(), // Empty query should fail
            page_size: Some(101),  // Too large page size
            page: Some(0),         // Invalid page number
            role_filter: None,
        };
        assert!(invalid_request.validate().is_err());
    }
}
