use std::sync::Arc;

use axum::{routing::get, extract::{Path, State}};
use std::net::SocketAddr;
use rustycog_http::{AppState, AuthUser, RouteBuilder, UserIdExtractor};
use base64::Engine as _;
use rustycog_permission::{Permission, PermissionsFetcher, PermissionEngine, ResourceId};
use rustycog_core::error::DomainError;
use tower::ServiceExt;
use uuid::Uuid;

// Dummy handler that expects resource IDs to be already set in request extensions
async fn ok_handler_one_level(
    State(state): State<AppState>,
    Path(organization_id): Path<ResourceId>,
    auth_user: AuthUser,
) -> &'static str {
    "OK"
}

async fn ok_handler_two_level(
    State(state): State<AppState>,
    Path((organization_id, member_id)): Path<(ResourceId, ResourceId)>,
    auth_user: AuthUser,
) -> &'static str {
    "OK"
}

async fn ok_handler_three_level(
    State(state): State<AppState>,
    Path((organization_id, member_id, role_id)): Path<(ResourceId, ResourceId, ResourceId)>,
    auth_user: AuthUser,
) -> &'static str {
    "OK"
}

struct MockFetcher {
    rules: std::collections::HashMap<(Uuid, Vec<ResourceId>), Vec<Permission>>,
}

impl MockFetcher {
    fn new() -> Self { Self { rules: std::collections::HashMap::new() } }
    fn set(&mut self, user: Uuid, resources: Vec<ResourceId>, perms: Vec<Permission>) {
        self.rules.insert((user, resources), perms);
    }
}

#[async_trait::async_trait]
impl PermissionsFetcher for MockFetcher {
    async fn fetch_permissions(&self, user_id: Uuid, resource_ids: Vec<ResourceId>) -> Result<Vec<Permission>, DomainError> {
        Ok(self.rules.get(&(user_id, resource_ids)).cloned().unwrap_or_default())
    }
}

async fn make_server(fetcher: Arc<dyn PermissionsFetcher>, model: &'static str) -> (SocketAddr, tokio::task::JoinHandle<Result<(), DomainError>>) {
    let registry = Arc::new(rustycog_command::CommandRegistry::default());
    let state = AppState::new(Arc::new(rustycog_command::GenericCommandService::new(registry)), UserIdExtractor::new());
    let addr = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap().local_addr().unwrap();

    let handle = tokio::task::spawn(async move {
    RouteBuilder::new(state)
        .permissions_dir(std::path::Path::new("tests/fixtures").to_path_buf())
        .resource(model)
        .with_permission_fetcher(fetcher.clone())
        .get("/orgs/{org_id}", ok_handler_one_level)
        .authenticated()
        .with_permission(Permission::Read)
        .get("/orgs/{org_id}/members/{member_id}", ok_handler_two_level)
        .authenticated()
        .with_permission(Permission::Write)
        .get("/orgs/{org_id}/members/{member_id}/roles/{role_id}", ok_handler_three_level)
        .authenticated()
        .with_permission(Permission::Owner)
        .build(rustycog_config::ServerConfig{
            host: "127.0.0.1".into(), port: addr.port(), tls_enabled: false, tls_port: 0,
            tls_cert_path: Default::default(), tls_key_path: Default::default(),
        }).await
        .map_err(|e| DomainError::internal_error(&format!("Server startup failed: {}", e)))?;
       Ok(())
    });
    
    (addr, handle)
}


fn make_token_for_user(user: Uuid) -> String {
    // Create a minimal JWT-like string with base64 payload matching extractor expectations
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("{}".as_bytes());
    let payload = serde_json::json!({
        "sub": user.to_string(),
        "exp": (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
        "iat": chrono::Utc::now().timestamp(),
        "jti": Uuid::new_v4().to_string(),
    });
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
    format!("{}.{}.sig", header, payload_b64)
}

async fn http_get(addr: SocketAddr, path: &str, user: Option<Uuid>) -> reqwest::Response {
    let url = format!("http://{}{}", addr, path);
    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if let Some(u) = user {
        let token = make_token_for_user(u);
        req = req.header("Authorization", format!("Bearer {}", token));
    }
    req.send().await.unwrap()
}

mod one_level {
    use super::*;

    #[tokio::test]
    async fn unauthorized_without_token() {
        let fetcher = Arc::new(MockFetcher::new());
        let (addr, _h) = make_server(fetcher, "resource1").await;
        let res = http_get(addr, "/orgs/11111111-1111-1111-1111-111111111111", None).await;
        assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn forbid_without_permission() {
        let fetcher = Arc::new(MockFetcher::new());
        let (addr, _h) = make_server(fetcher, "resource1").await;
        let user = Uuid::new_v4();
        let res = http_get(addr, "/orgs/11111111-1111-1111-1111-111111111111", Some(user)).await;
        assert_eq!(res.status(), reqwest::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn allow_with_read_permission() {
        let user = Uuid::new_v4();
        let org = ResourceId::new_v4();
        let mut mf = MockFetcher::new();
        mf.set(user, vec![org.clone()], vec![Permission::Read]);
        let (addr, _h) = make_server(Arc::new(mf), "resource1").await;
        let res = http_get(addr, format!("/orgs/{}", org.id()).as_str(), Some(user)).await;
        assert_eq!(res.status(), reqwest::StatusCode::OK);
    }
}

mod two_level {
    use super::*;

    #[tokio::test]
    async fn allow_write_when_granted() {
        let user = Uuid::new_v4();
        let a = ResourceId::new_v4();
        let b = ResourceId::new_v4();
        let mut mf = MockFetcher::new();
        mf.set(user, vec![a.clone(), b.clone()], vec![Permission::Write]);
        let (addr, _h) = make_server(Arc::new(mf), "resource2").await;
        let res = http_get(addr, format!("/orgs/{}/members/{}", a.id(), b.id()).as_str(), Some(user)).await;
        assert_eq!(res.status(), reqwest::StatusCode::OK);
    }
}

mod three_level {
    use super::*;

    #[tokio::test]
    async fn owner_allows_all() {
        let user = Uuid::new_v4();
        let a = ResourceId::new_v4();
        let b = ResourceId::new_v4();
        let c = ResourceId::new_v4();
        let mut mf = MockFetcher::new();
        mf.set(user, vec![a.clone(), b.clone(), c.clone()], vec![Permission::Owner]);
        let (addr, _h) = make_server(Arc::new(mf), "resource3").await;
        let res = http_get(addr, format!("/orgs/{}/members/{}/roles/{}", a.id(), b.id(), c.id()).as_str(), Some(user)).await;
        assert_eq!(res.status(), reqwest::StatusCode::OK);
    }
}


