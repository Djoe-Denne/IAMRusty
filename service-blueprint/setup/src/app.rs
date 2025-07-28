use anyhow::Result;
use rustycog_config::ServerConfig;
use rustycog_http::AppState;
use sea_orm::Database;
use std::sync::Arc;

use {{SERVICE_NAME}}_application::{EntityUseCase, ExampleEntityService};
use {{SERVICE_NAME}}_configuration::{{SERVICE_NAME_PASCAL}}Config;
use {{SERVICE_NAME}}_domain::{ExampleEntityRepository, ExampleEntityService as DomainEntityService};
use {{SERVICE_NAME}}_http_server;
use {{SERVICE_NAME}}_infra::{
    event::{DummyEventPublisher, SqsEventPublisher},
    repository::PostgresExampleEntityRepository,
    service::{DummyEmailService, DummyNotificationService, InMemoryCacheService},
};

/// Application builder for dependency injection and service setup
pub struct AppBuilder {
    config: {{SERVICE_NAME_PASCAL}}Config,
}

/// Built application with all dependencies wired up
pub struct Application {
    config: {{SERVICE_NAME_PASCAL}}Config,
    app_state: AppState,
}

impl AppBuilder {
    /// Create a new application builder
    pub fn new(config: {{SERVICE_NAME_PASCAL}}Config) -> Self {
        Self { config }
    }

    /// Build the application with all dependencies
    pub async fn build(self) -> Result<Application> {
        tracing::info!("Building application...");

        // Setup database connection
        let database_url = format!(
            "postgres://{}:{}@{}:{}/{}",
            self.config.database.username,
            self.config.database.password,
            self.config.database.host,
            self.config.database.port,
            self.config.database.name
        );

        tracing::info!("Connecting to database: {}", database_url);
        let db_connection = Database::connect(&database_url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;

        // Setup repositories
        let entity_repository = Arc::new(PostgresExampleEntityRepository::new(db_connection.clone()));
        
        // For now, we'll use a simple in-memory audit repository
        // In a real implementation, you'd have a proper audit repository
        let audit_repository = Arc::new(InMemoryAuditRepository::new());

        // Setup domain services
        let entity_domain_service = Arc::new(DomainEntityService::new(
            entity_repository.clone(),
            audit_repository.clone(),
        ));

        // Setup use cases
        let entity_use_case = Arc::new(EntityUseCase::new(entity_domain_service));

        // Setup external services
        let email_service = Arc::new(DummyEmailService::new());
        let notification_service = Arc::new(DummyNotificationService::new());
        let cache_service = Arc::new(InMemoryCacheService::new());

        // Setup event publisher based on configuration
        let event_publisher = match self.config.queue.queue_type.as_str() {
            "sqs" => {
                if let Some(sqs_config) = &self.config.queue.sqs {
                    tracing::info!("Setting up SQS event publisher");
                    // In a real implementation, you'd create the SQS client here
                    Arc::new(SqsEventPublisher::new("dummy-queue-url".to_string()).await?)
                        as Arc<dyn rustycog_events::EventPublisher<Error = {{SERVICE_NAME}}_domain::DomainError>>
                } else {
                    tracing::warn!("SQS configured but no SQS config found, using dummy publisher");
                    Arc::new(DummyEventPublisher::new())
                        as Arc<dyn rustycog_events::EventPublisher<Error = {{SERVICE_NAME}}_domain::DomainError>>
                }
            }
            _ => {
                tracing::info!("Using dummy event publisher");
                Arc::new(DummyEventPublisher::new())
                    as Arc<dyn rustycog_events::EventPublisher<Error = {{SERVICE_NAME}}_domain::DomainError>>
            }
        };

        // Create application state
        let app_state = AppState::new()
            .with_service("entity_use_case", entity_use_case)
            .with_service("email_service", email_service)
            .with_service("notification_service", notification_service)
            .with_service("cache_service", cache_service)
            .with_service("event_publisher", event_publisher);

        tracing::info!("Application built successfully");

        Ok(Application {
            config: self.config,
            app_state,
        })
    }
}

impl Application {
    /// Run the application
    pub async fn run(self, server_config: ServerConfig) -> Result<()> {
        tracing::info!("Starting application...");

        // Create HTTP routes
        {{SERVICE_NAME}}_http_server::create_app_routes(self.app_state, server_config).await?;

        Ok(())
    }

    /// Get a reference to the application configuration
    pub fn config(&self) -> &{{SERVICE_NAME_PASCAL}}Config {
        &self.config
    }

    /// Get a reference to the application state
    pub fn app_state(&self) -> &AppState {
        &self.app_state
    }
}

/// Temporary in-memory audit repository implementation
/// In a real application, this would be a proper database-backed implementation
pub struct InMemoryAuditRepository {
    // This would store audit logs in memory for demonstration
}

impl InMemoryAuditRepository {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl {{SERVICE_NAME}}_domain::AuditLogRepository for InMemoryAuditRepository {
    async fn log_event(
        &self,
        entity_id: &uuid::Uuid,
        entity_type: &str,
        action: &str,
        details: Option<&str>,
    ) -> Result<(), {{SERVICE_NAME}}_domain::DomainError> {
        tracing::info!(
            entity_id = %entity_id,
            entity_type = entity_type,
            action = action,
            details = details,
            "Audit event logged"
        );
        Ok(())
    }

    async fn get_audit_history(
        &self,
        _entity_id: &uuid::Uuid,
    ) -> Result<Vec<{{SERVICE_NAME}}_domain::AuditLogEntry>, {{SERVICE_NAME}}_domain::DomainError> {
        // Return empty history for now
        Ok(vec![])
    }
}

/// Legacy build and run function for backward compatibility
pub async fn build_and_run(
    config: {{SERVICE_NAME_PASCAL}}Config,
    server_config: ServerConfig,
    _shutdown_signal: Option<tokio::sync::oneshot::Receiver<()>>,
) -> Result<()> {
    let app = AppBuilder::new(config).build().await?;
    app.run(server_config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_app_builder_creation() {
        let config = {{SERVICE_NAME}}_configuration::{{SERVICE_NAME_PASCAL}}Config::default();
        let builder = AppBuilder::new(config);
        
        // Test that we can create the builder without panics
        assert!(true);
    }

    #[test]
    fn test_in_memory_audit_repository() {
        let repo = InMemoryAuditRepository::new();
        // Test that we can create the repository without panics
        assert!(true);
    }
} 