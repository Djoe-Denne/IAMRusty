use std::sync::Arc;

use axum::{body::Body, extract::State, http::{Request, StatusCode}, middleware::Next, response::Response};
use rustycog_permission::{Permission, PermissionEngine, PermissionsFetcher, ResourceId};
use uuid::Uuid;
use tracing::{info, debug};

/// Permission middleware settings for a route
#[derive(Clone)]
pub struct PermissionGuard {
    pub required: Permission,
    pub fetcher: Arc<dyn PermissionsFetcher>,
    pub model_path: String,
}

/// Extract resource IDs from route path segments.
///
/// This middleware is service-agnostic: it forwards every URL path segment that can be
/// parsed as a UUID and preserves the original order. Service-specific interpretation
/// of those IDs belongs in each `PermissionsFetcher` implementation.
fn extract_resource_ids(path: &str) -> Vec<ResourceId> {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .filter_map(|s| Uuid::parse_str(s).ok())
        .map(ResourceId::from)
        .collect()
}

/// Permission-checking middleware
pub async fn permission_middleware(
    State(guard): State<Arc<PermissionGuard>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let request_path = req.uri().path().to_owned();
    debug!("permission_middleware: request_path={}", request_path);
    
    // Anonymous users are rejected here; use optional guard for might_be_authenticated
    let user_id = req
        .extensions()
        .get::<Uuid>()
        .copied()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Extract ResourceIds from request path by collecting UUID segments.
    let resource_ids = extract_resource_ids(&request_path);
    
    if resource_ids.is_empty() {
        debug!("permission_middleware: no resource_ids found -> FORBIDDEN");
        return Err(StatusCode::FORBIDDEN);
    }

    debug!("permission_middleware: resource_ids={:?}, building engine with file: {:?}", resource_ids, guard.model_path);
    // Build engine on-demand per request (enforcer is per-request)
    let engine = rustycog_permission::casbin::CasbinPermissionEngine::new(
        guard.model_path.clone(),
        guard.fetcher.clone(),
    )
    .await
    .map_err(|_| StatusCode::FORBIDDEN)?;

    let allowed = engine
        .has_permission(user_id, resource_ids, guard.required.clone(), serde_json::json!({}))
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;

    if !allowed {
        info!("permission_middleware: decision=DENY, for user={} asking for {:?}", user_id, guard.required);
        return Err(StatusCode::FORBIDDEN);
    }

    // Continue
    info!("permission_middleware: decision=ALLOW, for user={} asking for {:?}", user_id, guard.required);
    Ok(next.run(req).await)
}

/// A permission-checking middleware that tolerates anonymous users
pub async fn optional_permission_middleware(
    State(guard): State<Arc<PermissionGuard>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let request_path = req.uri().path().to_owned();
    debug!("optional_permission_middleware: request_path={}", request_path);
    
    let user_id = match req.extensions().get::<Uuid>().copied() {
        Some(id) => id,
        None => {
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    debug!("User ID added to request extensions: {:?}", user_id);
    
    // Extract ResourceIds from request path by collecting UUID segments.
    let resource_ids = extract_resource_ids(&request_path);
    
    if resource_ids.is_empty() {
        return Ok(next.run(req).await);
    }

    debug!("optional_permission_middleware: resource_ids={:?}", resource_ids);

    let engine = rustycog_permission::casbin::CasbinPermissionEngine::new(
        guard.model_path.clone(),
        guard.fetcher.clone(),
    )
    .await
    .map_err(|_| StatusCode::FORBIDDEN)?;

    let allowed = engine
        .has_permission(user_id, resource_ids, guard.required.clone(), serde_json::json!({}))
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;

    if !allowed {
        info!("optional_permission_middleware: decision=DENY, for user={} asking for {:?}", user_id, guard.required);
        return Err(StatusCode::FORBIDDEN);
    }

    info!("optional_permission_middleware: decision=ALLOW, for user={} asking for {:?}", user_id, guard.required);
    Ok(next.run(req).await)
}

