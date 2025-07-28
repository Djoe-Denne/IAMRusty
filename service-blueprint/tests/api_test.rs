mod common;
mod fixtures;

use axum_test::TestServer;
use serde_json::json;
use uuid::Uuid;

use {{SERVICE_NAME}}_application::{CreateEntityRequest, UpdateEntityRequest};
use {{SERVICE_NAME}}_setup::AppBuilder;

/// API integration tests
#[cfg(test)]
mod api_tests {
    use super::*;

    async fn create_test_server() -> TestServer {
        common::init();
        
        let config = common::create_test_config();
        let app = AppBuilder::new(config).build().await.unwrap();
        
        // In a real implementation, you would set up the Axum app here
        // For now, we'll create a simple test server
        TestServer::new(axum::Router::new()).unwrap()
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let server = create_test_server().await;
        
        let response = server.get("/health").await;
        
        response.assert_status_ok();
        response.assert_json(&json!({
            "status": "healthy"
        }));
    }

    #[tokio::test]
    async fn test_create_entity_endpoint() {
        let server = create_test_server().await;
        
        let request_body = CreateEntityRequest {
            name: "API Test Entity".to_string(),
            description: Some("Created via API test".to_string()),
        };
        
        let response = server
            .post("/api/entities")
            .json(&request_body)
            .await;
        
        response.assert_status(201); // Created
        
        let entity: serde_json::Value = response.json();
        assert_eq!(entity["name"], "API Test Entity");
        assert_eq!(entity["description"], "Created via API test");
    }

    #[tokio::test]
    async fn test_get_entity_endpoint() {
        let server = create_test_server().await;
        
        // First create an entity
        let create_request = CreateEntityRequest {
            name: "Get Test Entity".to_string(),
            description: Some("For get endpoint test".to_string()),
        };
        
        let create_response = server
            .post("/api/entities")
            .json(&create_request)
            .await;
        
        let created_entity: serde_json::Value = create_response.json();
        let entity_id = created_entity["id"].as_str().unwrap();
        
        // Now get the entity
        let get_response = server
            .get(&format!("/api/entities/{}", entity_id))
            .await;
        
        get_response.assert_status_ok();
        
        let retrieved_entity: serde_json::Value = get_response.json();
        assert_eq!(retrieved_entity["id"], entity_id);
        assert_eq!(retrieved_entity["name"], "Get Test Entity");
    }

    #[tokio::test]
    async fn test_update_entity_endpoint() {
        let server = create_test_server().await;
        
        // Create an entity
        let create_request = CreateEntityRequest {
            name: "Update Test Entity".to_string(),
            description: Some("Original description".to_string()),
        };
        
        let create_response = server
            .post("/api/entities")
            .json(&create_request)
            .await;
        
        let created_entity: serde_json::Value = create_response.json();
        let entity_id = created_entity["id"].as_str().unwrap();
        
        // Update the entity
        let update_request = UpdateEntityRequest {
            name: Some("Updated Entity Name".to_string()),
            description: Some("Updated description".to_string()),
        };
        
        let update_response = server
            .put(&format!("/api/entities/{}", entity_id))
            .json(&update_request)
            .await;
        
        update_response.assert_status_ok();
        
        let updated_entity: serde_json::Value = update_response.json();
        assert_eq!(updated_entity["name"], "Updated Entity Name");
        assert_eq!(updated_entity["description"], "Updated description");
    }

    #[tokio::test]
    async fn test_delete_entity_endpoint() {
        let server = create_test_server().await;
        
        // Create an entity
        let create_request = CreateEntityRequest {
            name: "Delete Test Entity".to_string(),
            description: Some("Will be deleted".to_string()),
        };
        
        let create_response = server
            .post("/api/entities")
            .json(&create_request)
            .await;
        
        let created_entity: serde_json::Value = create_response.json();
        let entity_id = created_entity["id"].as_str().unwrap();
        
        // First, deactivate the entity (business rule)
        let _deactivate_response = server
            .post(&format!("/api/entities/{}/deactivate", entity_id))
            .await;
        
        // Delete the entity
        let delete_response = server
            .delete(&format!("/api/entities/{}", entity_id))
            .await;
        
        delete_response.assert_status(204); // No Content
        
        // Verify entity is deleted
        let get_response = server
            .get(&format!("/api/entities/{}", entity_id))
            .await;
        
        get_response.assert_status(404); // Not Found
    }

    #[tokio::test]
    async fn test_list_entities_endpoint() {
        let server = create_test_server().await;
        
        // Create multiple entities
        for i in 1..=5 {
            let request = CreateEntityRequest {
                name: format!("List Test Entity {}", i),
                description: Some(format!("Description {}", i)),
            };
            
            server.post("/api/entities").json(&request).await;
        }
        
        // List all entities
        let response = server.get("/api/entities").await;
        response.assert_status_ok();
        
        let entities: serde_json::Value = response.json();
        assert!(entities.is_array());
        assert_eq!(entities.as_array().unwrap().len(), 5);
    }

    #[tokio::test]
    async fn test_search_entities_endpoint() {
        let server = create_test_server().await;
        
        // Create entities with different names
        let names = vec!["Apple Product", "Banana Service", "Cherry Tool"];
        for name in names {
            let request = CreateEntityRequest {
                name: name.to_string(),
                description: Some("Search test entity".to_string()),
            };
            
            server.post("/api/entities").json(&request).await;
        }
        
        // Search for entities containing "a"
        let response = server
            .get("/api/entities/search")
            .add_query_param("q", "a")
            .await;
        
        response.assert_status_ok();
        
        let results: serde_json::Value = response.json();
        assert!(results["entities"].is_array());
        // Should find "Apple Product" and "Banana Service"
        assert!(results["entities"].as_array().unwrap().len() >= 2);
    }

    #[tokio::test]
    async fn test_entity_lifecycle_endpoints() {
        let server = create_test_server().await;
        
        // Create entity
        let create_request = CreateEntityRequest {
            name: "Lifecycle Test Entity".to_string(),
            description: Some("Testing lifecycle".to_string()),
        };
        
        let create_response = server
            .post("/api/entities")
            .json(&create_request)
            .await;
        
        let entity: serde_json::Value = create_response.json();
        let entity_id = entity["id"].as_str().unwrap();
        
        // Activate entity
        let activate_response = server
            .post(&format!("/api/entities/{}/activate", entity_id))
            .await;
        
        activate_response.assert_status_ok();
        let activated_entity: serde_json::Value = activate_response.json();
        assert_eq!(activated_entity["status"], "active");
        
        // Deactivate entity
        let deactivate_response = server
            .post(&format!("/api/entities/{}/deactivate", entity_id))
            .await;
        
        deactivate_response.assert_status_ok();
        let deactivated_entity: serde_json::Value = deactivate_response.json();
        assert_eq!(deactivated_entity["status"], "inactive");
        
        // Archive entity
        let archive_response = server
            .post(&format!("/api/entities/{}/archive", entity_id))
            .await;
        
        archive_response.assert_status_ok();
        let archived_entity: serde_json::Value = archive_response.json();
        assert_eq!(archived_entity["status"], "archived");
    }

    #[tokio::test]
    async fn test_validation_errors() {
        let server = create_test_server().await;
        
        // Try to create entity with empty name
        let invalid_request = CreateEntityRequest {
            name: "".to_string(),
            description: Some("Invalid entity".to_string()),
        };
        
        let response = server
            .post("/api/entities")
            .json(&invalid_request)
            .await;
        
        response.assert_status(400); // Bad Request
        
        let error: serde_json::Value = response.json();
        assert_eq!(error["error"], "validation_error");
        assert!(error["validation_errors"].is_array());
    }

    #[tokio::test]
    async fn test_not_found_error() {
        let server = create_test_server().await;
        
        let non_existent_id = Uuid::new_v4();
        let response = server
            .get(&format!("/api/entities/{}", non_existent_id))
            .await;
        
        response.assert_status(404);
        
        let error: serde_json::Value = response.json();
        assert_eq!(error["error"], "entity_not_found");
    }

    #[tokio::test]
    async fn test_entity_count_endpoint() {
        let server = create_test_server().await;
        
        // Get initial count
        let initial_response = server.get("/api/entities/count").await;
        initial_response.assert_status_ok();
        
        let initial_count: serde_json::Value = initial_response.json();
        let initial_count_value = initial_count["count"].as_i64().unwrap();
        
        // Create some entities
        for i in 1..=3 {
            let request = CreateEntityRequest {
                name: format!("Count Test Entity {}", i),
                description: None,
            };
            
            server.post("/api/entities").json(&request).await;
        }
        
        // Get updated count
        let final_response = server.get("/api/entities/count").await;
        final_response.assert_status_ok();
        
        let final_count: serde_json::Value = final_response.json();
        let final_count_value = final_count["count"].as_i64().unwrap();
        
        assert_eq!(final_count_value, initial_count_value + 3);
    }
} 