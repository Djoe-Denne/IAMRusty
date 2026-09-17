//! HTTP routes for Lazaret: health plus workload enroll/session (no IAM middleware).

use std::sync::Arc;

use axum::extract::Extension;
use axum::Router;
use lazaret_application::{IdentityService, InvokeService};
use lazaret_configuration::ServerConfig;
use readiness::{attach_ready, ReadinessProbe};
use rustycog::http::{AppState, RouteBuilder};
use tower::layer::util::Identity;

mod error;
mod handlers;
mod invoke;

/// Bounded-context path for standalone and monolith embedding.
pub const SERVICE_PREFIX: &str = "/lazaret";

/// Unprefixed router (`/health`, `/enroll`, `/session`). Nested under [`SERVICE_PREFIX`] by the monolith.
pub fn create_router(
    state: AppState,
    identity: Arc<IdentityService>,
    invoke: Arc<InvokeService>,
) -> Router {
    RouteBuilder::new(state)
        .health_check()
        .post("/enroll", handlers::enroll)
        .post("/session", handlers::session)
        .post("/invoke", invoke::invoke)
        .into_router()
        .layer(Extension(identity))
        .layer(Extension(invoke))
        // rustycog HTTP does not expose a TLS client certificate. Cleartext
        // requests therefore never carry `VerifiedClientCertificate`; `/session`
        // returns 401 until a terminator or test inserts the extension.
        .layer(Identity::new())
}

/// Nest `inner` under [`SERVICE_PREFIX`] once.
fn nest_under_service_prefix(inner: Router) -> Router {
    Router::new().nest(SERVICE_PREFIX, inner)
}

/// Router nested under [`SERVICE_PREFIX`], with `GET /ready`.
pub fn create_prefixed_router(
    state: AppState,
    probe: Arc<ReadinessProbe>,
    identity: Arc<IdentityService>,
    invoke: Arc<InvokeService>,
) -> Router {
    nest_under_service_prefix(attach_ready(create_router(state, identity, invoke), probe))
}

/// Serve the prefixed router (standalone binary).
///
/// # Errors
///
/// Returns an error when the HTTP server cannot bind or serve.
pub async fn create_app_routes(
    state: AppState,
    config: ServerConfig,
    probe: Arc<ReadinessProbe>,
    identity: Arc<IdentityService>,
    invoke: Arc<InvokeService>,
) -> anyhow::Result<()> {
    rustycog::http::serve_router(
        create_prefixed_router(state, probe, identity, invoke),
        config,
    )
    .await
}

#[cfg(test)]
mod tests {
    use apparatus_contracts::INVOKE_PATH;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::post;
    use axum::Router;
    use tower::ServiceExt;

    use super::{nest_under_service_prefix, SERVICE_PREFIX};

    #[test]
    fn service_prefix_nests_health_and_ready() {
        assert_eq!(SERVICE_PREFIX, "/lazaret");
        assert_eq!(format!("{SERVICE_PREFIX}/health"), "/lazaret/health");
        assert_eq!(format!("{SERVICE_PREFIX}/ready"), "/lazaret/ready");
    }

    #[test]
    fn service_prefix_joins_invoke_path() {
        assert_eq!(format!("{SERVICE_PREFIX}{INVOKE_PATH}"), "/lazaret/invoke");
    }

    async fn post_status(router: &Router, uri: &str) -> StatusCode {
        router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("oneshot")
            .status()
    }

    #[tokio::test]
    async fn nest_under_service_prefix_mounts_invoke_once() {
        let inner = Router::new().route(INVOKE_PATH, post(|| async { StatusCode::NO_CONTENT }));
        let router = nest_under_service_prefix(inner);

        assert_eq!(
            post_status(&router, "/lazaret/invoke").await,
            StatusCode::NO_CONTENT
        );
        assert_eq!(post_status(&router, "/invoke").await, StatusCode::NOT_FOUND);
        assert_eq!(
            post_status(&router, "/lazaret/lazaret/invoke").await,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            post_status(&router, "/lazaret/invoke/").await,
            StatusCode::NOT_FOUND
        );
    }
}
