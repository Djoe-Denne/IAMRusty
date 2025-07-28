use rustycog_command::{Command, CommandHandler, CommandResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    dto::{CreateEntityRequest, EntityResponse, UpdateEntityRequest},
    DomainError, ExampleEntityService,
};

/// Command to create a new entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntityCommand {
    pub name: String,
    pub description: Option<String>,
}

/// Command to update an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEntityCommand {
    pub id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Command to activate an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivateEntityCommand {
    pub id: Uuid,
}

/// Command to deactivate an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeactivateEntityCommand {
    pub id: Uuid,
}

/// Command to archive an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntityCommand {
    pub id: Uuid,
}

/// Command to delete an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteEntityCommand {
    pub id: Uuid,
}

impl From<CreateEntityRequest> for CreateEntityCommand {
    fn from(request: CreateEntityRequest) -> Self {
        Self {
            name: request.name,
            description: request.description,
        }
    }
}

impl From<UpdateEntityRequest> for UpdateEntityCommand {
    fn from(request: UpdateEntityRequest) -> Self {
        Self {
            id: Uuid::new_v4(), // This would typically be set by the handler
            name: request.name,
            description: request.description,
        }
    }
}

/// Command handler for entity-related commands
pub struct EntityCommandHandler<S> {
    service: S,
}

impl<S> EntityCommandHandler<S> {
    pub fn new(service: S) -> Self {
        Self { service }
    }
}

impl<S> CommandHandler<CreateEntityCommand> for EntityCommandHandler<S>
where
    S: ExampleEntityService + Send + Sync + 'static,
{
    type Output = EntityResponse;

    async fn handle(&self, command: CreateEntityCommand) -> CommandResult<Self::Output> {
        let entity = self
            .service
            .create_entity(command.name, command.description)
            .await
            .map_err(|e| e.to_string())?;

        Ok(entity.into())
    }
}

impl<S> CommandHandler<UpdateEntityCommand> for EntityCommandHandler<S>
where
    S: ExampleEntityService + Send + Sync + 'static,
{
    type Output = EntityResponse;

    async fn handle(&self, command: UpdateEntityCommand) -> CommandResult<Self::Output> {
        let entity = self
            .service
            .update_entity(&command.id, command.name, command.description)
            .await
            .map_err(|e| e.to_string())?;

        Ok(entity.into())
    }
}

impl<S> CommandHandler<ActivateEntityCommand> for EntityCommandHandler<S>
where
    S: ExampleEntityService + Send + Sync + 'static,
{
    type Output = EntityResponse;

    async fn handle(&self, command: ActivateEntityCommand) -> CommandResult<Self::Output> {
        let entity = self
            .service
            .activate_entity(&command.id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(entity.into())
    }
}

impl<S> CommandHandler<DeactivateEntityCommand> for EntityCommandHandler<S>
where
    S: ExampleEntityService + Send + Sync + 'static,
{
    type Output = EntityResponse;

    async fn handle(&self, command: DeactivateEntityCommand) -> CommandResult<Self::Output> {
        let entity = self
            .service
            .deactivate_entity(&command.id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(entity.into())
    }
}

impl<S> CommandHandler<ArchiveEntityCommand> for EntityCommandHandler<S>
where
    S: ExampleEntityService + Send + Sync + 'static,
{
    type Output = EntityResponse;

    async fn handle(&self, command: ArchiveEntityCommand) -> CommandResult<Self::Output> {
        let entity = self
            .service
            .archive_entity(&command.id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(entity.into())
    }
}

impl<S> CommandHandler<DeleteEntityCommand> for EntityCommandHandler<S>
where
    S: ExampleEntityService + Send + Sync + 'static,
{
    type Output = ();

    async fn handle(&self, command: DeleteEntityCommand) -> CommandResult<Self::Output> {
        self.service
            .delete_entity(&command.id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

/// Helper trait to define what an ExampleEntityService should provide
/// This allows for easier testing and mocking
pub trait ExampleEntityService {
    async fn create_entity(
        &self,
        name: String,
        description: Option<String>,
    ) -> Result<crate::ExampleEntity, DomainError>;

    async fn update_entity(
        &self,
        id: &Uuid,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<crate::ExampleEntity, DomainError>;

    async fn activate_entity(&self, id: &Uuid) -> Result<crate::ExampleEntity, DomainError>;

    async fn deactivate_entity(&self, id: &Uuid) -> Result<crate::ExampleEntity, DomainError>;

    async fn archive_entity(&self, id: &Uuid) -> Result<crate::ExampleEntity, DomainError>;

    async fn delete_entity(&self, id: &Uuid) -> Result<(), DomainError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use uuid::Uuid;

    // Mock service for testing
    pub struct MockEntityService;

    #[async_trait]
    impl ExampleEntityService for MockEntityService {
        async fn create_entity(
            &self,
            name: String,
            description: Option<String>,
        ) -> Result<crate::ExampleEntity, DomainError> {
            crate::ExampleEntity::new(name, description)
        }

        async fn update_entity(
            &self,
            _id: &Uuid,
            name: Option<String>,
            description: Option<String>,
        ) -> Result<crate::ExampleEntity, DomainError> {
            let mut entity = crate::ExampleEntity::new("Test".to_string(), None)?;
            if let Some(new_name) = name {
                entity.update_name(new_name)?;
            }
            if let Some(new_description) = description {
                entity.update_description(Some(new_description));
            }
            Ok(entity)
        }

        async fn activate_entity(&self, _id: &Uuid) -> Result<crate::ExampleEntity, DomainError> {
            let mut entity = crate::ExampleEntity::new("Test".to_string(), None)?;
            entity.activate()?;
            Ok(entity)
        }

        async fn deactivate_entity(&self, _id: &Uuid) -> Result<crate::ExampleEntity, DomainError> {
            let mut entity = crate::ExampleEntity::new("Test".to_string(), None)?;
            entity.deactivate();
            Ok(entity)
        }

        async fn archive_entity(&self, _id: &Uuid) -> Result<crate::ExampleEntity, DomainError> {
            let mut entity = crate::ExampleEntity::new("Test".to_string(), None)?;
            entity.deactivate(); // Must deactivate before archiving
            entity.archive()?;
            Ok(entity)
        }

        async fn delete_entity(&self, _id: &Uuid) -> Result<(), DomainError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_create_entity_command() {
        let service = MockEntityService;
        let handler = EntityCommandHandler::new(service);

        let command = CreateEntityCommand {
            name: "Test Entity".to_string(),
            description: Some("Test Description".to_string()),
        };

        let result = handler.handle(command).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.name, "Test Entity");
        assert_eq!(response.description, Some("Test Description".to_string()));
    }

    #[tokio::test]
    async fn test_activate_entity_command() {
        let service = MockEntityService;
        let handler = EntityCommandHandler::new(service);

        let command = ActivateEntityCommand {
            id: Uuid::new_v4(),
        };

        let result = handler.handle(command).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(matches!(response.status, crate::EntityStatusDto::Active));
    }

    #[tokio::test]
    async fn test_delete_entity_command() {
        let service = MockEntityService;
        let handler = EntityCommandHandler::new(service);

        let command = DeleteEntityCommand {
            id: Uuid::new_v4(),
        };

        let result = handler.handle(command).await;
        assert!(result.is_ok());
    }
} 