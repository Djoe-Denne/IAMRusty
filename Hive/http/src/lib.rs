use hive_configuration::ServerConfig;
use rustycog_http::{AppState, RouteBuilder};

pub mod error;
pub mod handlers;
pub mod validation;

pub use error::HttpError;
pub use handlers::*;
pub use validation::{validate_pagination, validate_query_params, ValidatedJson};

/// Create the application routes using the fluent builder API
pub async fn create_app_routes(state: AppState, config: ServerConfig) -> anyhow::Result<()> {
    RouteBuilder::new(state.clone())
        .health_check()
        // Public organization routes
        .get("/api/organizations/search", search_organizations)
        .get("/api/organizations/{organization_id}", get_organization)
        // Authenticated organization routes
        .authenticated_post("/api/organizations", create_organization)
        .authenticated_put("/api/organizations/{organization_id}", update_organization)
        .authenticated_delete("/api/organizations/{organization_id}", delete_organization)
        .authenticated_get("/api/organizations", list_organizations)
        // Member routes
        .authenticated_post("/api/organizations/{organization_id}/members", add_member)
        .authenticated_delete(
            "/api/organizations/{organization_id}/members/{user_id}",
            remove_member,
        )
        .authenticated_get("/api/organizations/{organization_id}/members", list_members)
        .authenticated_get(
            "/api/organizations/{organization_id}/members/{user_id}",
            get_member,
        )
        // Invitation routes
        .authenticated_post(
            "/api/organizations/{organization_id}/invitations",
            create_invitation,
        )
        // External link routes
        .authenticated_post(
            "/api/organizations/{organization_id}/external-links",
            create_external_link,
        )
        // Sync job routes
        .authenticated_post(
            "/api/organizations/{organization_id}/sync-jobs",
            start_sync_job,
        )
        // Role routes
        .authenticated_post("/api/organizations/{organization_id}/roles", create_role)
        .authenticated_get("/api/organizations/{organization_id}/roles", list_roles)
        .authenticated_get(
            "/api/organizations/{organization_id}/roles/{role_id}",
            get_role,
        )
        .authenticated_put(
            "/api/organizations/{organization_id}/roles/{role_id}",
            update_role,
        )
        .authenticated_delete(
            "/api/organizations/{organization_id}/roles/{role_id}",
            delete_role,
        )
        .build(config)
        .await
}
