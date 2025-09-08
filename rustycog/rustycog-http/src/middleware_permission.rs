use std::sync::Arc;

use axum::{body::Body, extract::{Path, State}, http::{Request, StatusCode}, middleware::Next, response::Response, RequestExt};
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

/// Permission-checking middleware
pub async fn permission_middleware(
    Path(resource_ids): Path<Vec<ResourceId>>,
    State(guard): State<Arc<PermissionGuard>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    debug!("permission_middleware");
    // Anonymous users are rejected here; use optional guard for might_be_authenticated
    let user_id = req
        .extensions()
        .get::<Uuid>()
        .copied()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if resource_ids.is_empty() {
        debug!("permission_middleware: no resource_ids found -> FORBIDDEN");
        return Err(StatusCode::FORBIDDEN);
    }

    debug!("permission_middleware: building engine with file: {:?}", guard.model_path);
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
    Path(resource_ids): Path<Vec<ResourceId>>,
    State(guard): State<Arc<PermissionGuard>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let user_id = match req.extensions().get::<Uuid>().copied() {
        Some(id) => id,
        None => {
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    
    if resource_ids.is_empty() {
        return Ok(next.run(req).await);
    }

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

