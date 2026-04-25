use axum::Router;
use rustycog_config::ServerConfig;
use rustycog_http::{AppState, RouteBuilder};
use rustycog_permission::Permission;

pub mod error;
pub mod handlers;

pub use error::HttpError;
pub use handlers::*;

pub const SERVICE_PREFIX: &str = "/manifesto";

/// Create the application routes using the fluent builder API.
///
/// Every guarded route delegates to the centralized `PermissionChecker`
/// wired into `AppState` (OpenFGA in production). Project-scoped routes use
/// the `"project"` object type; component-scoped routes use `"component"`.
/// Members, permission grants, and archives all collapse to project-level
/// relations.
pub fn create_router(state: AppState) -> Router {
    RouteBuilder::new(state)
        .health_check()
        // Project routes
        .get("/api/projects", list_projects)
        .might_be_authenticated()
        .get("/api/projects/{project_id}", get_project)
        .might_be_authenticated()
        .with_permission_on(Permission::Read, "project")
        .get("/api/projects/{project_id}/details", get_project_detail)
        .might_be_authenticated()
        .with_permission_on(Permission::Read, "project")
        // Authenticated project routes
        .post("/api/projects", create_project)
        .authenticated()
        .put("/api/projects/{project_id}", update_project)
        .authenticated()
        .with_permission_on(Permission::Write, "project")
        .delete("/api/projects/{project_id}", delete_project)
        .authenticated()
        .with_permission_on(Permission::Owner, "project")
        .post("/api/projects/{project_id}/publish", publish_project)
        .authenticated()
        .with_permission_on(Permission::Admin, "project")
        .post("/api/projects/{project_id}/archive", archive_project)
        .authenticated()
        .with_permission_on(Permission::Admin, "project")
        // Component routes — the deepest UUID is the component id only when
        // the route carries `{component_type}` as a string, so only the
        // list/add routes authorize at `"project"`; typed component routes
        // still check `"project"` because Manifesto models component type as
        // a non-UUID segment today.
        .get("/api/projects/{project_id}/components", list_components)
        .might_be_authenticated()
        .with_permission_on(Permission::Read, "project")
        .get(
            "/api/projects/{project_id}/components/{component_type}",
            get_component,
        )
        .might_be_authenticated()
        .with_permission_on(Permission::Read, "project")
        .post("/api/projects/{project_id}/components", add_component)
        .authenticated()
        .with_permission_on(Permission::Admin, "project")
        .patch(
            "/api/projects/{project_id}/components/{component_type}",
            update_component_status,
        )
        .authenticated()
        .with_permission_on(Permission::Admin, "project")
        .delete(
            "/api/projects/{project_id}/components/{component_type}",
            remove_component,
        )
        .authenticated()
        .with_permission_on(Permission::Admin, "project")
        // Member routes (project-scoped)
        .get("/api/projects/{project_id}/members", list_members)
        .authenticated()
        .with_permission_on(Permission::Read, "project")
        .get("/api/projects/{project_id}/members/{user_id}", get_member)
        .authenticated()
        .with_permission_on(Permission::Read, "project")
        .post("/api/projects/{project_id}/members", add_member)
        .authenticated()
        .with_permission_on(Permission::Admin, "project")
        .put(
            "/api/projects/{project_id}/members/{user_id}",
            update_member,
        )
        .authenticated()
        .with_permission_on(Permission::Admin, "project")
        .delete(
            "/api/projects/{project_id}/members/{user_id}",
            remove_member,
        )
        .authenticated()
        .with_permission_on(Permission::Admin, "project")
        // Permission management routes (project-admin only)
        .post(
            "/api/projects/{project_id}/members/{user_id}/permissions/{resource}",
            grant_permission,
        )
        .authenticated()
        .with_permission_on(Permission::Admin, "project")
        .delete(
            "/api/projects/{project_id}/members/{user_id}/permissions/{resource}",
            revoke_permission,
        )
        .authenticated()
        .with_permission_on(Permission::Admin, "project")
        .post(
            "/api/projects/{project_id}/members/{user_id}/permissions/{resource}/{resource_id}",
            grant_permission_specific,
        )
        .authenticated()
        .with_permission_on(Permission::Admin, "project")
        .delete(
            "/api/projects/{project_id}/members/{user_id}/permissions/{resource}/{resource_id}",
            revoke_permission_specific,
        )
        .authenticated()
        .with_permission_on(Permission::Admin, "project")
        .into_router()
}

/// Create the Manifesto router under its bounded-context prefix.
pub fn create_prefixed_router(state: AppState) -> Router {
    Router::new().nest(SERVICE_PREFIX, create_router(state))
}

/// Create and start the application routes using the fluent builder API.
pub async fn create_app_routes(state: AppState, config: ServerConfig) -> anyhow::Result<()> {
    rustycog_http::serve_router(create_prefixed_router(state), config).await
}
