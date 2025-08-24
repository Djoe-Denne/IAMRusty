use hive_configuration::ServerConfig;
use rustycog_http::{AppState, RouteBuilder};
use rustycog_permission::{Permission, PermissionsFetcher};
use std::sync::Arc;

pub mod error;
pub mod handlers;
pub mod validation;

pub use error::HttpError;
pub use handlers::*;
pub use validation::{validate_pagination, validate_query_params, ValidatedJson};

/// Create the application routes using the fluent builder API
pub async fn create_app_routes(state: AppState, config: ServerConfig, organization_permission_fetcher: Arc<dyn PermissionsFetcher>, member_permission_fetcher: Arc<dyn PermissionsFetcher>, external_link_permission_fetcher: Arc<dyn PermissionsFetcher>) -> anyhow::Result<()> {
    RouteBuilder::new(state.clone())
        .health_check()
        .permissions_dir(std::path::Path::new("resources/permissions").to_path_buf())
        .resource("organization")
        .with_permission_fetcher(organization_permission_fetcher)
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
            .with_permission(Permission::Admin)
        .delete("/api/organizations/{organization_id}", delete_organization)
            .authenticated()
            .with_permission(Permission::Admin)
        .get("/api/organizations", list_organizations)
            .authenticated()
            .with_permission(Permission::Read)        
        // Sync job routes
        .post("/api/organizations/{organization_id}/sync-jobs", start_sync_job)
            .authenticated()
            .with_permission(Permission::Write)
        // Role routes
        .get("/api/organizations/{organization_id}/roles", list_roles)
            .authenticated()
            .with_permission(Permission::Read)
        .get("/api/organizations/{organization_id}/roles/{role_id}", get_role)
            .authenticated()
            .with_permission(Permission::Read)
        // Member routes
        .resource("member")
        .with_permission_fetcher(member_permission_fetcher)
        .post("/api/organizations/{organization_id}/members", add_member)
            .authenticated()
            .with_permission(Permission::Write)
        .delete("/api/organizations/{organization_id}/members/{user_id}", remove_member)
            .authenticated()
            .with_permission(Permission::Write)
        .get("/api/organizations/{organization_id}/members", list_members)
            .authenticated()
            .with_permission(Permission::Read)
        .get("/api/organizations/{organization_id}/members/{user_id}", get_member)
            .authenticated()
            .with_permission(Permission::Read)
        // Invitation routes
        .post("/api/organizations/{organization_id}/invitations", create_invitation)
            .authenticated()
            .with_permission(Permission::Write)
        // External link routes
        .resource("external_link")
        .with_permission_fetcher(external_link_permission_fetcher)
        .post("/api/organizations/{organization_id}/external-links", create_external_link)
            .authenticated()
            .with_permission(Permission::Write)
        .build(config)
        .await
}