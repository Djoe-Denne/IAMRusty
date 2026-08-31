use axum::Router;
use hive_configuration::ServerConfig;
use readiness::{attach_ready, ReadinessProbe};
use rustycog::http::{AppState, RouteBuilder};
use rustycog::permission::Permission;
use std::sync::Arc;

pub mod error;
pub mod handlers;
pub mod validation;

pub use error::HttpError;
pub use handlers::*;
pub use validation::{validate_pagination, validate_query_params, ValidatedJson};

pub const SERVICE_PREFIX: &str = "/hive";

/// Create the application routes using the fluent builder API
///
/// All authorization goes through `AppState.permission_checker` (set up in
/// `hive_setup`) which talks to the centralized `OpenFGA` store. Org-scoped
/// routes check `"organization"`. Nested member/role/invitation routes bind
/// `{organization_id}` via `with_permission_on_param` so the last UUID
/// (`user_id`, `role_id`, `invitation_id`) is not used as the object id.
/// `POST /api/invitations/{token}/accept` is authenticated but not org-scoped.
pub fn create_router(state: AppState) -> Router {
    RouteBuilder::new(state)
        .health_check()
        // Search remains public (Public orgs only). GET by id requires membership.
        .get("/api/organizations/search", search_organizations)
        .might_be_authenticated()
        .get("/api/organizations/{organization_id}", get_organization)
        .authenticated()
        .with_permission_on(Permission::Read, "organization")
        // Authenticated organization routes
        .post("/api/organizations", create_organization)
        .authenticated()
        .put("/api/organizations/{organization_id}", update_organization)
        .authenticated()
        .with_permission_on(Permission::Admin, "organization")
        .delete("/api/organizations/{organization_id}", delete_organization)
        .authenticated()
        .with_permission_on(Permission::Admin, "organization")
        .get("/api/organizations", list_organizations)
        .authenticated()
        // Sync job routes
        .post(
            "/api/organizations/{organization_id}/sync-jobs",
            start_sync_job,
        )
        .authenticated()
        .with_permission_on(Permission::Write, "organization")
        // Role routes
        .get("/api/organizations/{organization_id}/roles", list_roles)
        .authenticated()
        .with_permission_on(Permission::Read, "organization")
        .get(
            "/api/organizations/{organization_id}/roles/{role_id}",
            get_role,
        )
        .authenticated()
        .with_permission_on_param(Permission::Read, "organization", "organization_id")
        // Member routes (scoped to the organization in OpenFGA)
        .post("/api/organizations/{organization_id}/members", add_member)
        .authenticated()
        .with_permission_on(Permission::Write, "organization")
        .delete(
            "/api/organizations/{organization_id}/members/{user_id}",
            remove_member,
        )
        .authenticated()
        .with_permission_on_param(Permission::Write, "organization", "organization_id")
        .get("/api/organizations/{organization_id}/members", list_members)
        .authenticated()
        .with_permission_on(Permission::Read, "organization")
        .get(
            "/api/organizations/{organization_id}/members/{user_id}",
            get_member,
        )
        .authenticated()
        .with_permission_on_param(Permission::Read, "organization", "organization_id")
        .patch(
            "/api/organizations/{organization_id}/members/{user_id}",
            update_member,
        )
        .authenticated()
        .with_permission_on_param(Permission::Write, "organization", "organization_id")
        // Invitation routes
        .post(
            "/api/organizations/{organization_id}/invitations",
            create_invitation,
        )
        .authenticated()
        .with_permission_on(Permission::Write, "organization")
        .delete(
            "/api/organizations/{organization_id}/invitations/{invitation_id}",
            cancel_invitation,
        )
        .authenticated()
        .with_permission_on_param(Permission::Write, "organization", "organization_id")
        .post("/api/invitations/{token}/accept", accept_invitation)
        .authenticated()
        // External link routes (admin-only action on the parent organization)
        .post(
            "/api/organizations/{organization_id}/external-links",
            create_external_link,
        )
        .authenticated()
        .with_permission_on(Permission::Admin, "organization")
        .into_router()
}

/// Create the Hive router under its bounded-context prefix.
pub fn create_prefixed_router(state: AppState, probe: Arc<ReadinessProbe>) -> Router {
    Router::new().nest(SERVICE_PREFIX, attach_ready(create_router(state), probe))
}

/// Create and start the application routes using the fluent builder API.
///
/// # Errors
///
/// Returns an error when the HTTP server cannot bind or serve the router.
pub async fn create_app_routes(
    state: AppState,
    config: ServerConfig,
    probe: Arc<ReadinessProbe>,
) -> anyhow::Result<()> {
    rustycog::http::serve_router(create_prefixed_router(state, probe), config).await
}
