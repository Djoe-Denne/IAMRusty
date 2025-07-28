use uuid::Uuid;
use validator::Validate;

use crate::{
    dto::{
        CreateEntityRequest, EntityListResponse, EntityResponse, EntitySearchRequest,
        UpdateEntityRequest, ValidationError,
    },
    DomainError, ExampleEntityService,
};

/// Use case for entity management operations
pub struct EntityUseCase<S> {
    service: S,
}

impl<S> EntityUseCase<S>
where
    S: ExampleEntityService,
{
    pub fn new(service: S) -> Self {
        Self { service }
    }

    /// Create a new entity
    pub async fn create_entity(
        &self,
        request: CreateEntityRequest,
    ) -> Result<EntityResponse, ApplicationError> {
        // Validate input
        self.validate_request(&request)?;

        // Execute business logic
        let entity = self
            .service
            .create_entity(request.name, request.description)
            .await?;

        Ok(entity.into())
    }

    /// Update an existing entity
    pub async fn update_entity(
        &self,
        id: Uuid,
        request: UpdateEntityRequest,
    ) -> Result<EntityResponse, ApplicationError> {
        // Validate input
        self.validate_request(&request)?;

        // Execute business logic
        let entity = self
            .service
            .update_entity(&id, request.name, request.description)
            .await?;

        Ok(entity.into())
    }

    /// Get an entity by ID
    pub async fn get_entity(&self, id: Uuid) -> Result<EntityResponse, ApplicationError> {
        let entity = self.service.get_entity(&id).await?;
        Ok(entity.into())
    }

    /// List all entities
    pub async fn list_entities(&self) -> Result<Vec<EntityResponse>, ApplicationError> {
        let entities = self.service.list_entities().await?;
        Ok(entities.into_iter().map(Into::into).collect())
    }

    /// List only active entities
    pub async fn list_active_entities(&self) -> Result<Vec<EntityResponse>, ApplicationError> {
        let entities = self.service.list_active_entities().await?;
        Ok(entities.into_iter().map(Into::into).collect())
    }

    /// Search entities with pagination
    pub async fn search_entities(
        &self,
        request: EntitySearchRequest,
    ) -> Result<EntityListResponse, ApplicationError> {
        // Validate input
        self.validate_request(&request)?;

        let page = request.page.unwrap_or(1);
        let page_size = request.page_size.unwrap_or(20);

        // Search entities by name
        let entities = self.service.search_entities_by_name(&request.query).await?;

        // Apply status filter if provided
        let filtered_entities = if let Some(status_filter) = request.status_filter {
            entities
                .into_iter()
                .filter(|e| {
                    let entity_status: crate::EntityStatusDto = e.status.clone().into();
                    matches!(
                        (entity_status, &status_filter),
                        (crate::EntityStatusDto::Active, crate::EntityStatusDto::Active)
                            | (crate::EntityStatusDto::Inactive, crate::EntityStatusDto::Inactive)
                            | (crate::EntityStatusDto::Pending, crate::EntityStatusDto::Pending)
                            | (crate::EntityStatusDto::Archived, crate::EntityStatusDto::Archived)
                    )
                })
                .collect()
        } else {
            entities
        };

        // Calculate pagination
        let total_count = filtered_entities.len() as i64;
        let start_index = ((page - 1) * page_size) as usize;
        let end_index = (start_index + page_size as usize).min(filtered_entities.len());

        let paginated_entities = if start_index < filtered_entities.len() {
            filtered_entities[start_index..end_index].to_vec()
        } else {
            vec![]
        };

        Ok(EntityListResponse::new(
            paginated_entities,
            page,
            page_size,
            total_count,
        ))
    }

    /// Activate an entity
    pub async fn activate_entity(&self, id: Uuid) -> Result<EntityResponse, ApplicationError> {
        let entity = self.service.activate_entity(&id).await?;
        Ok(entity.into())
    }

    /// Deactivate an entity
    pub async fn deactivate_entity(&self, id: Uuid) -> Result<EntityResponse, ApplicationError> {
        let entity = self.service.deactivate_entity(&id).await?;
        Ok(entity.into())
    }

    /// Archive an entity
    pub async fn archive_entity(&self, id: Uuid) -> Result<EntityResponse, ApplicationError> {
        let entity = self.service.archive_entity(&id).await?;
        Ok(entity.into())
    }

    /// Delete an entity
    pub async fn delete_entity(&self, id: Uuid) -> Result<(), ApplicationError> {
        self.service.delete_entity(&id).await?;
        Ok(())
    }

    /// Get entity count
    pub async fn get_entity_count(&self) -> Result<i64, ApplicationError> {
        let count = self.service.count_entities().await?;
        Ok(count)
    }

    /// Validate a request using the validator crate
    fn validate_request<T: Validate>(&self, request: &T) -> Result<(), ApplicationError> {
        match request.validate() {
            Ok(_) => Ok(()),
            Err(validation_errors) => {
                let errors: Vec<ValidationError> = validation_errors
                    .field_errors()
                    .iter()
                    .flat_map(|(field, field_errors)| {
                        field_errors.iter().map(|error| ValidationError {
                            field: field.to_string(),
                            message: error
                                .message
                                .as_ref()
                                .map(|m| m.to_string())
                                .unwrap_or_else(|| "Validation error".to_string()),
                        })
                    })
                    .collect();

                Err(ApplicationError::ValidationError(errors))
            }
        }
    }
}

/// Application-specific errors
#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Validation error")]
    ValidationError(Vec<ValidationError>),

    #[error("External service error: {service}: {message}")]
    ExternalService { service: String, message: String },

    #[error("Internal application error: {message}")]
    Internal { message: String },
}

impl ApplicationError {
    pub fn external_service_error(service: &str, message: &str) -> Self {
        Self::ExternalService {
            service: service.to_string(),
            message: message.to_string(),
        }
    }

    pub fn internal_error(message: &str) -> Self {
        Self::Internal {
            message: message.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::MockEntityService;

    #[tokio::test]
    async fn test_create_entity_use_case() {
        let service = MockEntityService;
        let use_case = EntityUseCase::new(service);

        let request = CreateEntityRequest {
            name: "Test Entity".to_string(),
            description: Some("Test Description".to_string()),
        };

        let result = use_case.create_entity(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.name, "Test Entity");
        assert_eq!(response.description, Some("Test Description".to_string()));
    }

    #[tokio::test]
    async fn test_create_entity_validation_error() {
        let service = MockEntityService;
        let use_case = EntityUseCase::new(service);

        let request = CreateEntityRequest {
            name: "".to_string(), // Invalid: empty name
            description: Some("x".repeat(1001)), // Invalid: too long
        };

        let result = use_case.create_entity(request).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ApplicationError::ValidationError(errors) => {
                assert!(!errors.is_empty());
                assert!(errors.iter().any(|e| e.field == "name"));
                assert!(errors.iter().any(|e| e.field == "description"));
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[tokio::test]
    async fn test_search_entities() {
        let service = MockEntityService;
        let use_case = EntityUseCase::new(service);

        let request = EntitySearchRequest {
            query: "test".to_string(),
            page: Some(1),
            page_size: Some(10),
            status_filter: None,
        };

        let result = use_case.search_entities(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.page, 1);
        assert_eq!(response.page_size, 10);
    }

    #[tokio::test]
    async fn test_activate_entity() {
        let service = MockEntityService;
        let use_case = EntityUseCase::new(service);

        let result = use_case.activate_entity(Uuid::new_v4()).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(matches!(response.status, crate::EntityStatusDto::Active));
    }
} 