use rustycog_config::ServerConfig;
use rustycog_http::{AppState, RouteBuilder};
use rustycog_permission::{Permission, PermissionsFetcher};
use std::sync::Arc;

pub mod error;
pub mod handlers;

pub use error::HttpError;
pub use handlers::*;

/// Create the application routes using the fluent builder API
pub async fn create_app_routes(
    state: AppState,
    config: ServerConfig,
    project_permission_fetcher: Arc<dyn PermissionsFetcher>,
    member_permission_fetcher: Arc<dyn PermissionsFetcher>,
    component_permission_fetcher: Arc<dyn PermissionsFetcher>,
) -> anyhow::Result<()> {
    RouteBuilder::new(state.clone())
        .health_check()
        .permissions_dir(std::path::Path::new("resources/permissions").to_path_buf())
        // Project routes
        .resource("project")
        .with_permission_fetcher(project_permission_fetcher.clone())
        // Public project routes (might be authenticated for public visibility)
        .get("/api/projects", list_projects)
            .might_be_authenticated()
        .get("/api/projects/{project_id}", get_project)
            .might_be_authenticated()
        .get("/api/projects/{project_id}/details", get_project_detail)
            .might_be_authenticated()
        // Authenticated project routes
        .post("/api/projects", create_project)
            .authenticated()
        .put("/api/projects/{project_id}", update_project)
            .authenticated()
            .with_permission(Permission::Write)
        .delete("/api/projects/{project_id}", delete_project)
            .authenticated()
            .with_permission(Permission::Owner)
        .post("/api/projects/{project_id}/publish", publish_project)
            .authenticated()
            .with_permission(Permission::Admin)
        .post("/api/projects/{project_id}/archive", archive_project)
            .authenticated()
            .with_permission(Permission::Admin)
        // Component routes (nested under projects)
        .get("/api/projects/{project_id}/components", list_components)
            .might_be_authenticated()
            .with_permission(Permission::Read)
        .resource("component")
        .with_permission_fetcher(component_permission_fetcher.clone())
        .get("/api/projects/{project_id}/components/{component_type}", get_component)
            .might_be_authenticated()
            .with_permission(Permission::Read)
        .post("/api/projects/{project_id}/components", add_component)
            .authenticated()
            .with_permission(Permission::Admin)
        .patch("/api/projects/{project_id}/components/{component_type}", update_component_status)
            .authenticated()
            .with_permission(Permission::Admin)
        .delete("/api/projects/{project_id}/components/{component_type}", remove_component)
            .authenticated()
            .with_permission(Permission::Admin)
        // Member routes (nested under projects)
        .resource("member")
        .with_permission_fetcher(member_permission_fetcher.clone())
        .get("/api/projects/{project_id}/members", list_members)
            .authenticated()
            .with_permission(Permission::Read)
        .get("/api/projects/{project_id}/members/{user_id}", get_member)
            .authenticated()
            .with_permission(Permission::Read)
        .post("/api/projects/{project_id}/members", add_member)
            .authenticated()
            .with_permission(Permission::Admin)
        .put("/api/projects/{project_id}/members/{user_id}", update_member)
            .authenticated()
            .with_permission(Permission::Admin)
        .delete("/api/projects/{project_id}/members/{user_id}", remove_member)
            .authenticated()
            .with_permission(Permission::Admin)
        // Permission management routes (nested under members)
        // Note: shared middleware forwards all UUID path params; member permission
        // fetcher authorizes by project scope and ignores target-member/resource IDs.
        // Generic resource permissions (e.g., /permissions/component, /permissions/project)
        .post("/api/projects/{project_id}/members/{user_id}/permissions/{resource}", grant_permission)
            .authenticated()
            .with_permission(Permission::Admin)
        .delete("/api/projects/{project_id}/members/{user_id}/permissions/{resource}", revoke_permission)
            .authenticated()
            .with_permission(Permission::Admin)
        // Specific resource permissions (e.g., /permissions/component/{component_id})
        .post("/api/projects/{project_id}/members/{user_id}/permissions/{resource}/{resource_id}", grant_permission_specific)
            .authenticated()
            .with_permission(Permission::Admin)
        .delete("/api/projects/{project_id}/members/{user_id}/permissions/{resource}/{resource_id}", revoke_permission_specific)
            .authenticated()
            .with_permission(Permission::Admin)
        .build(config)
        .await
}
