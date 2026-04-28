#[path = "fixtures/mod.rs"]
mod fixtures;

use async_trait::async_trait;
use manifesto_configuration::ManifestoConfig;
use manifesto_http_server::SERVICE_PREFIX;
use manifesto_infra::ManifestoErrorMapper;
use manifesto_migration::{Migrator, MigratorTrait};
use manifesto_setup::build_and_run;
use reqwest::{Client, StatusCode};
use rustycog_config::{QueueConfig, ServerConfig};
use rustycog_events::create_multi_queue_event_publisher;
use rustycog_permission::{Permission, ResourceRef, Subject};
use rustycog_testing::{ServiceTestDescriptor, TestFixture};
use serde_json::{json, Value};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

use fixtures::DbFixtures;

const SENTINEL_SYNC_QUEUE: &str = "test-sentinel-sync-events";
const DEFAULT_QUEUE: &str = "test-manifesto-default-events";

struct ManifestoSqsTestDescriptor;

fn enable_sqs_for_this_test_binary() {
    unsafe {
        std::env::set_var("MANIFESTO_QUEUE__ENABLED", "true");
    }
}

#[async_trait]
impl ServiceTestDescriptor<TestFixture> for ManifestoSqsTestDescriptor {
    type Config = ManifestoConfig;

    async fn build_app(
        &self,
        _config: ManifestoConfig,
        _server_config: ServerConfig,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn run_app(
        &self,
        config: ManifestoConfig,
        server_config: ServerConfig,
    ) -> anyhow::Result<()> {
        let event_publisher =
            create_multi_queue_event_publisher(&config.queue, None, Arc::new(ManifestoErrorMapper))
                .await?;

        let mut config_without_consumer = config;
        config_without_consumer.queue = QueueConfig::Disabled;

        build_and_run(
            config_without_consumer,
            server_config,
            Some(event_publisher),
        )
        .await
    }

    async fn run_migrations_up(
        &self,
        connection: &sea_orm::DatabaseConnection,
    ) -> anyhow::Result<()> {
        Migrator::up(connection, None).await?;
        Ok(())
    }

    async fn run_migrations_down(
        &self,
        connection: &sea_orm::DatabaseConnection,
    ) -> anyhow::Result<()> {
        Migrator::down(connection, None).await?;
        Ok(())
    }

    fn has_db(&self) -> bool {
        true
    }

    fn has_sqs(&self) -> bool {
        true
    }

    fn has_openfga(&self) -> bool {
        true
    }
}

async fn setup_sqs_test_server() -> Result<(TestFixture, String, Client), Box<dyn std::error::Error>>
{
    enable_sqs_for_this_test_binary();

    let descriptor = Arc::new(ManifestoSqsTestDescriptor);
    let fixture = TestFixture::new(descriptor.clone()).await?;
    let (server_url, client) =
        rustycog_testing::setup_test_server::<ManifestoSqsTestDescriptor, TestFixture>(descriptor)
            .await?;

    Ok((fixture, format!("{server_url}{SERVICE_PREFIX}"), client))
}

fn create_test_jwt_token(user_id: Uuid) -> String {
    rustycog_testing::http::jwt::create_jwt_token(user_id)
}

async fn clear_routing_queues(fixture: &TestFixture) {
    let sqs = fixture.sqs();
    let _ = sqs
        .get_all_messages_from_queue(SENTINEL_SYNC_QUEUE, 1)
        .await;
    let _ = sqs.get_all_messages_from_queue(DEFAULT_QUEUE, 1).await;
}

async fn wait_for_single_event(fixture: &TestFixture, expected_event_type: &str) -> Value {
    let messages = fixture
        .sqs()
        .wait_for_messages_from_queue(SENTINEL_SYNC_QUEUE, 1, 10)
        .await
        .expect("expected event to be published to SentinelSync queue");

    let event: Value = serde_json::from_str(&messages[0]).expect("SQS message should be JSON");
    assert_eq!(event["event_type"], expected_event_type);

    let default_messages = fixture
        .sqs()
        .get_all_messages_from_queue(DEFAULT_QUEUE, 1)
        .await
        .expect("default queue should be readable");
    assert!(
        default_messages.is_empty(),
        "mapped Manifesto event should not be routed through the default queue"
    );

    event
}

fn event_payload(event: &Value) -> &Value {
    &event["data"]["data"]
}

#[tokio::test]
#[serial]
async fn routes_project_created_to_sentinel_sync_queue() {
    let (fixture, base_url, client) = setup_sqs_test_server()
        .await
        .expect("failed to setup Manifesto SQS test server");
    clear_routing_queues(&fixture).await;

    let creator_id = Uuid::new_v4();
    let jwt_token = create_test_jwt_token(creator_id);

    let response = client
        .post(format!("{base_url}/api/projects"))
        .header("Authorization", format!("Bearer {jwt_token}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "name": "SQS Routed Project",
            "description": "Created through SQS routing test",
            "owner_type": "personal",
            "visibility": "private"
        }))
        .send()
        .await
        .expect("failed to create project");
    assert_eq!(response.status(), StatusCode::CREATED);

    let created_project: Value = response
        .json()
        .await
        .expect("project response should be JSON");
    let project_id = created_project["id"]
        .as_str()
        .expect("project response should include id");

    let event = wait_for_single_event(&fixture, "project_created").await;
    let expected_creator_id = creator_id.to_string();
    assert_eq!(event["aggregate_id"].as_str(), Some(project_id));
    assert_eq!(
        event_payload(&event)["project_id"].as_str(),
        Some(project_id)
    );
    assert_eq!(
        event_payload(&event)["created_by"].as_str(),
        Some(expected_creator_id.as_str())
    );
}

#[tokio::test]
#[serial]
async fn routes_member_added_to_sentinel_sync_queue() {
    let (fixture, base_url, client) = setup_sqs_test_server()
        .await
        .expect("failed to setup Manifesto SQS test server");
    clear_routing_queues(&fixture).await;

    let owner_id = Uuid::new_v4();
    let new_member_id = Uuid::new_v4();
    let (project, _owner_member) = DbFixtures::create_project_with_owner(&fixture.db(), owner_id)
        .await
        .expect("failed to create project with owner");

    fixture
        .openfga()
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", project.id()),
        )
        .await
        .expect("failed to grant project admin");

    let jwt_token = create_test_jwt_token(owner_id);
    let response = client
        .post(format!(
            "{}/api/projects/{}/members",
            base_url,
            project.id()
        ))
        .header("Authorization", format!("Bearer {jwt_token}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "user_id": new_member_id.to_string(),
            "permission": "read",
            "resource": "project"
        }))
        .send()
        .await
        .expect("failed to add member");
    assert_eq!(response.status(), StatusCode::CREATED);

    let event = wait_for_single_event(&fixture, "member_added").await;
    let expected_project_id = project.id().to_string();
    let expected_member_id = new_member_id.to_string();
    assert_eq!(
        event["aggregate_id"].as_str(),
        Some(expected_project_id.as_str())
    );
    assert_eq!(
        event_payload(&event)["project_id"].as_str(),
        Some(expected_project_id.as_str())
    );
    assert_eq!(
        event_payload(&event)["user_id"].as_str(),
        Some(expected_member_id.as_str())
    );
}
