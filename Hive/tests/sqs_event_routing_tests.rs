use async_trait::async_trait;
use hive_configuration::AppConfig;
use hive_http::SERVICE_PREFIX;
use hive_migration::{Migrator, MigratorTrait};
use hive_setup::app::AppBuilder;
use reqwest::Client;
use reqwest::StatusCode;
use rustycog_config::ServerConfig;
use rustycog_testing::http::jwt::create_jwt_token;
use rustycog_testing::{ServiceTestDescriptor, TestFixture};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

use hive_application::dto::member::{AddMemberRequest, MemberResponse};
use hive_application::dto::organization::{CreateOrganizationRequest, OrganizationResponse};
use hive_application::dto::role::{MemberRole, MemberRolePermission};

mod common;
use common::{fixtures::db::DbFixtures, Permission, ResourceRef, Subject};

const SENTINEL_SYNC_QUEUE: &str = "test-sentinel-sync-events";
const DEFAULT_QUEUE: &str = "test-hive-default-events";

struct HiveSqsTestDescriptor;

fn enable_sqs_for_this_test_binary() {
    unsafe {
        std::env::set_var("HIVE_QUEUE__ENABLED", "true");
    }
}

#[async_trait]
impl ServiceTestDescriptor<TestFixture> for HiveSqsTestDescriptor {
    type Config = AppConfig;

    async fn build_app(
        &self,
        _config: AppConfig,
        _server_config: ServerConfig,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn run_app(&self, config: AppConfig, server_config: ServerConfig) -> anyhow::Result<()> {
        let app = AppBuilder::new(config).build().await?;
        app.run(server_config).await?;
        Ok(())
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

async fn setup_sqs_test_server(
) -> Result<(TestFixture, String, Client, common::TestOpenFga), Box<dyn std::error::Error>> {
    enable_sqs_for_this_test_binary();

    let descriptor = Arc::new(HiveSqsTestDescriptor);
    let fixture = TestFixture::new(descriptor.clone()).await?;
    let openfga = fixture.openfga().clone();
    let (server_url, client) =
        rustycog_testing::setup_test_server::<HiveSqsTestDescriptor, TestFixture>(descriptor)
            .await?;

    Ok((
        fixture,
        format!("{server_url}{SERVICE_PREFIX}"),
        client,
        openfga,
    ))
}

async fn clear_routing_queues(fixture: &TestFixture) {
    let sqs = fixture.sqs();
    let _ = sqs
        .get_all_messages_from_queue(SENTINEL_SYNC_QUEUE, 1)
        .await;
    let _ = sqs.get_all_messages_from_queue(DEFAULT_QUEUE, 1).await;
}

async fn wait_for_single_event(
    fixture: &TestFixture,
    expected_event_type: &str,
) -> serde_json::Value {
    let messages = fixture
        .sqs()
        .wait_for_messages_from_queue(SENTINEL_SYNC_QUEUE, 1, 10)
        .await
        .expect("expected event to be published to SentinelSync queue");

    let event: serde_json::Value =
        serde_json::from_str(&messages[0]).expect("SQS message should be valid JSON");
    assert_eq!(event["event_type"], expected_event_type);

    let default_messages = fixture
        .sqs()
        .get_all_messages_from_queue(DEFAULT_QUEUE, 1)
        .await
        .expect("default queue should be readable");
    assert!(
        default_messages.is_empty(),
        "mapped Hive event should not be routed through the default queue"
    );

    event
}

fn assert_aggregate_id(event: &serde_json::Value, expected_id: Uuid) {
    assert_eq!(
        event["aggregate_id"].as_str().unwrap(),
        expected_id.to_string()
    );
}

fn assert_payload_uuid(event: &serde_json::Value, field: &str, expected_id: Uuid) {
    assert_eq!(
        event_payload(event)[field].as_str().unwrap(),
        expected_id.to_string()
    );
}

fn event_payload(event: &serde_json::Value) -> &serde_json::Value {
    &event["data"]["data"]
}

#[tokio::test]
#[serial]
async fn routes_organization_created_to_sentinel_sync_queue() {
    let (fixture, server_url, client, _openfga) = setup_sqs_test_server().await.unwrap();
    clear_routing_queues(&fixture).await;

    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let create_body = CreateOrganizationRequest {
        name: "SQS Routed Org".to_string(),
        slug: format!("sqs-routed-org-{}", &Uuid::new_v4().to_string()[..8]),
        description: Some("Created through SQS routing test".to_string()),
        avatar_url: None,
    };

    let res = client
        .post(format!("{server_url}/api/organizations"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&create_body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let created: OrganizationResponse = res.json().await.unwrap();

    let event = wait_for_single_event(&fixture, "organization_created").await;
    assert_aggregate_id(&event, created.id);
    assert_payload_uuid(&event, "organization_id", created.id);
    assert_payload_uuid(&event, "owner_user_id", owner_id);
}

#[tokio::test]
#[serial]
async fn routes_organization_updated_to_sentinel_sync_queue() {
    let (fixture, server_url, client, openfga) = setup_sqs_test_server().await.unwrap();
    clear_routing_queues(&fixture).await;

    let owner_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("organization", org.id),
        )
        .await
        .expect("failed to grant organization admin");

    let res = client
        .put(format!("{}/api/organizations/{}", server_url, org.id))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "name": "SQS Routed Org Updated",
            "description": "Updated through SQS routing test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let event = wait_for_single_event(&fixture, "organization_updated").await;
    assert_aggregate_id(&event, org.id);
    assert_payload_uuid(&event, "organization_id", org.id);
    assert_payload_uuid(&event, "updated_by_user_id", owner_id);
}

#[tokio::test]
#[serial]
async fn routes_member_joined_to_sentinel_sync_queue() {
    let (fixture, server_url, client, openfga) = setup_sqs_test_server().await.unwrap();
    clear_routing_queues(&fixture).await;

    let owner_id = Uuid::new_v4();
    let new_user_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let org = DbFixtures::create_org_with_owner(fixture.db().as_ref(), owner_id)
        .await
        .unwrap();

    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Write,
            ResourceRef::new("organization", org.id),
        )
        .await
        .expect("failed to grant organization write");

    let body = AddMemberRequest {
        user_id: new_user_id,
        roles: vec![MemberRole {
            organization_id: org.id,
            resource: "organization".to_string(),
            permissions: MemberRolePermission::Read,
        }],
    };

    let res = client
        .post(format!(
            "{}/api/organizations/{}/members",
            server_url, org.id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let member: MemberResponse = res.json().await.unwrap();

    let event = wait_for_single_event(&fixture, "member_joined").await;
    assert_aggregate_id(&event, org.id);
    assert_payload_uuid(&event, "organization_id", org.id);
    assert_payload_uuid(&event, "user_id", member.user_id);
}

#[tokio::test]
#[serial]
async fn routes_member_removed_to_sentinel_sync_queue() {
    let (fixture, server_url, client, openfga) = setup_sqs_test_server().await.unwrap();
    clear_routing_queues(&fixture).await;

    let owner_id = Uuid::new_v4();
    let removed_user_id = Uuid::new_v4();
    let token = create_jwt_token(owner_id);
    let org = DbFixtures::create_org(
        fixture.db().as_ref(),
        owner_id,
        std::collections::HashMap::from([
            (owner_id.to_string(), "owner".to_string()),
            (removed_user_id.to_string(), "read".to_string()),
        ]),
    )
    .await
    .unwrap();

    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Write,
            ResourceRef::new("organization", removed_user_id),
        )
        .await
        .expect("failed to grant organization write");

    let res = client
        .delete(format!(
            "{}/api/organizations/{}/members/{}",
            server_url, org.id, removed_user_id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let event = wait_for_single_event(&fixture, "member_removed").await;
    assert_aggregate_id(&event, org.id);
    assert_payload_uuid(&event, "organization_id", org.id);
    assert_payload_uuid(&event, "user_id", removed_user_id);
}
