mod fixtures;

use async_trait::async_trait;
use iam_configuration::{AppConfig, ServerConfig};
use iam_http_server::SERVICE_PREFIX;
use iam_setup::app::build_and_run;
use iammigration::{Migrator, MigratorTrait};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

use fixtures::DbFixtures;
use rustycog_testing::{ServiceTestDescriptor, TestFixture};

const TELEGRAPH_QUEUE: &str = "test-telegraph-events";
const DEFAULT_QUEUE: &str = "test-iam-default-events";

struct IamSqsTestDescriptor;

fn enable_sqs_for_this_test_binary() {
    unsafe {
        std::env::set_var("IAM_QUEUE__ENABLED", "true");
    }
}

#[async_trait]
impl ServiceTestDescriptor<TestFixture> for IamSqsTestDescriptor {
    type Config = AppConfig;

    async fn build_app(
        &self,
        _config: AppConfig,
        _server_config: ServerConfig,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn run_app(&self, config: AppConfig, server_config: ServerConfig) -> anyhow::Result<()> {
        build_and_run(config, server_config, None).await
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
        false
    }
}

async fn setup_sqs_test_server() -> Result<(TestFixture, String, Client), Box<dyn std::error::Error>>
{
    enable_sqs_for_this_test_binary();

    let descriptor = Arc::new(IamSqsTestDescriptor);
    let fixture = TestFixture::new(descriptor.clone()).await?;
    let (server_url, client) =
        rustycog_testing::setup_test_server::<IamSqsTestDescriptor, TestFixture>(descriptor)
            .await?;

    Ok((fixture, format!("{server_url}{SERVICE_PREFIX}"), client))
}

async fn clear_routing_queues(fixture: &TestFixture) {
    let sqs = fixture.sqs();
    let _ = sqs.get_all_messages_from_queue(TELEGRAPH_QUEUE, 1).await;
    let _ = sqs.get_all_messages_from_queue(DEFAULT_QUEUE, 1).await;
}

async fn wait_for_single_event(fixture: &TestFixture, expected_event_type: &str) -> Value {
    let messages = fixture
        .sqs()
        .wait_for_messages_from_queue(TELEGRAPH_QUEUE, 1, 10)
        .await
        .expect("expected event to be published to Telegraph queue");

    let event: Value = serde_json::from_str(&messages[0]).expect("SQS message should be JSON");
    assert_eq!(event["event_type"], expected_event_type);

    let default_messages = fixture
        .sqs()
        .get_all_messages_from_queue(DEFAULT_QUEUE, 1)
        .await
        .expect("default queue should be readable");
    assert!(
        default_messages.is_empty(),
        "mapped IAM event should not be routed through the default queue"
    );

    event
}

fn event_payload(event: &Value) -> &Value {
    &event["data"]["data"]
}

#[tokio::test]
#[serial]
async fn routes_iam_events_to_telegraph_queue() {
    let (fixture, base_url, client) = setup_sqs_test_server()
        .await
        .expect("failed to setup IAM SQS test server");
    clear_routing_queues(&fixture).await;

    let email = format!("sqs-user-{}@example.com", &Uuid::new_v4().to_string()[..8]);
    let signup_response = client
        .post(format!("{base_url}/api/auth/signup"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "email": email,
            "password": "securePassword123"
        }))
        .send()
        .await
        .expect("failed to send signup request");
    assert_eq!(signup_response.status(), StatusCode::ACCEPTED);

    let signup_body: Value = signup_response
        .json()
        .await
        .expect("signup body should be JSON");
    let registration_token = signup_body["registration_token"]
        .as_str()
        .expect("signup should return registration token");

    let username = format!("sqsuser{}", &Uuid::new_v4().to_string()[..8]);
    let completion_response = client
        .post(format!("{base_url}/api/auth/complete-registration"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "registration_token": registration_token,
            "username": username
        }))
        .send()
        .await
        .expect("failed to complete registration");
    assert_eq!(completion_response.status(), StatusCode::OK);

    let event = wait_for_single_event(&fixture, "user_signed_up").await;
    assert_eq!(
        event_payload(&event)["email"].as_str(),
        Some(email.as_str())
    );
    assert_eq!(
        event_payload(&event)["username"].as_str(),
        Some(username.as_str())
    );
    clear_routing_queues(&fixture).await;

    let email = format!("sqs-reset-{}@example.com", &Uuid::new_v4().to_string()[..8]);
    DbFixtures::create_user_with_email_password(
        fixture.db().as_ref(),
        &email,
        "securePassword123",
        Some("sqsresetuser"),
    )
    .await
    .expect("failed to create test user");

    let response = client
        .post(format!("{base_url}/api/auth/password/reset-request"))
        .header("Content-Type", "application/json")
        .json(&json!({ "email": email }))
        .send()
        .await
        .expect("failed to request password reset");
    assert_eq!(response.status(), StatusCode::OK);

    let event = wait_for_single_event(&fixture, "password_reset_requested").await;
    assert_eq!(
        event_payload(&event)["email"].as_str(),
        Some(email.as_str())
    );
}
