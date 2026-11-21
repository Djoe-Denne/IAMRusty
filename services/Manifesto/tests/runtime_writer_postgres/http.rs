//! In-process HTTP contract through the real command service and PostgreSQL
//! writer. Fixed origin verdicts stand in for the transport verifier only.

use super::{arrange_effect, caller, cas, counter, lease, observation, registration, seed_binding};
use apparatus_contracts::runtime::{
    RuntimeBindingPageV1, RuntimeBindingSnapshotV1, RuntimeCompleteEffectRequestV1,
    RuntimeControlCallerV1, RuntimeControlErrorV1, RuntimeEffectCompletionStateV1,
    RuntimeEffectReservationV1, RuntimeErrorCodeV1, RuntimeInstanceRevokeRequestV1,
    RuntimeInstanceStateV1, RuntimeServiceRoleV1, RuntimeWriteAckV1, RuntimeWriterLeaseRequestV1,
    RuntimeWriterLeaseV1, MAX_RUNTIME_BODY_BYTES,
};
use axum::{
    body::{to_bytes, Body},
    http::{header::CONTENT_TYPE, Method, Request, StatusCode},
    Router,
};
use manifesto_application::{
    RuntimeControlCommand, RuntimeControlCommandErrorMapper, RuntimeControlCommandHandler,
    RuntimeControlUseCase,
};
use manifesto_http_server::{create_router, RuntimeServiceOriginAuthenticator, SERVICE_PREFIX};
use manifesto_infra::apparatus_runtime::{
    runtime_control::SqlRuntimeControl, runtime_cursor::RuntimeCursorKey,
};
use rustycog::{
    command::{CommandRegistry, GenericCommandService},
    config::{AuthConfig, JwtAuthConfig},
    http::{AppState, PeerClientCertificate, UserIdExtractor},
    permission::InMemoryPermissionChecker,
};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serde::{de::DeserializeOwned, Serialize};
use serial_test::serial;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

const BINDINGS: &str = "/internal/apparatus/runtime/bindings";
const PROJECTOR: RuntimeServiceRoleV1 = RuntimeServiceRoleV1::Projector;
const LAZARET: RuntimeServiceRoleV1 = RuntimeServiceRoleV1::Lazaret;

struct FixtureOrigin(RuntimeServiceRoleV1);

impl RuntimeServiceOriginAuthenticator for FixtureOrigin {
    fn authenticate(
        &self,
        _peer: Option<&PeerClientCertificate>,
    ) -> Result<RuntimeControlCallerV1, RuntimeControlErrorV1> {
        Ok(caller(self.0))
    }
}

fn router(writer: Arc<SqlRuntimeControl>, role: RuntimeServiceRoleV1) -> Router {
    let mut registry = CommandRegistry::new();
    registry.register::<RuntimeControlCommand, _>(
        "apparatus_runtime_control_v1".to_owned(),
        Arc::new(RuntimeControlCommandHandler::new(Arc::new(
            RuntimeControlUseCase::new(writer),
        ))),
        Arc::new(RuntimeControlCommandErrorMapper),
    );
    let auth = Arc::new(
        UserIdExtractor::new(AuthConfig {
            jwt: JwtAuthConfig {
                hs256_secret: Some(Uuid::new_v4().to_string()),
                issuer: Some("runtime-fixture".to_owned()),
                audience: Some("runtime-fixture".to_owned()),
                platform_issuer: Some("runtime-fixture".to_owned()),
                allow_historical_platform_issuer: Some(false),
                allowed_algorithms: vec!["HS256".to_owned()],
                ..JwtAuthConfig::default()
            },
            ..AuthConfig::default()
        })
        .expect("unused project route verifier"),
    );
    let state = AppState {
        running: true,
        command_service: Arc::new(GenericCommandService::new(Arc::new(registry))),
        user_id_extractor: auth.clone(),
        permission_checker: Arc::new(InMemoryPermissionChecker::new()),
    };
    Router::new().nest(
        SERVICE_PREFIX,
        create_router(state, auth, Arc::new(FixtureOrigin(role))),
    )
}

#[derive(Clone, Copy)]
struct Binding {
    project: Uuid,
    id: Uuid,
}

impl Binding {
    fn path(self, suffix: &str) -> String {
        format!("{BINDINGS}/{}{suffix}?project_id={}", self.id, self.project)
    }

    fn snapshot_path(self, selector: &str) -> String {
        format!("{}&schema_version=1&{selector}", self.path(""))
    }
}

fn json<T: Serialize>(value: &T) -> Body {
    Body::from(serde_json::to_vec(value).expect("runtime request JSON"))
}

async fn response<T: DeserializeOwned>(
    router: &Router,
    method: Method,
    path: &str,
    body: Body,
    expected_status: StatusCode,
) -> T {
    let request = Request::builder()
        .method(method.clone())
        .uri(format!("{SERVICE_PREFIX}{path}"))
        .header(CONTENT_TYPE, "application/json")
        .body(body)
        .expect("request");
    let response = router
        .clone()
        .oneshot(request)
        .await
        .expect("HTTP response");
    assert_eq!(response.status(), expected_status, "{method} {path}");
    assert_eq!(response.headers()[CONTENT_TYPE], "application/json");
    let bytes = to_bytes(response.into_body(), MAX_RUNTIME_BODY_BYTES)
        .await
        .expect("bounded runtime reply");
    serde_json::from_slice(&bytes).expect("runtime response contract")
}

async fn refused(
    router: &Router,
    method: Method,
    path: &str,
    body: Body,
    status: StatusCode,
    code: RuntimeErrorCodeV1,
) {
    let error: RuntimeControlErrorV1 = response(router, method, path, body, status).await;
    assert_eq!(error.schema_version, 1);
    assert_eq!(error.code, code, "{path}");
}

#[tokio::test]
#[serial]
async fn runtime_http_pages_and_selects_committed_bindings() {
    let (fixture, _, _, _, _) = super::common::setup_test_server()
        .await
        .expect("real server");
    let db = fixture.db();
    let binding = Binding {
        project: Uuid::new_v4(),
        id: Uuid::new_v4(),
    };
    let mut ids = [binding.id, Uuid::new_v4()];
    ids.sort_unstable();
    for id in ids {
        seed_binding(&db, binding.project, id, "active").await;
    }
    let key = Arc::new(
        RuntimeCursorKey::new(format!("rcl-v1-{}", Uuid::new_v4()), rand::random())
            .expect("ephemeral cursor key"),
    );
    let writer = Arc::new(SqlRuntimeControl::new(db.clone()).with_cursor_key(key));
    let acquired = lease(writer.as_ref(), binding.project, binding.id, cas()).await;
    let instance = registration(&db, &acquired, cas()).await;
    writer
        .register_instance(
            &caller(PROJECTOR),
            binding.project,
            binding.id,
            instance.instance_id,
            &instance,
        )
        .await
        .expect("arrange committed runtime instance");
    let http = router(writer, PROJECTOR);
    let list = format!(
        "{BINDINGS}?schema_version=1&project_id={}&page_size=1",
        binding.project
    );
    let first: RuntimeBindingPageV1 =
        response(&http, Method::GET, &list, Body::empty(), StatusCode::OK).await;
    assert_eq!(first.items.len(), 1);
    assert_eq!(first.items[0].binding_id, ids[0]);
    assert!(first.items[0].instances.is_none());
    let cursor = first.next_cursor.expect("second binding cursor");
    let second: RuntimeBindingPageV1 = response(
        &http,
        Method::GET,
        &format!("{list}&cursor={}", cursor.0),
        Body::empty(),
        StatusCode::OK,
    )
    .await;
    assert_eq!(second.items.len(), 1);
    assert_eq!(second.items[0].binding_id, ids[1]);
    assert_eq!(second.next_cursor, None);

    for selector in [
        "instances_mode=page&instances_page_size=1".to_owned(),
        format!("instances_mode=by_pod_uid&pod_uid={}", instance.pod_uid),
        format!(
            "instances_mode=by_instance_id&instance_id={}",
            instance.instance_id
        ),
    ] {
        let snapshot: RuntimeBindingSnapshotV1 = response(
            &http,
            Method::GET,
            &binding.snapshot_path(&selector),
            Body::empty(),
            StatusCode::OK,
        )
        .await;
        assert_eq!(snapshot.binding_id, binding.id);
        let items = snapshot.instances.expect("explicit selector").items;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].instance_id, instance.instance_id);
        assert_eq!(items[0].state, RuntimeInstanceStateV1::Current);
    }
    assert_invalid_reads(&http, binding, &list).await;
}

async fn assert_invalid_reads(http: &Router, binding: Binding, list: &str) {
    use RuntimeErrorCodeV1::{
        RuntimeBindingNotFound, RuntimeInvalidRequest, RuntimeUnsupportedVersion,
    };
    let no_page = Binding {
        id: Uuid::new_v4(),
        ..binding
    };
    for (path, status, code) in [
        (
            list.replace("schema_version=1", "schema_version=2"),
            StatusCode::BAD_REQUEST,
            RuntimeUnsupportedVersion,
        ),
        (
            format!("{list}&unknown=1"),
            StatusCode::BAD_REQUEST,
            RuntimeInvalidRequest,
        ),
        (
            binding.snapshot_path("instances_mode=by_pod_uid"),
            StatusCode::BAD_REQUEST,
            RuntimeInvalidRequest,
        ),
        (
            binding.snapshot_path("instances_mode=by_instance_id"),
            StatusCode::BAD_REQUEST,
            RuntimeInvalidRequest,
        ),
        (
            binding.snapshot_path("instances_mode=page&pod_uid=unexpected"),
            StatusCode::BAD_REQUEST,
            RuntimeInvalidRequest,
        ),
        (
            binding
                .snapshot_path("instances_mode=page")
                .replace(&binding.id.to_string(), "not-a-uuid"),
            StatusCode::BAD_REQUEST,
            RuntimeInvalidRequest,
        ),
        (
            no_page.snapshot_path("instances_mode=page"),
            StatusCode::NOT_FOUND,
            RuntimeBindingNotFound,
        ),
    ] {
        refused(http, Method::GET, &path, Body::empty(), status, code).await;
    }
}

async fn acquire_over_http(
    http: &Router,
    denied: &Router,
    binding: Binding,
) -> RuntimeWriterLeaseV1 {
    let path = binding.path("/writer-lease");
    let request = RuntimeWriterLeaseRequestV1 {
        schema_version: 1,
        holder_id: Uuid::new_v4(),
        expected: cas(),
        renew: None,
    };
    refused(
        denied,
        Method::POST,
        &path,
        json(&request),
        StatusCode::FORBIDDEN,
        RuntimeErrorCodeV1::RuntimeServiceForbidden,
    )
    .await;
    refused(
        http,
        Method::POST,
        &path,
        Body::from("{"),
        StatusCode::BAD_REQUEST,
        RuntimeErrorCodeV1::RuntimeInvalidRequest,
    )
    .await;
    refused(
        http,
        Method::POST,
        &path,
        Body::from(vec![b' '; MAX_RUNTIME_BODY_BYTES + 1]),
        StatusCode::PAYLOAD_TOO_LARGE,
        RuntimeErrorCodeV1::RuntimePayloadTooLarge,
    )
    .await;
    let acquired: RuntimeWriterLeaseV1 =
        response(http, Method::POST, &path, json(&request), StatusCode::OK).await;
    assert_eq!(acquired.project_id, binding.project);
    assert_eq!(acquired.binding_id, binding.id);
    assert_eq!(acquired.proof.holder_id, request.holder_id);
    assert_eq!(acquired.proof.writer_fence, counter(1));
    acquired
}

#[tokio::test]
#[serial]
async fn runtime_http_persists_projector_writes_and_lazaret_revocation() {
    let (fixture, _, _, _, _) = super::common::setup_test_server()
        .await
        .expect("real server");
    let db = fixture.db();
    let binding = Binding {
        project: Uuid::new_v4(),
        id: Uuid::new_v4(),
    };
    seed_binding(&db, binding.project, binding.id, "active").await;
    let writer = Arc::new(SqlRuntimeControl::new(db.clone()));
    let projector = router(writer.clone(), PROJECTOR);
    let lazaret = router(writer, LAZARET);
    let acquired = acquire_over_http(&projector, &lazaret, binding).await;
    let observation = observation(&db, &acquired, acquired.cas.clone()).await;
    let projected: RuntimeWriteAckV1 = response(
        &projector,
        Method::PUT,
        &binding.path("/observation"),
        json(&observation),
        StatusCode::OK,
    )
    .await;
    assert_eq!(projected.cas.projection_version, counter(2));
    let instance = registration(&db, &acquired, projected.cas).await;
    let instance_path = format!("/instances/{}", instance.instance_id);
    let registered: RuntimeWriteAckV1 = response(
        &projector,
        Method::PUT,
        &binding.path(&instance_path),
        json(&instance),
        StatusCode::OK,
    )
    .await;
    assert_eq!(registered.cas.projection_version, counter(3));
    let revoke = RuntimeInstanceRevokeRequestV1 {
        schema_version: 1,
        expected: registered.cas,
        instance_id: instance.instance_id,
    };
    let revoke_path = binding.path(&format!("{instance_path}/revoke"));
    refused(
        &projector,
        Method::POST,
        &revoke_path,
        json(&revoke),
        StatusCode::FORBIDDEN,
        RuntimeErrorCodeV1::RuntimeServiceForbidden,
    )
    .await;
    let revoked: RuntimeWriteAckV1 = response(
        &lazaret,
        Method::POST,
        &revoke_path,
        json(&revoke),
        StatusCode::OK,
    )
    .await;
    assert_eq!(revoked.cas.projection_version, counter(4));
    let selector = format!(
        "instances_mode=by_instance_id&instance_id={}",
        instance.instance_id
    );
    let snapshot: RuntimeBindingSnapshotV1 = response(
        &lazaret,
        Method::GET,
        &binding.snapshot_path(&selector),
        Body::empty(),
        StatusCode::OK,
    )
    .await;
    assert!(snapshot.cas == revoked.cas);
    assert_eq!(
        snapshot.observation.expect("stored observation").endpoint,
        observation.endpoint
    );
    let items = snapshot.instances.expect("stored mapping").items;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].instance_id, instance.instance_id);
    assert_eq!(items[0].state, RuntimeInstanceStateV1::Tombstone);
}

#[tokio::test]
#[serial]
async fn runtime_http_reserves_and_completes_an_effect_once() {
    let (fixture, _, _, _, _) = super::common::setup_test_server()
        .await
        .expect("real server");
    let db: Arc<DatabaseConnection> = fixture.db();
    let begin = arrange_effect(&db).await;
    let binding = Binding {
        project: begin.context.project_id,
        id: begin.context.target.binding_id,
    };
    let writer = Arc::new(SqlRuntimeControl::new(db.clone()));
    let http = router(writer, LAZARET);
    let begin_path = binding.path("/effects/begin");
    let reservation: RuntimeEffectReservationV1 = response(
        &http,
        Method::POST,
        &begin_path,
        json(&begin),
        StatusCode::OK,
    )
    .await;
    assert_eq!(reservation.effect_id, begin.effect_id);
    assert_eq!(reservation.execution_id, begin.context.execution_id);
    assert!(reservation.deadline_unix_ms < begin.context.deadline_unix_ms);
    refused(
        &http,
        Method::POST,
        &begin_path,
        json(&begin),
        StatusCode::CONFLICT,
        RuntimeErrorCodeV1::RuntimeReplay,
    )
    .await;
    let mut complete = RuntimeCompleteEffectRequestV1 {
        schema_version: 1,
        reservation,
        completion_state: RuntimeEffectCompletionStateV1::Completed,
    };
    let complete_path = binding.path(&format!("/effects/{}/complete", begin.effect_id));
    let ack: RuntimeWriteAckV1 = response(
        &http,
        Method::POST,
        &complete_path,
        json(&complete),
        StatusCode::OK,
    )
    .await;
    assert_eq!(ack.binding_id, binding.id);
    complete.completion_state = RuntimeEffectCompletionStateV1::Indeterminate;
    refused(
        &http,
        Method::POST,
        &complete_path,
        json(&complete),
        StatusCode::CONFLICT,
        RuntimeErrorCodeV1::RuntimeIdReused,
    )
    .await;
    let rows = db.query_all(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT effect_id, completion_state FROM apparatus_runtime_effects WHERE project_id=$1 AND binding_id=$2",
        [binding.project.into(), binding.id.into()],
    )).await.expect("read persisted effect journal");
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].try_get::<Uuid>("", "effect_id").expect("effect ID"),
        begin.effect_id
    );
    assert_eq!(
        rows[0]
            .try_get::<String>("", "completion_state")
            .expect("completion"),
        "completed"
    );
}
