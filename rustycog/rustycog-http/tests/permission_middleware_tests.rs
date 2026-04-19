use std::sync::Arc;

use axum::extract::{Path, State};
use std::net::SocketAddr;
use rustycog_http::{AppState, AuthUser, RouteBuilder, UserIdExtractor};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rustycog_permission::{Permission, PermissionsFetcher, ResourceId};
use rustycog_core::error::DomainError;
use serde::Serialize;
use uuid::Uuid;

const TEST_JWT_SECRET: &str = "rustycog-test-hs256-secret";

#[derive(Debug, Serialize)]
struct TestClaims {
    sub: String,
    exp: usize,
    iat: usize,
    jti: String,
}

// Dummy handler that expects resource IDs to be already set in request extensions
async fn ok_handler_one_level(
    State(_state): State<AppState>,
    Path(_organization_id): Path<ResourceId>,
    _auth_user: AuthUser,
) -> &'static str {
    "OK"
}

async fn ok_handler_two_level(
    State(_state): State<AppState>,
    Path((_organization_id, _member_id)): Path<(ResourceId, ResourceId)>,
    _auth_user: AuthUser,
) -> &'static str {
    "OK"
}

async fn ok_handler_three_level(
    State(_state): State<AppState>,
    Path((_organization_id, _member_id, _role_id)): Path<(ResourceId, ResourceId, ResourceId)>,
    _auth_user: AuthUser,
) -> &'static str {
    "OK"
}

async fn ok_handler_three_level_with_segment(
    State(_state): State<AppState>,
    Path((_organization_id, _member_id, _resource, _target_id)): Path<(ResourceId, ResourceId, String, ResourceId)>,
    _auth_user: AuthUser,
) -> &'static str {
    "OK"
}

struct MockFetcher {
    rules: std::collections::HashMap<(Option<Uuid>, Vec<ResourceId>), Vec<Permission>>,
}

impl MockFetcher {
    fn new() -> Self { Self { rules: std::collections::HashMap::new() } }
    fn set(&mut self, user: Uuid, resources: Vec<ResourceId>, perms: Vec<Permission>) {
        self.rules.insert((Some(user), resources), perms);
    }
}

#[async_trait::async_trait]
impl PermissionsFetcher for MockFetcher {
    async fn fetch_permissions(&self, user_id: Option<Uuid>, resource_ids: Vec<ResourceId>) -> Result<Vec<Permission>, DomainError> {
        Ok(self.rules.get(&(user_id, resource_ids)).cloned().unwrap_or_default())
    }
}

async fn make_server(fetcher: Arc<dyn PermissionsFetcher>, model: &'static str) -> (SocketAddr, tokio::task::JoinHandle<Result<(), DomainError>>) {
    let registry = Arc::new(rustycog_command::CommandRegistry::default());
    let extractor = UserIdExtractor::from_resolved_secret(TEST_JWT_SECRET).unwrap();
    let state = AppState::new(Arc::new(rustycog_command::GenericCommandService::new(registry)), extractor);
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
        .get("/orgs/{org_id}/members/{member_id}/permissions/{resource}/{target_id}", ok_handler_three_level_with_segment)
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
    let now = chrono::Utc::now();
    let claims = TestClaims {
        sub: user.to_string(),
        exp: (now + chrono::Duration::hours(1)).timestamp() as usize,
        iat: now.timestamp() as usize,
        jti: Uuid::new_v4().to_string(),
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
    )
    .unwrap()
}

fn make_tampered_token_for_user(user: Uuid) -> String {
    let mut token = make_token_for_user(user);
    let last_char = token.pop().expect("token should not be empty");
    token.push(if last_char == 'a' { 'b' } else { 'a' });
    token
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

async fn http_get_with_token(addr: SocketAddr, path: &str, token: &str) -> reqwest::Response {
    let url = format!("http://{}{}", addr, path);
    reqwest::Client::new()
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap()
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

    #[tokio::test]
    async fn reject_tampered_token() {
        let fetcher = Arc::new(MockFetcher::new());
        let (addr, _h) = make_server(fetcher, "resource1").await;
        let user = Uuid::new_v4();
        let tampered_token = make_tampered_token_for_user(user);

        let res = http_get_with_token(
            addr,
            "/orgs/11111111-1111-1111-1111-111111111111",
            &tampered_token,
        )
        .await;

        assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);
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

    #[tokio::test]
    async fn owner_allows_with_non_uuid_path_segment() {
        let user = Uuid::new_v4();
        let a = ResourceId::new_v4();
        let b = ResourceId::new_v4();
        let c = ResourceId::new_v4();
        let mut mf = MockFetcher::new();
        mf.set(user, vec![a.clone(), b.clone(), c.clone()], vec![Permission::Owner]);
        let (addr, _h) = make_server(Arc::new(mf), "resource3").await;
        let res = http_get(
            addr,
            format!("/orgs/{}/members/{}/permissions/component/{}", a.id(), b.id(), c.id()).as_str(),
            Some(user),
        )
        .await;
        assert_eq!(res.status(), reqwest::StatusCode::OK);
    }
}


