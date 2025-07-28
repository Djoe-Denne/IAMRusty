use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{DomainError, ExampleEntity, EntityStatus};

/// DTO for creating a new entity
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateEntityRequest {
    #[validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters"))]
    pub name: String,
    
    #[validate(length(max = 1000, message = "Description cannot exceed 1000 characters"))]
    pub description: Option<String>,
}

/// DTO for updating an entity
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateEntityRequest {
    #[validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters"))]
    pub name: Option<String>,
    
    #[validate(length(max = 1000, message = "Description cannot exceed 1000 characters"))]
    pub description: Option<String>,
}

/// DTO for entity response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: EntityStatusDto,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for entity status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityStatusDto {
    Active,
    Inactive,
    Pending,
    Archived,
}

/// DTO for paginated entity list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityListResponse {
    pub entities: Vec<EntityResponse>,
    pub page: u32,
    pub page_size: u32,
    pub total_count: i64,
    pub total_pages: u32,
}

/// DTO for entity search request
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct EntitySearchRequest {
    #[validate(length(min = 1, message = "Search term cannot be empty"))]
    pub query: String,
    
    #[validate(range(min = 1, max = 100, message = "Page size must be between 1 and 100"))]
    pub page_size: Option<u32>,
    
    #[validate(range(min = 1, message = "Page must be at least 1"))]
    pub page: Option<u32>,
    
    pub status_filter: Option<EntityStatusDto>,
}

/// DTO for validation errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

/// DTO for API error responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error_type: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub validation_errors: Option<Vec<ValidationError>>,
}

impl From<ExampleEntity> for EntityResponse {
    fn from(entity: ExampleEntity) -> Self {
        Self {
            id: entity.id,
            name: entity.name,
            description: entity.description,
            status: entity.status.into(),
            created_at: entity.created_at,
            updated_at: entity.updated_at,
        }
    }
}

impl From<EntityStatus> for EntityStatusDto {
    fn from(status: EntityStatus) -> Self {
        match status {
            EntityStatus::Active => EntityStatusDto::Active,
            EntityStatus::Inactive => EntityStatusDto::Inactive,
            EntityStatus::Pending => EntityStatusDto::Pending,
            EntityStatus::Archived => EntityStatusDto::Archived,
        }
    }
}

impl From<EntityStatusDto> for EntityStatus {
    fn from(status_dto: EntityStatusDto) -> Self {
        match status_dto {
            EntityStatusDto::Active => EntityStatus::Active,
            EntityStatusDto::Inactive => EntityStatus::Inactive,
            EntityStatusDto::Pending => EntityStatus::Pending,
            EntityStatusDto::Archived => EntityStatus::Archived,
        }
    }
}

impl From<DomainError> for ApiError {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::EntityNotFound { entity_type, id } => ApiError {
                error_type: "entity_not_found".to_string(),
                message: format!("{} not found", entity_type),
                details: Some(serde_json::json!({ "id": id })),
                validation_errors: None,
            },
            DomainError::InvalidInput { message } => ApiError {
                error_type: "invalid_input".to_string(),
                message,
                details: None,
                validation_errors: None,
            },
            DomainError::BusinessRuleViolation { rule } => ApiError {
                error_type: "business_rule_violation".to_string(),
                message: rule,
                details: None,
                validation_errors: None,
            },
            DomainError::Unauthorized { operation } => ApiError {
                error_type: "unauthorized".to_string(),
                message: format!("Unauthorized: {}", operation),
                details: None,
                validation_errors: None,
            },
            DomainError::ResourceAlreadyExists { resource_type, identifier } => ApiError {
                error_type: "resource_already_exists".to_string(),
                message: format!("{} already exists", resource_type),
                details: Some(serde_json::json!({ "identifier": identifier })),
                validation_errors: None,
            },
            DomainError::ExternalServiceError { service, message } => ApiError {
                error_type: "external_service_error".to_string(),
                message: format!("External service error: {}", service),
                details: Some(serde_json::json!({ "service": service, "error": message })),
                validation_errors: None,
            },
            DomainError::Internal { message } => ApiError {
                error_type: "internal_error".to_string(),
                message: "An internal error occurred".to_string(),
                details: Some(serde_json::json!({ "error": message })),
                validation_errors: None,
            },
        }
    }
}

impl EntityListResponse {
    pub fn new(
        entities: Vec<ExampleEntity>,
        page: u32,
        page_size: u32,
        total_count: i64,
    ) -> Self {
        let entity_responses: Vec<EntityResponse> = entities.into_iter().map(Into::into).collect();
        let total_pages = ((total_count as f64) / (page_size as f64)).ceil() as u32;

        Self {
            entities: entity_responses,
            page,
            page_size,
            total_count,
            total_pages,
        }
    }
}

impl Default for EntitySearchRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            page_size: Some(20),
            page: Some(1),
            status_filter: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_entity_request_validation() {
        let valid_request = CreateEntityRequest {
            name: "Test Entity".to_string(),
            description: Some("Test Description".to_string()),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = CreateEntityRequest {
            name: "".to_string(), // Empty name should fail
            description: Some("x".repeat(1001)), // Too long description should fail
        };
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_entity_response_conversion() {
        let entity = ExampleEntity::new(
            "Test Entity".to_string(),
            Some("Test Description".to_string()),
        ).unwrap();
        
        let response: EntityResponse = entity.into();
        assert_eq!(response.name, "Test Entity");
        assert_eq!(response.description, Some("Test Description".to_string()));
    }

    #[test]
    fn test_entity_status_conversion() {
        let domain_status = EntityStatus::Active;
        let dto_status: EntityStatusDto = domain_status.into();
        let back_to_domain: EntityStatus = dto_status.into();
        
        assert!(matches!(back_to_domain, EntityStatus::Active));
    }

    #[test]
    fn test_entity_list_response() {
        let entities = vec![
            ExampleEntity::new("Entity 1".to_string(), None).unwrap(),
            ExampleEntity::new("Entity 2".to_string(), None).unwrap(),
        ];
        
        let response = EntityListResponse::new(entities, 1, 10, 25);
        
        assert_eq!(response.entities.len(), 2);
        assert_eq!(response.page, 1);
        assert_eq!(response.page_size, 10);
        assert_eq!(response.total_count, 25);
        assert_eq!(response.total_pages, 3);
    }
} 