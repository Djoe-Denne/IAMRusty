mod common;
mod fixtures;

use fixtures::{DatabaseFixture, EntityFixtures, InMemoryDatabaseFixture};
use testcontainers::clients::Cli;

use {{SERVICE_NAME}}_application::{CreateEntityRequest, EntityUseCase, UpdateEntityRequest};
use {{SERVICE_NAME}}_domain::{ExampleEntityService as DomainEntityService, EntityStatus};
use {{SERVICE_NAME}}_infra::{
    repository::PostgresExampleEntityRepository,
    AuditLogRepository,
};

/// Integration tests for the entity use case
#[cfg(test)]
mod entity_use_case_tests {
    use super::*;

    // Helper to create a test audit repository
    struct TestAuditRepository;

    #[async_trait::async_trait]
    impl AuditLogRepository for TestAuditRepository {
        async fn log_event(
            &self,
            _entity_id: &uuid::Uuid,
            _entity_type: &str,
            _action: &str,
            _details: Option<&str>,
        ) -> Result<(), {{SERVICE_NAME}}_domain::DomainError> {
            Ok(())
        }

        async fn get_audit_history(
            &self,
            _entity_id: &uuid::Uuid,
        ) -> Result<Vec<{{SERVICE_NAME}}_domain::AuditLogEntry>, {{SERVICE_NAME}}_domain::DomainError> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_create_entity_use_case() {
        common::init();
        
        // Setup database
        let db_fixture = InMemoryDatabaseFixture::new().await.unwrap();
        let entity_repository = PostgresExampleEntityRepository::new(db_fixture.connection().clone());
        let audit_repository = TestAuditRepository;
        let domain_service = DomainEntityService::new(entity_repository, audit_repository);
        let use_case = EntityUseCase::new(domain_service);

        // Test data
        let request = CreateEntityRequest {
            name: "Test Entity".to_string(),
            description: Some("Test Description".to_string()),
        };

        // Execute
        let result = use_case.create_entity(request).await;

        // Assert
        assert!(result.is_ok());
        let entity_response = result.unwrap();
        assert_eq!(entity_response.name, "Test Entity");
        assert_eq!(entity_response.description, Some("Test Description".to_string()));
    }

    #[tokio::test]
    async fn test_update_entity_use_case() {
        common::init();
        
        // Setup database
        let db_fixture = InMemoryDatabaseFixture::new().await.unwrap();
        let entity_repository = PostgresExampleEntityRepository::new(db_fixture.connection().clone());
        let audit_repository = TestAuditRepository;
        let domain_service = DomainEntityService::new(entity_repository, audit_repository);
        let use_case = EntityUseCase::new(domain_service);

        // Create an entity first
        let create_request = CreateEntityRequest {
            name: "Original Name".to_string(),
            description: Some("Original Description".to_string()),
        };
        let created_entity = use_case.create_entity(create_request).await.unwrap();

        // Update the entity
        let update_request = UpdateEntityRequest {
            name: Some("Updated Name".to_string()),
            description: Some("Updated Description".to_string()),
        };

        let result = use_case.update_entity(created_entity.id, update_request).await;

        // Assert
        assert!(result.is_ok());
        let updated_entity = result.unwrap();
        assert_eq!(updated_entity.name, "Updated Name");
        assert_eq!(updated_entity.description, Some("Updated Description".to_string()));
    }

    #[tokio::test]
    async fn test_entity_lifecycle() {
        common::init();
        
        // Setup database
        let db_fixture = InMemoryDatabaseFixture::new().await.unwrap();
        let entity_repository = PostgresExampleEntityRepository::new(db_fixture.connection().clone());
        let audit_repository = TestAuditRepository;
        let domain_service = DomainEntityService::new(entity_repository, audit_repository);
        let use_case = EntityUseCase::new(domain_service);

        // Create entity
        let create_request = CreateEntityRequest {
            name: "Lifecycle Test Entity".to_string(),
            description: Some("Testing entity lifecycle".to_string()),
        };
        let entity = use_case.create_entity(create_request).await.unwrap();
        assert!(matches!(entity.status, {{SERVICE_NAME}}_application::EntityStatusDto::Pending));

        // Activate entity
        let activated_entity = use_case.activate_entity(entity.id).await.unwrap();
        assert!(matches!(activated_entity.status, {{SERVICE_NAME}}_application::EntityStatusDto::Active));

        // Deactivate entity
        let deactivated_entity = use_case.deactivate_entity(entity.id).await.unwrap();
        assert!(matches!(deactivated_entity.status, {{SERVICE_NAME}}_application::EntityStatusDto::Inactive));

        // Archive entity
        let archived_entity = use_case.archive_entity(entity.id).await.unwrap();
        assert!(matches!(archived_entity.status, {{SERVICE_NAME}}_application::EntityStatusDto::Archived));

        // Delete entity
        let delete_result = use_case.delete_entity(entity.id).await;
        assert!(delete_result.is_ok());

        // Verify entity is deleted
        let get_result = use_case.get_entity(entity.id).await;
        assert!(get_result.is_err());
    }

    #[tokio::test]
    async fn test_list_entities() {
        common::init();
        
        // Setup database
        let db_fixture = InMemoryDatabaseFixture::new().await.unwrap();
        let entity_repository = PostgresExampleEntityRepository::new(db_fixture.connection().clone());
        let audit_repository = TestAuditRepository;
        let domain_service = DomainEntityService::new(entity_repository, audit_repository);
        let use_case = EntityUseCase::new(domain_service);

        // Create multiple entities
        for i in 1..=5 {
            let request = CreateEntityRequest {
                name: format!("Entity {}", i),
                description: Some(format!("Description {}", i)),
            };
            use_case.create_entity(request).await.unwrap();
        }

        // List all entities
        let all_entities = use_case.list_entities().await.unwrap();
        assert_eq!(all_entities.len(), 5);

        // Activate some entities
        for entity in &all_entities[0..3] {
            use_case.activate_entity(entity.id).await.unwrap();
        }

        // List only active entities
        let active_entities = use_case.list_active_entities().await.unwrap();
        assert_eq!(active_entities.len(), 3);
    }

    #[tokio::test]
    async fn test_entity_count() {
        common::init();
        
        // Setup database
        let db_fixture = InMemoryDatabaseFixture::new().await.unwrap();
        let entity_repository = PostgresExampleEntityRepository::new(db_fixture.connection().clone());
        let audit_repository = TestAuditRepository;
        let domain_service = DomainEntityService::new(entity_repository, audit_repository);
        let use_case = EntityUseCase::new(domain_service);

        // Initially no entities
        let initial_count = use_case.get_entity_count().await.unwrap();
        assert_eq!(initial_count, 0);

        // Create some entities
        for i in 1..=3 {
            let request = CreateEntityRequest {
                name: format!("Entity {}", i),
                description: None,
            };
            use_case.create_entity(request).await.unwrap();
        }

        // Check count
        let final_count = use_case.get_entity_count().await.unwrap();
        assert_eq!(final_count, 3);
    }
}

/// Integration tests with real PostgreSQL database
#[cfg(test)]
mod postgres_integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Only run when PostgreSQL is available
    async fn test_with_postgres_database() {
        common::init();
        
        let docker = Cli::default();
        let db_fixture = DatabaseFixture::new(&docker).await.unwrap();
        
        // Clean database before test
        db_fixture.clean_database().await.unwrap();
        
        // Your test logic here...
        // This would be similar to the in-memory tests but using real PostgreSQL
        
        let entity_repository = PostgresExampleEntityRepository::new(db_fixture.connection().clone());
        
        // Test that we can create and retrieve an entity
        let test_entity = EntityFixtures::basic_entity();
        let saved_entity = entity_repository.save(&test_entity).await.unwrap();
        assert_eq!(saved_entity.name, test_entity.name);
        
        let retrieved_entity = entity_repository.find_by_id(&saved_entity.id).await.unwrap();
        assert!(retrieved_entity.is_some());
        assert_eq!(retrieved_entity.unwrap().id, saved_entity.id);
    }
} 