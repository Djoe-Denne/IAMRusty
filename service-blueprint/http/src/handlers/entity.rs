use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::{error::HttpError, validation::ValidatedJson};
use {{SERVICE_NAME}}_application::{
    CreateEntityRequest, EntityListResponse, EntityResponse, EntitySearchRequest,
    EntityUseCase, UpdateEntityRequest,
};

/// Query parameters for listing entities
#[derive(Debug, Deserialize, Validate)]
pub struct ListEntitiesQuery {
    #[validate(range(min = 1, max = 100, message = "Page size must be between 1 and 100"))]
    pub page_size: Option<u32>,
    
    #[validate(range(min = 1, message = "Page must be at least 1"))]
    pub page: Option<u32>,
    
    pub status: Option<String>,
}

/// Query parameters for searching entities
#[derive(Debug, Deserialize, Validate)]
pub struct SearchEntitiesQuery {
    #[validate(length(min = 1, message = "Query cannot be empty"))]
    pub q: String,
    
    #[validate(range(min = 1, max = 100, message = "Page size must be between 1 and 100"))]
    pub page_size: Option<u32>,
    
    #[validate(range(min = 1, message = "Page must be at least 1"))]
    pub page: Option<u32>,
    
    pub status: Option<String>,
}

/// Handler to create a new entity
pub async fn create_entity(
    State(use_case): State<EntityUseCase<impl {{SERVICE_NAME}}_application::ExampleEntityService>>,
    ValidatedJson(request): ValidatedJson<CreateEntityRequest>,
) -> Result<Response, HttpError> {
    let entity = use_case.create_entity(request).await?;
    Ok((StatusCode::CREATED, Json(entity)).into_response())
}

/// Handler to get an entity by ID
pub async fn get_entity(
    State(use_case): State<EntityUseCase<impl {{SERVICE_NAME}}_application::ExampleEntityService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<EntityResponse>, HttpError> {
    let entity = use_case.get_entity(id).await?;
    Ok(Json(entity))
}

/// Handler to update an entity
pub async fn update_entity(
    State(use_case): State<EntityUseCase<impl {{SERVICE_NAME}}_application::ExampleEntityService>>,
    Path(id): Path<Uuid>,
    ValidatedJson(request): ValidatedJson<UpdateEntityRequest>,
) -> Result<Json<EntityResponse>, HttpError> {
    let entity = use_case.update_entity(id, request).await?;
    Ok(Json(entity))
}

/// Handler to delete an entity
pub async fn delete_entity(
    State(use_case): State<EntityUseCase<impl {{SERVICE_NAME}}_application::ExampleEntityService>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, HttpError> {
    use_case.delete_entity(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Handler to list all entities
pub async fn list_entities(
    State(use_case): State<EntityUseCase<impl {{SERVICE_NAME}}_application::ExampleEntityService>>,
    Query(params): Query<ListEntitiesQuery>,
) -> Result<Json<Vec<EntityResponse>>, HttpError> {
    // Validate query parameters
    params.validate().map_err(|_| {
        HttpError::validation_error("Invalid query parameters")
    })?;

    let entities = if params.status.as_deref() == Some("active") {
        use_case.list_active_entities().await?
    } else {
        use_case.list_entities().await?
    };

    Ok(Json(entities))
}

/// Handler to search entities
pub async fn search_entities(
    State(use_case): State<EntityUseCase<impl {{SERVICE_NAME}}_application::ExampleEntityService>>,
    Query(params): Query<SearchEntitiesQuery>,
) -> Result<Json<EntityListResponse>, HttpError> {
    // Validate query parameters
    params.validate().map_err(|_| {
        HttpError::validation_error("Invalid search parameters")
    })?;

    let search_request = EntitySearchRequest {
        query: params.q,
        page: params.page,
        page_size: params.page_size,
        status_filter: params.status.as_ref().and_then(|s| {
            match s.as_str() {
                "active" => Some({{SERVICE_NAME}}_application::EntityStatusDto::Active),
                "inactive" => Some({{SERVICE_NAME}}_application::EntityStatusDto::Inactive),
                "pending" => Some({{SERVICE_NAME}}_application::EntityStatusDto::Pending),
                "archived" => Some({{SERVICE_NAME}}_application::EntityStatusDto::Archived),
                _ => None,
            }
        }),
    };

    let results = use_case.search_entities(search_request).await?;
    Ok(Json(results))
}

/// Handler to activate an entity
pub async fn activate_entity(
    State(use_case): State<EntityUseCase<impl {{SERVICE_NAME}}_application::ExampleEntityService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<EntityResponse>, HttpError> {
    let entity = use_case.activate_entity(id).await?;
    Ok(Json(entity))
}

/// Handler to deactivate an entity
pub async fn deactivate_entity(
    State(use_case): State<EntityUseCase<impl {{SERVICE_NAME}}_application::ExampleEntityService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<EntityResponse>, HttpError> {
    let entity = use_case.deactivate_entity(id).await?;
    Ok(Json(entity))
}

/// Handler to archive an entity
pub async fn archive_entity(
    State(use_case): State<EntityUseCase<impl {{SERVICE_NAME}}_application::ExampleEntityService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<EntityResponse>, HttpError> {
    let entity = use_case.archive_entity(id).await?;
    Ok(Json(entity))
}

/// Handler to get entity count
pub async fn get_entity_count(
    State(use_case): State<EntityUseCase<impl {{SERVICE_NAME}}_application::ExampleEntityService>>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let count = use_case.get_entity_count().await?;
    Ok(Json(serde_json::json!({ "count": count })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use {{SERVICE_NAME}}_application::command::MockEntityService;

    #[tokio::test]
    async fn test_list_entities_query_validation() {
        let valid_query = ListEntitiesQuery {
            page_size: Some(20),
            page: Some(1),
            status: Some("active".to_string()),
        };
        assert!(valid_query.validate().is_ok());

        let invalid_query = ListEntitiesQuery {
            page_size: Some(101), // Too large
            page: Some(0),        // Too small
            status: None,
        };
        assert!(invalid_query.validate().is_err());
    }

    #[tokio::test]
    async fn test_search_entities_query_validation() {
        let valid_query = SearchEntitiesQuery {
            q: "test".to_string(),
            page_size: Some(10),
            page: Some(1),
            status: Some("active".to_string()),
        };
        assert!(valid_query.validate().is_ok());

        let invalid_query = SearchEntitiesQuery {
            q: "".to_string(), // Empty query
            page_size: Some(101), // Too large
            page: Some(0),        // Too small
            status: None,
        };
        assert!(invalid_query.validate().is_err());
    }
} 