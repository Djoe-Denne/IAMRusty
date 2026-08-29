//! Axum helpers for `GET /ready`.

use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};

use crate::probe::ReadinessProbe;

/// Merge `GET /ready` onto an already-built router (typically after `into_router()`).
#[must_use]
pub fn attach_ready(router: Router, probe: Arc<ReadinessProbe>) -> Router {
    router.route(
        "/ready",
        get({
            let probe = Arc::clone(&probe);
            move || {
                let probe = Arc::clone(&probe);
                async move { ready_response(probe).await }
            }
        }),
    )
}

async fn ready_response(probe: Arc<ReadinessProbe>) -> impl IntoResponse {
    let report = probe.report().await;
    let status = if report.status == "ready" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::ComponentStatus;
    use crate::probe::ReadinessProbe;

    #[tokio::test]
    async fn ready_endpoint_is_ok_when_queue_disabled() {
        let probe = Arc::new(ReadinessProbe::new("test").with_publisher(ComponentStatus::Disabled, None));
        let server = axum_test::TestServer::new(attach_ready(Router::new(), probe)).unwrap();
        let response = server.get("/ready").await;
        response.assert_status_ok();
        let body: serde_json::Value = response.json();
        assert_eq!(body["status"], "ready");
        assert_eq!(body["checks"]["queue_publisher"]["status"], "disabled");
    }

    #[tokio::test]
    async fn ready_endpoint_is_unavailable_when_queue_degraded() {
        let probe = Arc::new(ReadinessProbe::new("test").with_publisher(
            crate::classify::ComponentStatus::Degraded {
                expected: crate::classify::QueueKind::Sqs,
                reason: "factory_fallback_noop",
            },
            None,
        ));
        let server = axum_test::TestServer::new(attach_ready(Router::new(), probe)).unwrap();
        let response = server.get("/ready").await;
        response.assert_status(StatusCode::SERVICE_UNAVAILABLE);
        let body: serde_json::Value = response.json();
        assert_eq!(body["status"], "not_ready");
    }
}
