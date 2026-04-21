use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Path, State};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rustycog_core::error::DomainError;
use rustycog_http::{AppState, AuthUser, RouteBuilder, UserIdExtractor};
use rustycog_permission::{
    InMemoryPermissionChecker, Permission, PermissionChecker, ResourceId, ResourceRef, Subject,
};
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
    Path((_organization_id, _member_id, _resource, _target_id)): Path<(
        ResourceId,
        ResourceId,
        String,
        ResourceId,
    )>,
    _auth_user: AuthUser,
) -> &'static str {
    "OK"
}

async fn make_server(
    checker: Arc<InMemoryPermissionChecker>,
) -> (
    SocketAddr,
    tokio::task::JoinHandle<Result<(), DomainError>>,
) {
    let registry = Arc::new(rustycog_command::CommandRegistry::default());
    let extractor = UserIdExtractor::from_resolved_secret(TEST_JWT_SECRET).unwrap();
    let state = AppState::new(
        Arc::new(rustycog_command::GenericCommandService::new(registry)),
        extractor,
        checker as Arc<dyn PermissionChecker>,
    );
    let addr = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap()
        .local_addr()
        .unwrap();

    let handle = tokio::task::spawn(async move {
        RouteBuilder::new(state)
            .get("/orgs/{org_id}", ok_handler_one_level)
            .authenticated()
            .with_permission_on(Permission::Read, "organization")
            .get(
                "/orgs/{org_id}/members/{member_id}",
                ok_handler_two_level,
            )
            .authenticated()
            .with_permission_on(Permission::Write, "organization")
            .get(
                "/orgs/{org_id}/members/{member_id}/roles/{role_id}",
                ok_handler_three_level,
            )
            .authenticated()
            .with_permission_on(Permission::Owner, "organization")
            .get(
                "/orgs/{org_id}/members/{member_id}/permissions/{resource}/{target_id}",
                ok_handler_three_level_with_segment,
            )
            .authenticated()
            .with_permission_on(Permission::Owner, "organization")
            .build(rustycog_config::ServerConfig {
                host: "127.0.0.1".into(),
                port: addr.port(),
                tls_enabled: false,
                tls_port: 0,
                tls_cert_path: Default::default(),
                tls_key_path: Default::default(),
            })
            .await
            .map_err(|e| {
                DomainError::internal_error(&format!("Server startup failed: {}", e))
            })?;
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

async fn http_get_with_token(
    addr: SocketAddr,
    path: &str,
    token: &str,
) -> reqwest::Response {
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
        let checker = Arc::new(InMemoryPermissionChecker::new());
        let (addr, _h) = make_server(checker).await;
        let res = http_get(addr, "/orgs/11111111-1111-1111-1111-111111111111", None).await;
        assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn forbid_without_permission() {
        let checker = Arc::new(InMemoryPermissionChecker::new());
        let (addr, _h) = make_server(checker).await;
        let user = Uuid::new_v4();
        let res = http_get(addr, "/orgs/11111111-1111-1111-1111-111111111111", Some(user)).await;
        assert_eq!(res.status(), reqwest::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn allow_with_read_permission() {
        let user = Uuid::new_v4();
        let org = Uuid::new_v4();
        let checker = Arc::new(InMemoryPermissionChecker::new());
        checker.allow(
            Subject::new(user),
            Permission::Read,
            ResourceRef::new("organization", org),
        );
        let (addr, _h) = make_server(checker).await;
        let res = http_get(addr, format!("/orgs/{}", org).as_str(), Some(user)).await;
        assert_eq!(res.status(), reqwest::StatusCode::OK);
    }

    #[tokio::test]
    async fn reject_tampered_token() {
        let checker = Arc::new(InMemoryPermissionChecker::new());
        let (addr, _h) = make_server(checker).await;
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
    async fn allow_write_when_granted_on_deepest_resource() {
        let user = Uuid::new_v4();
        let org = Uuid::new_v4();
        let member = Uuid::new_v4();
        let checker = Arc::new(InMemoryPermissionChecker::new());
        // Middleware scopes the check to the deepest UUID in the path.
        checker.allow(
            Subject::new(user),
            Permission::Write,
            ResourceRef::new("organization", member),
        );
        let (addr, _h) = make_server(checker).await;
        let res = http_get(
            addr,
            format!("/orgs/{}/members/{}", org, member).as_str(),
            Some(user),
        )
        .await;
        assert_eq!(res.status(), reqwest::StatusCode::OK);
    }
}

mod three_level {
    use super::*;

    #[tokio::test]
    async fn owner_allows_on_deepest_resource() {
        let user = Uuid::new_v4();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let checker = Arc::new(InMemoryPermissionChecker::new());
        checker.allow(
            Subject::new(user),
            Permission::Owner,
            ResourceRef::new("organization", c),
        );
        let (addr, _h) = make_server(checker).await;
        let res = http_get(
            addr,
            format!("/orgs/{}/members/{}/roles/{}", a, b, c).as_str(),
            Some(user),
        )
        .await;
        assert_eq!(res.status(), reqwest::StatusCode::OK);
    }

    #[tokio::test]
    async fn owner_allows_with_non_uuid_path_segment() {
        let user = Uuid::new_v4();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let checker = Arc::new(InMemoryPermissionChecker::new());
        checker.allow(
            Subject::new(user),
            Permission::Owner,
            ResourceRef::new("organization", c),
        );
        let (addr, _h) = make_server(checker).await;
        let res = http_get(
            addr,
            format!(
                "/orgs/{}/members/{}/permissions/component/{}",
                a, b, c
            )
            .as_str(),
            Some(user),
        )
        .await;
        assert_eq!(res.status(), reqwest::StatusCode::OK);
    }
}
