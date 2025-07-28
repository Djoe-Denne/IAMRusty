use axum::{http::StatusCode, response::Json};
use serde_json::{json, Value};

/// Health check endpoint
pub async fn health_check() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "service": "{{SERVICE_NAME}}-service",
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    )
}

/// Readiness check endpoint
pub async fn readiness_check() -> (StatusCode, Json<Value>) {
    // In a real implementation, you would check:
    // - Database connectivity
    // - External service availability
    // - Queue connectivity
    // etc.
    
    (
        StatusCode::OK,
        Json(json!({
            "status": "ready",
            "service": "{{SERVICE_NAME}}-service",
            "checks": {
                "database": "ok",
                "queue": "ok"
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    )
}

/// Liveness check endpoint
pub async fn liveness_check() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "alive",
            "service": "{{SERVICE_NAME}}-service",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let (status, response) = health_check().await;
        
        assert_eq!(status, StatusCode::OK);
        assert!(response.0.get("status").is_some());
        assert!(response.0.get("service").is_some());
        assert!(response.0.get("version").is_some());
        assert!(response.0.get("timestamp").is_some());
    }

    #[tokio::test]
    async fn test_readiness_check() {
        let (status, response) = readiness_check().await;
        
        assert_eq!(status, StatusCode::OK);
        assert!(response.0.get("status").is_some());
        assert!(response.0.get("checks").is_some());
    }

    #[tokio::test]
    async fn test_liveness_check() {
        let (status, response) = liveness_check().await;
        
        assert_eq!(status, StatusCode::OK);
        assert!(response.0.get("status").is_some());
    }
} 