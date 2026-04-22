//! Wiremock-backed OpenFGA fake for permission integration tests.
//!
//! Wraps the shared [`crate::wiremock::MockServerFixture`] and exposes
//! `mock_check_*` helpers that mount stubs against the same
//! `POST /stores/{store_id}/check` endpoint that
//! `rustycog_permission::OpenFgaPermissionChecker` calls in production.

use std::sync::Arc;

use rustycog_permission::{OpenFgaClientConfig, Permission, ResourceRef, Subject};
use serde_json::Value;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use super::resources::{CheckRequestBody, CheckResponseBody, CheckTupleKey};
use crate::wiremock::MockServerFixture;

/// Default `store_id` used when callers do not supply one.
///
/// Pinning a deterministic UUID-shaped value keeps stub paths predictable in
/// test logs and lets multiple tests in the same suite reuse the same
/// configuration without colliding on store ids.
const DEFAULT_STORE_ID: &str = "01h0test0store0fixture000openfga";

/// Wiremock fake for the OpenFGA `Check` endpoint.
///
/// Construct via [`crate::permission::OpenFgaFixtures::service`] (preferred)
/// or [`OpenFgaMockService::with_store_id`] when the test needs a specific
/// store id. The fixture pins both the [`Arc<MockServer>`] (for mounting) and
/// the [`MockServerFixture`] (so its `Drop` impl resets all mocks for the
/// next test).
pub struct OpenFgaMockService {
    server: Arc<MockServer>,
    _fixture: MockServerFixture,
    store_id: String,
}

impl OpenFgaMockService {
    /// Create a new fake bound to [`DEFAULT_STORE_ID`].
    pub async fn new() -> Self {
        Self::with_store_id(DEFAULT_STORE_ID).await
    }

    /// Create a new fake whose stubs target `/stores/{store_id}/check`.
    pub async fn with_store_id(store_id: impl Into<String>) -> Self {
        let fixture = MockServerFixture::new().await;
        let server = fixture.server();
        Self {
            server,
            _fixture: fixture,
            store_id: store_id.into(),
        }
    }

    /// Base URL of the underlying wiremock server (without the OpenFGA path).
    ///
    /// Use this as `OpenFgaClientConfig::api_url` when wiring the fake into a
    /// service-under-test by hand.
    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    /// Store id this fixture was constructed with.
    pub fn store_id(&self) -> &str {
        &self.store_id
    }

    /// Path the `Check` stubs are mounted at, including the configured store id.
    pub fn check_path(&self) -> String {
        format!("/stores/{}/check", self.store_id)
    }

    /// Ready-made [`OpenFgaClientConfig`] pointing at this fake.
    ///
    /// Hand it to `OpenFgaPermissionChecker::new` to drive the real production
    /// checker against the wiremock server.
    pub fn client_config(&self) -> OpenFgaClientConfig {
        OpenFgaClientConfig {
            api_url: self.base_url(),
            store_id: self.store_id.clone(),
            authorization_model_id: None,
            api_token: None,
            cache_ttl_seconds: Some(0),
        }
    }

    /// Wipe every previously mounted `Check` stub on the shared wiremock
    /// server.
    ///
    /// Useful for tests that need to flip the fake from "allow this tuple" to
    /// "deny this tuple" partway through the scenario (e.g. a grant ➜ revoke
    /// flow). Mount the fresh per-tuple stubs after this call. The shared
    /// wiremock server matches mocks in registration order (first-match
    /// wins), so a `reset()` is the cleanest way to override stubs that were
    /// arranged by `setup_test_server` or earlier in the test.
    pub async fn reset(&self) {
        self._fixture.reset().await;
    }

    // ---------------------------------------------------------------------
    // mock_* helpers
    // ---------------------------------------------------------------------

    /// Stub `Check` to return `{"allowed": true}` for the exact tuple
    /// `(subject, action, resource)`.
    pub async fn mock_check_allow(
        &self,
        subject: Subject,
        action: Permission,
        resource: ResourceRef,
    ) -> &Self {
        self.mount_tuple_match(subject, action, resource, CheckResponseBody::allow(), 200)
            .await;
        self
    }

    /// Stub `Check` to return `{"allowed": false}` for the exact tuple
    /// `(subject, action, resource)`.
    pub async fn mock_check_deny(
        &self,
        subject: Subject,
        action: Permission,
        resource: ResourceRef,
    ) -> &Self {
        self.mount_tuple_match(subject, action, resource, CheckResponseBody::deny(), 200)
            .await;
        self
    }

    /// Stub every `Check` call against the configured store with
    /// `{"allowed": <allow>}`. Useful as a permissive or restrictive default
    /// before adding more specific tuple stubs on top.
    pub async fn mock_check_any(&self, allow: bool) -> &Self {
        let body = if allow {
            CheckResponseBody::allow()
        } else {
            CheckResponseBody::deny()
        };
        Mock::given(method("POST"))
            .and(path(self.check_path()))
            .respond_with(json_response(200, body))
            .mount(&*self.server)
            .await;
        self
    }

    /// Stub `Check` to return a non-success status with the given body. Use
    /// to drive the `OpenFGA Check returned <status>` error path through
    /// `OpenFgaPermissionChecker`.
    pub async fn mock_check_error(&self, status: u16, body: Value) -> &Self {
        Mock::given(method("POST"))
            .and(path(self.check_path()))
            .respond_with(
                ResponseTemplate::new(status)
                    .set_body_json(body)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&*self.server)
            .await;
        self
    }

    /// Stub `Check` only when the request carries the matching
    /// `Authorization: Bearer <token>` header. Pair with
    /// [`mock_check_any`] or [`mock_check_allow`] when verifying that the
    /// production checker forwards `OpenFgaClientConfig::api_token`.
    pub async fn mock_check_requires_bearer(&self, token: &str, allow: bool) -> &Self {
        let body = if allow {
            CheckResponseBody::allow()
        } else {
            CheckResponseBody::deny()
        };
        Mock::given(method("POST"))
            .and(path(self.check_path()))
            .and(header("authorization", format!("Bearer {token}").as_str()))
            .respond_with(json_response(200, body))
            .mount(&*self.server)
            .await;
        self
    }

    // ---------------------------------------------------------------------
    // Inspection
    // ---------------------------------------------------------------------

    /// All requests received by the underlying wiremock server, regardless of
    /// path. Use [`Self::received_check_requests`] for the OpenFGA-only view.
    pub async fn received_requests(&self) -> Vec<Request> {
        self.server.received_requests().await.unwrap_or_default()
    }

    /// All requests posted to this fixture's `Check` path.
    pub async fn received_check_requests(&self) -> Vec<Request> {
        let target = self.check_path();
        self.received_requests()
            .await
            .into_iter()
            .filter(|req| req.url.path() == target)
            .collect()
    }

    /// Number of `Check` calls observed.
    pub async fn check_count(&self) -> usize {
        self.received_check_requests().await.len()
    }

    /// `true` when at least one observed `Check` request matches the given
    /// `(subject, action, resource)` tuple.
    pub async fn verify_check_called(
        &self,
        subject: Subject,
        action: Permission,
        resource: ResourceRef,
    ) -> bool {
        let expected = CheckTupleKey::from_subject_action_resource(subject, action, resource);
        self.received_check_requests().await.into_iter().any(|req| {
            serde_json::from_slice::<CheckRequestBody>(&req.body)
                .map(|body| {
                    body.tuple_key.user == expected.user
                        && body.tuple_key.relation == expected.relation
                        && body.tuple_key.object == expected.object
                })
                .unwrap_or(false)
        })
    }

    // ---------------------------------------------------------------------
    // Internal helpers
    // ---------------------------------------------------------------------

    async fn mount_tuple_match(
        &self,
        subject: Subject,
        action: Permission,
        resource: ResourceRef,
        body: CheckResponseBody,
        status: u16,
    ) {
        let tuple = CheckTupleKey::from_subject_action_resource(subject, action, resource);
        let body_matcher = serde_json::json!({
            "tuple_key": {
                "user": tuple.user,
                "relation": tuple.relation,
                "object": tuple.object,
            }
        });
        Mock::given(method("POST"))
            .and(path(self.check_path()))
            .and(body_json(body_matcher))
            .respond_with(json_response(status, body))
            .mount(&*self.server)
            .await;
    }
}

fn json_response(status: u16, body: CheckResponseBody) -> ResponseTemplate {
    ResponseTemplate::new(status)
        .set_body_json(body)
        .insert_header("content-type", "application/json")
}
