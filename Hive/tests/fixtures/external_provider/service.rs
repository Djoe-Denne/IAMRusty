use rustycog_testing::wiremock::MockServerFixture;
use std::sync::Arc;
use wiremock::{matchers::{method, path, body_string_contains}, Mock, MockServer, ResponseTemplate};

use super::resources::*;

pub struct ExternalProviderMockService {
    server: Arc<MockServer>,
    _fixture: MockServerFixture,
}

impl ExternalProviderMockService {
    pub async fn new() -> Self {
        let fixture = MockServerFixture::new().await;
        let server = fixture.server();
        Self { server, _fixture: fixture }
    }

    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    pub async fn mock_validate_config_ok(&self) -> &Self {
        Mock::given(method("POST"))
            .and(path("/config/validate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .mount(&*self.server)
            .await;
        self
    }

    pub async fn mock_validate_config_fail(&self, message_contains: &str) -> &Self {
        Mock::given(method("POST"))
            .and(path("/config/validate"))
            .and(body_string_contains(message_contains))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({"error": message_contains})))
            .mount(&*self.server)
            .await;
        self
    }

    pub async fn mock_connection_test(&self, connected: bool) -> &Self {
        Mock::given(method("POST"))
            .and(path("/connection/test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ConnectionTestResponseBody { connected }))
            .mount(&*self.server)
            .await;
        self
    }

    pub async fn mock_organization_info(&self, name: &str, external_id: &str) -> &Self {
        Mock::given(method("POST"))
            .and(path("/organization/info"))
            .respond_with(ResponseTemplate::new(200).set_body_json(OrganizationInfo { name: name.to_string(), external_id: external_id.to_string() }))
            .mount(&*self.server)
            .await;
        self
    }

    pub async fn mock_members(&self, members: Vec<Member>) -> &Self {
        Mock::given(method("POST"))
            .and(path("/members"))
            .respond_with(ResponseTemplate::new(200).set_body_json(MembersResponse { members }))
            .mount(&*self.server)
            .await;
        self
    }

    pub async fn mock_is_member(&self, is_member: bool) -> &Self {
        Mock::given(method("POST"))
            .and(path("/members/check"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"is_member": is_member})))
            .mount(&*self.server)
            .await;
        self
    }
}


