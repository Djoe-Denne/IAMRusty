use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use rustycog_permission::{Permission, PermissionChecker, ResourceRef, Subject};
use tracing::{debug, info};
use uuid::Uuid;

/// Permission middleware settings for a route.
///
/// Constructed by `RouteBuilder::with_permission_on`. The middleware takes the
/// deepest UUID path segment of the request, builds a `ResourceRef` of
/// `object_type`, and asks the shared `PermissionChecker` whether the caller
/// is allowed to perform `required`.
#[derive(Clone)]
pub struct PermissionGuard {
    pub required: Permission,
    pub object_type: &'static str,
    pub checker: Arc<dyn PermissionChecker>,
}

/// Pick the deepest UUID-shaped segment from the request path.
///
/// Routes typically embed resource IDs as path parameters (e.g.
/// `/orgs/{org_id}/projects/{project_id}`); the permission question we want to
/// answer is always scoped to the most-specific resource, which is the last
/// UUID in the path.
fn extract_deepest_resource_id(path: &str) -> Option<Uuid> {
    path.split('/')
        .rev()
        .filter(|segment| !segment.is_empty())
        .find_map(|s| Uuid::parse_str(s).ok())
}

/// Permission-checking middleware. Rejects anonymous callers before touching
/// the checker.
pub async fn permission_middleware(
    State(guard): State<Arc<PermissionGuard>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let request_path = req.uri().path().to_owned();
    debug!(path = %request_path, "permission_middleware: entering");

    let user_id = req
        .extensions()
        .get::<Uuid>()
        .copied()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let Some(resource_id) = extract_deepest_resource_id(&request_path) else {
        debug!(path = %request_path, "permission_middleware: no resource UUID in path -> FORBIDDEN");
        return Err(StatusCode::FORBIDDEN);
    };

    let subject = Subject::new(user_id);
    let resource = ResourceRef::new(guard.object_type, resource_id);

    let allowed = guard
        .checker
        .check(subject, guard.required, resource)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "permission_middleware: checker error");
            StatusCode::FORBIDDEN
        })?;

    if !allowed {
        info!(
            user = %user_id,
            permission = %guard.required,
            object_type = guard.object_type,
            object_id = %resource_id,
            "permission_middleware: DENY"
        );
        return Err(StatusCode::FORBIDDEN);
    }

    info!(
        user = %user_id,
        permission = %guard.required,
        object_type = guard.object_type,
        object_id = %resource_id,
        "permission_middleware: ALLOW"
    );
    Ok(next.run(req).await)
}

/// Permission-checking middleware that tolerates anonymous callers.
///
/// If no `Subject` is attached (unauthenticated request), the middleware
/// passes through only when the path has no resource UUID. A path that does
/// carry a resource UUID still requires an explicit allow decision, so
/// anonymous access cannot reach protected resources by accident.
pub async fn optional_permission_middleware(
    State(guard): State<Arc<PermissionGuard>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let request_path = req.uri().path().to_owned();
    debug!(path = %request_path, "optional_permission_middleware: entering");

    let user_id = req.extensions().get::<Uuid>().copied();
    let Some(resource_id) = extract_deepest_resource_id(&request_path) else {
        return Ok(next.run(req).await);
    };

    let Some(user_id) = user_id else {
        debug!("optional_permission_middleware: no subject, resource present -> FORBIDDEN");
        return Err(StatusCode::FORBIDDEN);
    };

    let subject = Subject::new(user_id);
    let resource = ResourceRef::new(guard.object_type, resource_id);

    let allowed = guard
        .checker
        .check(subject, guard.required, resource)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "optional_permission_middleware: checker error");
            StatusCode::FORBIDDEN
        })?;

    if !allowed {
        info!(
            user = %user_id,
            permission = %guard.required,
            object_type = guard.object_type,
            object_id = %resource_id,
            "optional_permission_middleware: DENY"
        );
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(req).await)
}
