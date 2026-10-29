//! Local setup regression tests: real SDK crypto/middleware and Manifesto routes.
//! Only the command boundary is recorded; no DB, FGA, network or runtime is started.

use super::grant_snapshot_user_id_extractor;
use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
    Router,
};
use manifesto_application::{
    BindingGrantSnapshotResponse, CreateProjectCommand, GetBindingGrantSnapshotCommand,
    PrincipalMembershipSnapshot, ProjectErrorMapper, ProjectResponse,
};
use manifesto_configuration::AppConfig;
use rustycog::{
    command::{Command, CommandError, CommandHandler, CommandRegistry, GenericCommandService},
    config::{AuthConfig, JwtAuthConfig},
    http::{AppState, UserIdExtractor},
    permission::InMemoryPermissionChecker,
};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};
use tower::ServiceExt;
use uuid::Uuid;

const ACTOR: Uuid = Uuid::from_u128(0x11111111111141118111111111111111);
const PRINCIPAL: Uuid = Uuid::from_u128(0x22222222222242228222222222222222);
const PROJECT: Uuid = Uuid::from_u128(0x33333333333343338333333333333333);
const COMPONENT: Uuid = Uuid::from_u128(0x44444444444444448444444444444444);
const KEY: &str = "manifesto-service-policy-fixture-only";
const IAM_ISSUER: &str = "https://platform.example/iam";

// Nonsecret, offline signed fixtures. All have the SAME UUID subject and (except
// WRONG_KEY) key. exp=2100-01-01, iat=1; EXPIRED has exp=1. This keeps the test
// independent of minting libraries or wall-clock sleeps, but verifies real HMACs.
const SERVICE: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMTExMTExMS0xMTExLTQxMTEtODExMS0xMTExMTExMTExMTEiLCJpc3MiOiJhaWZvcmFsbC1wbGF0Zm9ybSIsImF1ZCI6Im1hbmlmZXN0by1iaW5kaW5ncyIsImV4cCI6NDEwMjQ0NDgwMCwiaWF0IjoxLCJqdGkiOiJzZXJ2aWNlLXBvbGljeS1maXh0dXJlIn0.Kn0LrW5yJFfbOTCfhj9velJKbYgfSVLFjH-_lwXuprA";
const IAM: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMTExMTExMS0xMTExLTQxMTEtODExMS0xMTExMTExMTExMTEiLCJpc3MiOiJodHRwczovL3BsYXRmb3JtLmV4YW1wbGUvaWFtIiwiYXVkIjoibWFuaWZlc3RvLWJpbmRpbmdzIiwiZXhwIjo0MTAyNDQ0ODAwLCJpYXQiOjEsImp0aSI6InNlcnZpY2UtcG9saWN5LWZpeHR1cmUifQ.5hMVY-Irxk2BKevjoo_5WzSuvSQ5DHFCX7d7M6GyQgU";
const ORG: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMTExMTExMS0xMTExLTQxMTEtODExMS0xMTExMTExMTExMTEiLCJpc3MiOiJodHRwczovL29yZy5leGFtcGxlL2lhbSIsImF1ZCI6Im1hbmlmZXN0by1iaW5kaW5ncyIsImV4cCI6NDEwMjQ0NDgwMCwiaWF0IjoxLCJqdGkiOiJzZXJ2aWNlLXBvbGljeS1maXh0dXJlIn0.Q15YRawZ5ZQxk8GVDHG4PlJkyuLuawbB-q0TgmBADIw";
const OTHER_SERVICE: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMTExMTExMS0xMTExLTQxMTEtODExMS0xMTExMTExMTExMTEiLCJpc3MiOiJhbm90aGVyLXNlcnZpY2UiLCJhdWQiOiJtYW5pZmVzdG8tYmluZGluZ3MiLCJleHAiOjQxMDI0NDQ4MDAsImlhdCI6MSwianRpIjoic2VydmljZS1wb2xpY3ktZml4dHVyZSJ9.mGI8nvuX_TDjqWX4Xt3AV8bpnumuA-PAOMJBER3eJNQ";
const HISTORICAL: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMTExMTExMS0xMTExLTQxMTEtODExMS0xMTExMTExMTExMTEiLCJpc3MiOiJpYW1ydXN0eSIsImF1ZCI6Im1hbmlmZXN0by1iaW5kaW5ncyIsImV4cCI6NDEwMjQ0NDgwMCwiaWF0IjoxLCJqdGkiOiJzZXJ2aWNlLXBvbGljeS1maXh0dXJlIn0.YgiJ9r3bY7ujBgYE-57yY0QgAKvoJ1DefNZ9ajdYF00";
const WRONG_AUD: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMTExMTExMS0xMTExLTQxMTEtODExMS0xMTExMTExMTExMTEiLCJpc3MiOiJhaWZvcmFsbC1wbGF0Zm9ybSIsImF1ZCI6Im90aGVyLWF1ZGllbmNlIiwiZXhwIjo0MTAyNDQ0ODAwLCJpYXQiOjEsImp0aSI6InNlcnZpY2UtcG9saWN5LWZpeHR1cmUifQ.Y3qDYXlQbakAr08DLKH0kK2k21YphjrdvepV3g3INJw";
const WRONG_ALG: &str = "eyJhbGciOiJIUzM4NCIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMTExMTExMS0xMTExLTQxMTEtODExMS0xMTExMTExMTExMTEiLCJpc3MiOiJhaWZvcmFsbC1wbGF0Zm9ybSIsImF1ZCI6Im1hbmlmZXN0by1iaW5kaW5ncyIsImV4cCI6NDEwMjQ0NDgwMCwiaWF0IjoxLCJqdGkiOiJzZXJ2aWNlLXBvbGljeS1maXh0dXJlIn0.nJBKjKktrT_kvwZyLs9oJPNqfuwAcNJZ0IhBc8rJK18arXW4bDQk555kDmCsMBzN";
const WRONG_KEY: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMTExMTExMS0xMTExLTQxMTEtODExMS0xMTExMTExMTExMTEiLCJpc3MiOiJhaWZvcmFsbC1wbGF0Zm9ybSIsImF1ZCI6Im1hbmlmZXN0by1iaW5kaW5ncyIsImV4cCI6NDEwMjQ0NDgwMCwiaWF0IjoxLCJqdGkiOiJzZXJ2aWNlLXBvbGljeS1maXh0dXJlIn0.Gl4UkBHw3zpl3ppj2MDjOn9bJ3ey4Hk7Jk_akEo0df0";
const EXPIRED: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMTExMTExMS0xMTExLTQxMTEtODExMS0xMTExMTExMTExMTEiLCJpc3MiOiJhaWZvcmFsbC1wbGF0Zm9ybSIsImF1ZCI6Im1hbmlmZXN0by1iaW5kaW5ncyIsImV4cCI6MSwiaWF0IjoxLCJqdGkiOiJzZXJ2aWNlLXBvbGljeS1maXh0dXJlIn0._gV7SfJAgtAtcTluZnN1-zeWlocqHYjY1bmWIWsB_mI";

fn config() -> AppConfig {
    let mut config = AppConfig::default();
    config.auth = AuthConfig {
        jwt: JwtAuthConfig {
            hs256_secret: Some(KEY.to_owned()),
            issuer: Some(IAM_ISSUER.to_owned()),
            audience: Some("manifesto-bindings".to_owned()),
            platform_issuer: Some(IAM_ISSUER.to_owned()),
            allow_historical_platform_issuer: Some(false),
            allowed_algorithms: vec!["HS256".to_owned()],
            ..JwtAuthConfig::default()
        },
        ..AuthConfig::default()
    };
    config.apparatus.grant_snapshot_auth.hs256_secret = Some(KEY.to_owned());
    config
        .apparatus
        .grant_snapshot_auth
        .service_principal_issuer = Some("aiforall-platform".to_owned());
    config
}

struct RecordingHandler<C: Command> {
    calls: Arc<Mutex<Vec<C>>>,
    response: fn(&C) -> C::Result,
}

// Spell out the async_trait ABI to avoid adding a dependency just for this probe.
impl<C: Command + 'static> CommandHandler<C> for RecordingHandler<C> {
    fn handle<'life0, 'async_trait>(
        &'life0 self,
        command: C,
    ) -> Pin<Box<dyn Future<Output = Result<C::Result, CommandError>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let response = (self.response)(&command);
            self.calls.lock().unwrap().push(command);
            Ok(response)
        })
    }
}

struct Harness {
    router: Router,
    service: Arc<UserIdExtractor>,
    human: Arc<UserIdExtractor>,
    snapshots: Arc<Mutex<Vec<GetBindingGrantSnapshotCommand>>>,
    accounts: Arc<Mutex<Vec<CreateProjectCommand>>>,
}

fn harness() -> Harness {
    let config = config();
    let service = grant_snapshot_user_id_extractor(&config).unwrap();
    let human = Arc::new(UserIdExtractor::new(config.auth.clone()).unwrap());
    let snapshots = Arc::new(Mutex::new(Vec::new()));
    let accounts = Arc::new(Mutex::new(Vec::new()));
    let mut registry = CommandRegistry::new();
    registry.register::<GetBindingGrantSnapshotCommand, _>(
        "get_binding_grant_snapshot".to_owned(),
        Arc::new(RecordingHandler {
            calls: snapshots.clone(),
            response: |command: &GetBindingGrantSnapshotCommand| BindingGrantSnapshotResponse {
                component_id: command.component_id,
                project_id: command.project_id,
                project_status: "active".to_owned(),
                component_status: "active".to_owned(),
                source: "managed".to_owned(),
                digest: Some("fixture-digest".to_owned()),
                desired_generation: 1,
                observed_generation: 1,
                grant_revision: 7,
                declared: Vec::new(),
                consents: Vec::new(),
                principal: command
                    .principal
                    .map(|user_id| PrincipalMembershipSnapshot {
                        user_id,
                        active: user_id == PRINCIPAL,
                    }),
            },
        }),
        Arc::new(ProjectErrorMapper),
    );
    registry.register::<CreateProjectCommand, _>(
        "create_project".to_owned(),
        Arc::new(RecordingHandler {
            calls: accounts.clone(),
            response: |command: &CreateProjectCommand| ProjectResponse {
                id: PROJECT,
                name: command.request.name.clone(),
                description: None,
                status: "draft".to_owned(),
                owner_type: "personal".to_owned(),
                owner_id: command.user_id,
                created_by: command.user_id,
                visibility: "private".to_owned(),
                created_at: "2026-01-01T00:00:00Z".parse().unwrap(),
                updated_at: "2026-01-01T00:00:00Z".parse().unwrap(),
                published_at: None,
            },
        }),
        Arc::new(ProjectErrorMapper),
    );
    let state = AppState {
        running: true,
        command_service: Arc::new(GenericCommandService::new(Arc::new(registry))),
        user_id_extractor: human.clone(),
        // Neither tested route has an FGA guard. This deny-all checker cannot
        // supply an allow or hide a rejection behind an unrelated FGA failure.
        permission_checker: Arc::new(InMemoryPermissionChecker::new()),
    };
    let router = manifesto_http_server::create_router(
        state,
        service.clone(),
        Arc::new(manifesto_http_server::RejectRuntimeServiceOrigin),
    );
    Harness {
        router,
        service,
        human,
        snapshots,
        accounts,
    }
}

fn snapshot_path(principal: Option<Uuid>) -> String {
    let path = format!("/api/projects/{PROJECT}/bindings/{COMPONENT}");
    principal.map_or(path.clone(), |user_id| {
        format!("{path}?principal={user_id}")
    })
}

async fn request(
    router: &Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    forged_identity: bool,
) -> (StatusCode, String) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    if forged_identity {
        builder = builder
            .header("x-principal-sub", ACTOR.to_string())
            .header("x-principal-iss", "aiforall-platform")
            .header("x-auth-request-user", ACTOR.to_string());
    }
    let mut request = builder
        .body(Body::from(
            r#"{"name":"account-probe","owner_type":"personal"}"#,
        ))
        .unwrap();
    if forged_identity {
        // Negative-only: a stale UUID extension may not replace a bearer proof.
        request.extensions_mut().insert(ACTOR);
    }
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

#[test]
fn grant_snapshot_requires_explicit_service_principal_policy() {
    let mut valid = config();
    // Even the mesh/JWKS global settings must not enter this bearer-only verifier.
    valid.auth.mesh.trusted_gateway_san = "envoy-mesh".to_owned();
    valid.auth.jwt.jwks_url = Some("http://127.0.0.1:1/unused".to_owned());
    valid.auth.jwt.allowed_algorithms = vec!["RS256".to_owned()];
    let service = grant_snapshot_user_id_extractor(&valid).unwrap();
    assert!(service.gateway_san().is_none());
    assert_eq!(
        service.principal_policy().unwrap().platform_issuer,
        "aiforall-platform"
    );
    for issuer in [
        None,
        Some(String::new()),
        Some(" aiforall-platform".to_owned()),
        Some(IAM_ISSUER.to_owned()),
        Some("x".repeat(257)),
    ] {
        let mut invalid = valid.clone();
        invalid
            .apparatus
            .grant_snapshot_auth
            .service_principal_issuer = issuer;
        let error = grant_snapshot_user_id_extractor(&invalid).err().unwrap();
        assert_eq!(
            format!("{error:#}"),
            "Invalid grant snapshot auth configuration"
        );
        assert_eq!(valid.auth.jwt.platform_issuer.as_deref(), Some(IAM_ISSUER));
    }
}

#[tokio::test]
async fn grant_snapshot_service_jwt_preserves_uuid_actor_and_query_principal() {
    let harness = harness();
    assert_ne!(ACTOR, PRINCIPAL);
    assert_eq!(
        harness.service.extract_user_id(SERVICE).await.unwrap(),
        ACTOR
    );
    let policy = harness.service.principal_policy().unwrap();
    assert_eq!(policy.platform_issuer, "aiforall-platform");
    assert!(!policy.allow_historical_platform_issuer);
    assert!(harness.service.gateway_san().is_none());
    let (status, body) = request(
        &harness.router,
        Method::GET,
        &snapshot_path(Some(PRINCIPAL)),
        Some(SERVICE),
        false,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains(&format!(r#""user_id":"{PRINCIPAL}","active":true"#)));
    assert!(!body.contains(&ACTOR.to_string()));
    {
        let calls = harness.snapshots.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].project_id, PROJECT);
        assert_eq!(calls[0].component_id, COMPONENT);
        assert_eq!(calls[0].principal, Some(PRINCIPAL));
    }
    let (status, body) = request(
        &harness.router,
        Method::GET,
        &snapshot_path(None),
        Some(SERVICE),
        false,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains(r#""principal":null"#));
    let calls = harness.snapshots.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].principal, None);
    assert!(harness.accounts.lock().unwrap().is_empty());
}

#[tokio::test]
async fn grant_snapshot_rejects_foreign_authority_before_snapshot_lookup() {
    let harness = harness();
    let path = snapshot_path(Some(PRINCIPAL));
    // Each foreign token has a positive cryptographic control under its own
    // exact authority. A malformed/signature-invalid fixture cannot stand in
    // for the negative authority oracle below.
    for (token, issuer) in [
        (IAM, IAM_ISSUER),
        (ORG, "https://org.example/iam"),
        (OTHER_SERVICE, "another-service"),
        (HISTORICAL, "iamrusty"),
    ] {
        let mut auth = config().auth;
        auth.jwt.issuer = Some(issuer.to_owned());
        auth.jwt.platform_issuer = Some(issuer.to_owned());
        let control = UserIdExtractor::new(auth).unwrap();
        assert_eq!(control.extract_user_id(token).await.unwrap(), ACTOR);
    }
    assert_eq!(
        request(&harness.router, Method::GET, &path, Some(SERVICE), false)
            .await
            .0,
        StatusCode::OK
    );
    for (case, token, forged) in [
        ("IAM same key/sub", Some(IAM), false),
        ("org same key/sub", Some(ORG), false),
        ("other service same key/sub", Some(OTHER_SERVICE), false),
        ("historical issuer same key/sub", Some(HISTORICAL), false),
        ("wrong audience", Some(WRONG_AUD), false),
        ("HS384", Some(WRONG_ALG), false),
        ("wrong key", Some(WRONG_KEY), false),
        ("expired", Some(EXPIRED), false),
        ("invalid bearer", Some("not.a.jwt"), false),
        ("absent bearer", None, false),
        ("UUID and headers without bearer", None, true),
        (
            "UUID and headers cannot repair foreign bearer",
            Some(IAM),
            true,
        ),
    ] {
        let (status, _) = request(&harness.router, Method::GET, &path, token, forged).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{case}");
        assert_eq!(harness.snapshots.lock().unwrap().len(), 1, "{case}");
        assert!(harness.accounts.lock().unwrap().is_empty(), "{case}");
    }
    assert_eq!(
        request(&harness.router, Method::GET, &path, Some(SERVICE), false)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(harness.snapshots.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn grant_snapshot_policy_does_not_leak_to_human_account_routes() {
    let harness = harness();
    assert_eq!(
        harness.human.principal_policy().unwrap().platform_issuer,
        IAM_ISSUER
    );
    assert_eq!(harness.human.extract_user_id(IAM).await.unwrap(), ACTOR);
    // Locate the refusal at the normal verifier, before the HumanAccount command.
    assert!(harness.human.extract_principal(SERVICE).await.is_err());
    let (status, _) = request(
        &harness.router,
        Method::POST,
        "/api/projects",
        Some(SERVICE),
        false,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(harness.accounts.lock().unwrap().is_empty());
    let (status, body) = request(
        &harness.router,
        Method::POST,
        "/api/projects",
        Some(IAM),
        false,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(body.contains(&format!(r#""created_by":"{ACTOR}""#)));
    let calls = harness.accounts.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].user_id, ACTOR);
    assert!(harness.snapshots.lock().unwrap().is_empty());
}
