use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{header::CONTENT_TYPE, HeaderMap, Request, StatusCode};
use axum::routing::post;
use axum::{Extension, Router};
use iam_http_server::handlers::session_liveness::session_liveness;
use iam_http_server::{configure_internal_service_token, PlatformIssuer};
use rustycog::http::{
    SessionLivenessChecker, SessionLivenessDecision, SessionLivenessError, SessionLivenessOutcome,
    SessionLivenessQuery,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

const TOKEN: &str = "session-liveness-endpoint-test-token";
const ISSUER: &str = "https://platform.example/iam";

struct FixedChecker {
    outcome: SessionLivenessOutcome,
}

#[async_trait]
impl SessionLivenessChecker for FixedChecker {
    async fn check(
        &self,
        _query: &SessionLivenessQuery,
    ) -> Result<SessionLivenessOutcome, SessionLivenessError> {
        Ok(self.outcome)
    }
}

fn app(outcome: SessionLivenessOutcome) -> Router {
    configure_internal_service_token(TOKEN);
    Router::new()
        .route("/internal/auth/session-liveness", post(session_liveness))
        .layer(Extension(PlatformIssuer::new(ISSUER.into()).unwrap()))
        .layer(Extension(
            Arc::new(FixedChecker { outcome }) as Arc<dyn SessionLivenessChecker>
        ))
}

async fn call(app: Router, body: Value, token: Option<&str>) -> (StatusCode, HeaderMap, Value) {
    let mut request = Request::builder()
        .method("POST")
        .uri("/internal/auth/session-liveness")
        .header(CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        request = request.header("x-iam-internal-token", token);
    }
    let response = app
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, headers, value)
}

#[tokio::test]
async fn live_response_is_authoritative_and_not_cacheable() {
    let (status, headers, body) = call(
        app(SessionLivenessOutcome::with_current_epoch(
            SessionLivenessDecision::Live,
            3,
        )),
        json!({
            "issuer": ISSUER,
            "subject": Uuid::new_v4(),
            "auth_epoch": "3",
        }),
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["cache-control"], "no-store");
    assert_eq!(body["decision"], "live");
    assert_eq!(body["current_epoch"], "3");
}

#[tokio::test]
async fn revoked_missing_account_has_no_current_epoch() {
    let (status, headers, body) = call(
        app(SessionLivenessOutcome::new(
            SessionLivenessDecision::Revoked,
        )),
        json!({
            "issuer": ISSUER,
            "subject": Uuid::new_v4(),
            "auth_epoch": "1",
        }),
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["cache-control"], "no-store");
    assert_eq!(body["decision"], "revoked");
    assert!(body.get("current_epoch").is_none());
}

#[tokio::test]
async fn invalid_workload_token_precedes_body_parsing() {
    let (status, _, _) = call(
        app(SessionLivenessOutcome::new(SessionLivenessDecision::Live)),
        json!({}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn null_and_malformed_epochs_are_bad_requests() {
    for epoch in [Value::Null, json!(3), json!("01"), json!("-1")] {
        let (status, _, _) = call(
            app(SessionLivenessOutcome::new(SessionLivenessDecision::Live)),
            json!({
                "issuer": ISSUER,
                "subject": Uuid::new_v4(),
                "auth_epoch": epoch,
            }),
            Some(TOKEN),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn non_platform_query_is_not_applicable_without_epoch_lookup() {
    let (status, headers, body) = call(
        app(SessionLivenessOutcome::new(
            SessionLivenessDecision::NotApplicable,
        )),
        json!({
            "issuer": "https://organization.example/iam",
            "subject": Uuid::new_v4(),
        }),
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["cache-control"], "no-store");
    assert_eq!(body["decision"], "not_applicable");
    assert!(body.get("current_epoch").is_none());
}

#[tokio::test]
async fn inconsistent_platform_not_applicable_is_unavailable() {
    let (status, _, _) = call(
        app(SessionLivenessOutcome::new(
            SessionLivenessDecision::NotApplicable,
        )),
        json!({
            "issuer": ISSUER,
            "subject": Uuid::new_v4(),
        }),
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}
