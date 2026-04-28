use axum::Router;
use hive_configuration::ServerConfig;
use rustycog_http::{AppState, RouteBuilder};
use rustycog_permission::Permission;

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
/// `hive_setup`) which talks to the centralized `OpenFGA` store. Each guarded
/// route declares the `OpenFGA` object type the deepest UUID path segment maps
/// onto (`"organization"` for every current Hive route — members and external
/// links are modeled as derived relations on the parent organization).
pub fn create_router(state: AppState) -> Router {
    RouteBuilder::new(state)
        .health_check()
        // Public organization routes (with optional auth)
        .get("/api/organizations/search", search_organizations)
        .might_be_authenticated()
        .get("/api/organizations/{organization_id}", get_organization)
        .might_be_authenticated()
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
        .with_permission_on(Permission::Read, "organization")
        // Member routes (scoped to the organization in OpenFGA)
        .post("/api/organizations/{organization_id}/members", add_member)
        .authenticated()
        .with_permission_on(Permission::Write, "organization")
        .delete(
            "/api/organizations/{organization_id}/members/{user_id}",
            remove_member,
        )
        .authenticated()
        .with_permission_on(Permission::Write, "organization")
        .get("/api/organizations/{organization_id}/members", list_members)
        .authenticated()
        .with_permission_on(Permission::Read, "organization")
        .get(
            "/api/organizations/{organization_id}/members/{user_id}",
            get_member,
        )
        .authenticated()
        .with_permission_on(Permission::Read, "organization")
        // Invitation routes
        .post(
            "/api/organizations/{organization_id}/invitations",
            create_invitation,
        )
        .authenticated()
        .with_permission_on(Permission::Write, "organization")
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
pub fn create_prefixed_router(state: AppState) -> Router {
    Router::new().nest(SERVICE_PREFIX, create_router(state))
}

/// Create and start the application routes using the fluent builder API.
pub async fn create_app_routes(state: AppState, config: ServerConfig) -> anyhow::Result<()> {
    rustycog_http::serve_router(create_prefixed_router(state), config).await
}
