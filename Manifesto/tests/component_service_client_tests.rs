//! Focused tests for the component service HTTP client

use manifesto_domain::port::{ComponentInfo, ComponentServicePort};
use manifesto_infra::adapters::ComponentServiceClient;
use rustycog_core::error::DomainError;
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn test_component_service_client_fails_closed_on_non_success_status() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/components"))
        .respond_with(ResponseTemplate::new(502).set_body_string("upstream unavailable"))
        .mount(&server)
        .await;

    let client = ComponentServiceClient::new(server.uri(), None, 1)
        .expect("Failed to create component service client");

    let error = client
        .list_available_components()
        .await
        .expect_err("Non-success upstream response should fail closed");

    match error {
        DomainError::ExternalServiceError { service, message } => {
            assert_eq!(service, "component_service");
            assert!(
                message.contains("HTTP 502"),
                "Error message should include the upstream status"
            );
            assert!(
                message.contains("upstream unavailable"),
                "Error message should include the upstream body"
            );
        }
        other => panic!("Expected external service error, got {other:?}"),
    }
}

#[tokio::test]
async fn test_component_service_client_sends_api_key_and_parses_success_response() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/components"))
        .and(header("authorization", "Bearer test-api-key"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(vec![ComponentInfo {
                component_type: "taskboard".to_string(),
                name: "Taskboard".to_string(),
                description: Some("Collaborative kanban board".to_string()),
                version: "1.2.3".to_string(),
                endpoint: "https://components.example/taskboard".to_string(),
            }]),
        )
        .mount(&server)
        .await;

    let client = ComponentServiceClient::new(server.uri(), Some("test-api-key".to_string()), 1)
        .expect("Failed to create component service client");

    let components = client
        .list_available_components()
        .await
        .expect("Expected successful component response");

    assert_eq!(components.len(), 1);
    assert_eq!(components[0].component_type, "taskboard");
    assert_eq!(components[0].name, "Taskboard");
}
