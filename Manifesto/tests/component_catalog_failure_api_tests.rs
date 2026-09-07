//! End-to-end failure mapping for the component-catalog collaborator.

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use fixtures::DbFixtures;
use manifesto_infra::repository::entity::project_components;
use reqwest::StatusCode;
use rustycog::permission::{Permission, ResourceRef, Subject};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serial_test::serial;
use uuid::Uuid;

fn jwt(user_id: Uuid) -> String {
    rustycog::testing::http::jwt::create_jwt_token(user_id)
}

#[tokio::test]
#[serial]
async fn unavailable_component_catalog_returns_500_and_leaves_project_unchanged() {
    let (fixture, base_url, client, openfga, components) = setup_test_server().await.unwrap();
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _owner) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .unwrap();
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", project.id()),
        )
        .await
        .unwrap();

    components.reset().await;
    components
        .mock_list_error(500, serde_json::json!({"error": "catalog offline"}))
        .await;

    let response = client
        .post(format!(
            "{base_url}/api/projects/{}/components",
            project.id()
        ))
        .header("Authorization", format!("Bearer {}", jwt(owner_id)))
        .json(&serde_json::json!({"component_type": "taskboard"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(components.catalog_call_count().await >= 1);

    let persisted = project_components::Entity::find()
        .filter(project_components::Column::ProjectId.eq(project.id()))
        .all(&*db)
        .await
        .unwrap();
    assert!(
        persisted.is_empty(),
        "a failed collaborator call must not create a component"
    );
}

#[tokio::test]
#[serial]
async fn missing_component_catalog_endpoint_has_the_same_atomic_http_behavior() {
    let (fixture, base_url, client, openfga, components) = setup_test_server().await.unwrap();
    let db = fixture.db();
    let owner_id = Uuid::new_v4();
    let (project, _owner) = DbFixtures::create_project_with_owner(&db, owner_id)
        .await
        .unwrap();
    openfga
        .allow(
            Subject::new(owner_id),
            Permission::Admin,
            ResourceRef::new("project", project.id()),
        )
        .await
        .unwrap();

    components.reset().await;
    components
        .mock_list_error(404, serde_json::json!({"error": "not found"}))
        .await;

    let response = client
        .post(format!(
            "{base_url}/api/projects/{}/components",
            project.id()
        ))
        .header("Authorization", format!("Bearer {}", jwt(owner_id)))
        .json(&serde_json::json!({"component_type": "wiki"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let persisted = project_components::Entity::find()
        .filter(project_components::Column::ProjectId.eq(project.id()))
        .all(&*db)
        .await
        .unwrap();
    assert!(persisted.is_empty());
}
