//! Wiremock fake for Manifesto `GET /api/projects/{id}/bindings/{id}`.

use std::sync::Arc;

use lazaret_domain::BindingGrantSnapshot;
use rustycog::testing::wiremock::MockServerFixture;
use uuid::Uuid;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use super::resources::BindingGrantSnapshotBody;

/// Wiremock fake for the Manifesto binding grant snapshot GET.
pub struct BindingSnapshotMockService {
    server: Arc<MockServer>,
    _fixture: MockServerFixture,
}

impl BindingSnapshotMockService {
    /// Isolated listener so health tests do not share stubs.
    pub async fn new() -> Self {
        let fixture = MockServerFixture::isolated().await;
        let server = fixture.server();
        Self {
            server,
            _fixture: fixture,
        }
    }

    /// Base URL for [`lazaret_infra::HttpBindingGrantClient`] (`uri()`, no extra prefix).
    #[must_use]
    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    /// Stub GET for an interactive caller (`?principal=`).
    pub async fn mock_get_snapshot(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        principal: Uuid,
        body: BindingGrantSnapshot,
    ) -> &Self {
        let route = format!("/api/projects/{project_id}/bindings/{component_id}");
        Mock::given(method("GET"))
            .and(path(route.as_str()))
            .and(query_param("principal", principal.to_string()))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(BindingGrantSnapshotBody::from(body))
                    .insert_header("content-type", "application/json"),
            )
            .mount(&*self.server)
            .await;
        self
    }

    /// Stub GET for a background caller (no `principal` query).
    pub async fn mock_get_snapshot_background(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        body: BindingGrantSnapshot,
    ) -> &Self {
        let route = format!("/api/projects/{project_id}/bindings/{component_id}");
        Mock::given(method("GET"))
            .and(path(route.as_str()))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(BindingGrantSnapshotBody::from(body))
                    .insert_header("content-type", "application/json"),
            )
            .mount(&*self.server)
            .await;
        self
    }

    /// Wipe mounted stubs.
    pub async fn reset(&self) {
        self._fixture.reset().await;
    }

    /// Stub GET that returns 404 (missing binding).
    pub async fn mock_get_snapshot_not_found(&self, project_id: Uuid, component_id: Uuid) -> &Self {
        let route = format!("/api/projects/{project_id}/bindings/{component_id}");
        Mock::given(method("GET"))
            .and(path(route.as_str()))
            .respond_with(ResponseTemplate::new(404))
            .mount(&*self.server)
            .await;
        self
    }

    /// Requests observed by this isolated server.
    pub async fn received_requests(&self) -> Vec<Request> {
        self.server.received_requests().await.unwrap_or_default()
    }
}
