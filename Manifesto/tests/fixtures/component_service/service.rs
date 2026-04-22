//! Wiremock-backed fake of the upstream component-catalog HTTP service.
//!
//! Manifesto's `ComponentServiceClient` (the production
//! `ComponentServicePort` adapter) issues `GET {api_url}/api/components`
//! every time `add_component` runs, so any test that exercises a
//! component-creation route needs this collaborator stubbed. Authoring
//! follows the recipe in `.cursor/skills/creating-wiremock-fixtures/` and
//! mirrors the `ExternalProviderMockService` / `SmtpService` /
//! `OpenFgaMockService` shape already used elsewhere in the repo.

use std::sync::Arc;

use rustycog_testing::wiremock::MockServerFixture;
use serde_json::Value;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use super::resources::ComponentInfoBody;

/// Path the catalog stub is mounted at. Matches the URL the production
/// `ComponentServiceClient::list_available_components` calls.
const CATALOG_PATH: &str = "/api/components";

/// Wiremock fake for the component-catalog service.
///
/// Construct via [`super::ComponentServiceFixtures::service`] (preferred) so
/// the wrapper shape stays consistent with the other in-repo fakes.
///
/// Holds both the [`Arc<MockServer>`] (for mounting stubs) and the
/// [`MockServerFixture`] (kept in `_fixture` so its `Drop` impl runs the
/// post-test reset). Tests that run with `#[serial]` automatically share
/// the singleton wiremock listener at `127.0.0.1:3000`.
pub struct ComponentServiceMockService {
    server: Arc<MockServer>,
    _fixture: MockServerFixture,
}

impl ComponentServiceMockService {
    pub async fn new() -> Self {
        let fixture = MockServerFixture::new().await;
        let server = fixture.server();
        Self {
            server,
            _fixture: fixture,
        }
    }

    /// Base URL of the underlying wiremock server.
    ///
    /// Use as `service.component_service.base_url` when wiring the fake into
    /// a service-under-test by hand.
    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    // ---------------------------------------------------------------------
    // mock_* helpers
    // ---------------------------------------------------------------------

    /// Stub `GET /api/components` to return the supplied catalog.
    pub async fn mock_list_components(&self, components: Vec<ComponentInfoBody>) -> &Self {
        Mock::given(method("GET"))
            .and(path(CATALOG_PATH))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(components)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&*self.server)
            .await;
        self
    }

    /// Mount the default Manifesto test catalog (taskboard + wiki).
    ///
    /// Covers every `component_type` Manifesto's checked-in tests request.
    /// Add to this list when a new test introduces a new component type.
    pub async fn mock_default_catalog(&self) -> &Self {
        self.mock_list_components(vec![
            ComponentInfoBody::new("taskboard"),
            ComponentInfoBody::new("wiki"),
        ])
        .await
    }

    /// Stub `GET /api/components` to return a non-success status with the
    /// supplied body. Used to drive
    /// `DomainError::ExternalServiceError { service: "component_service", .. }`
    /// through the `add_component` use case.
    pub async fn mock_list_error(&self, status: u16, body: Value) -> &Self {
        Mock::given(method("GET"))
            .and(path(CATALOG_PATH))
            .respond_with(
                ResponseTemplate::new(status)
                    .set_body_json(body)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&*self.server)
            .await;
        self
    }

    /// Wipe every previously mounted stub on the shared wiremock server.
    ///
    /// **Caution:** the wiremock server is a process-wide singleton, so
    /// calling `reset()` here also wipes any stubs mounted by
    /// `OpenFgaMockService` or any other sibling fixture in the same
    /// process. Tests that need to keep both alive must remount everything
    /// after the reset.
    pub async fn reset(&self) {
        self._fixture.reset().await;
    }

    // ---------------------------------------------------------------------
    // Inspection
    // ---------------------------------------------------------------------

    /// All requests received by the underlying wiremock server, regardless
    /// of path.
    pub async fn received_requests(&self) -> Vec<Request> {
        self.server.received_requests().await.unwrap_or_default()
    }

    /// All requests posted to this fixture's catalog path.
    pub async fn received_catalog_requests(&self) -> Vec<Request> {
        self.received_requests()
            .await
            .into_iter()
            .filter(|req| req.url.path() == CATALOG_PATH)
            .collect()
    }

    /// Number of `GET /api/components` calls observed.
    pub async fn catalog_call_count(&self) -> usize {
        self.received_catalog_requests().await.len()
    }
}
