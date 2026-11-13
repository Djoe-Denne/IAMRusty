//! Writer-port integration against the normal migrated Postgres fixture.
//!
//! The HTTP server/OpenFGA bootstrap is unchanged. Calls below deliberately test
//! the trusted writer port, NOT mTLS authentication or Kubernetes attestation.
//! Runtime rows are arranged directly: no maintenance authority is fabricated.

mod common;
mod fixtures;

use apparatus_contracts::runtime::{
    RuntimeBeginEffectRequestV1, RuntimeBindingStateV1, RuntimeCapabilityInvokeV1, RuntimeCasV1,
    RuntimeCompleteEffectRequestV1, RuntimeControlCallerV1, RuntimeControlErrorV1,
    RuntimeControlPort, RuntimeCounterV1, RuntimeDigestV1, RuntimeEffectCompletionStateV1,
    RuntimeEffectHashInputV1, RuntimeErrorCodeV1, RuntimeExecutionContextV1,
    RuntimeInstanceCursorPayloadV1, RuntimeInstanceKindV1, RuntimeInstanceRegistrationV1,
    RuntimeInstanceRevokeRequestV1, RuntimeInstanceSelectorV1, RuntimeInstanceStateV1,
    RuntimeListCursorPayloadV1, RuntimeObservationV1, RuntimeServiceRoleV1, RuntimeSourceV1,
    RuntimeTargetV1, RuntimeUnixMsV1, RuntimeWriteGuardV1, RuntimeWriterLeaseRequestV1,
    RuntimeWriterLeaseV1, EFFECT_HASH_DOMAIN,
};
use manifesto_infra::apparatus_runtime::{
    runtime_control::{close_binding_in_transaction, SqlRuntimeControl},
    runtime_cursor::RuntimeCursorKey,
};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement, TransactionTrait};
use serial_test::serial;
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

fn counter(value: u64) -> RuntimeCounterV1 {
    RuntimeCounterV1::new(value).expect("bounded test counter")
}

fn cas() -> RuntimeCasV1 {
    RuntimeCasV1 {
        generation: counter(1),
        grant_revision: counter(1),
        revocation_epoch: counter(0),
        projection_version: counter(1),
    }
}

const fn caller(role: RuntimeServiceRoleV1) -> RuntimeControlCallerV1 {
    RuntimeControlCallerV1 { role }
}

fn digest() -> RuntimeDigestV1 {
    RuntimeDigestV1::new(format!("sha256:{}", "a".repeat(64))).expect("test release")
}

fn assert_code<T>(result: Result<T, RuntimeControlErrorV1>, code: RuntimeErrorCodeV1) {
    match result {
        Ok(_) => panic!("expected closed runtime error {code:?}"),
        Err(error) => assert_eq!(error.code, code),
    }
}

async fn now(db: &DatabaseConnection) -> RuntimeUnixMsV1 {
    let row = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS now_ms",
        ))
        .await
        .expect("writer clock")
        .expect("clock row");
    RuntimeUnixMsV1::new(row.try_get("", "now_ms").expect("milliseconds"))
        .expect("nonnegative writer clock")
}

async fn seed_binding(db: &DatabaseConnection, project: Uuid, binding: Uuid, state: &str) {
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO apparatus_runtime_bindings
         (project_id,binding_id,generation,grant_revision,revocation_epoch,state,release,profile,updated_at_ms)
         VALUES ($1,$2,1,1,0,$3,$4,'writer-port-test',0)",
        [project.into(), binding.into(), state.into(), digest().as_str().into()],
    ))
    .await
    .expect("arrange writer binding in disposable migrated DB");
}

async fn lease(
    port: &dyn RuntimeControlPort,
    project: Uuid,
    binding: Uuid,
    expected: RuntimeCasV1,
) -> RuntimeWriterLeaseV1 {
    port.writer_lease(
        &caller(RuntimeServiceRoleV1::Projector),
        project,
        binding,
        &RuntimeWriterLeaseRequestV1 {
            schema_version: 1,
            holder_id: Uuid::new_v4(),
            expected,
            renew: None,
        },
    )
    .await
    .expect("acquire writer lease")
}

fn guard(lease: &RuntimeWriterLeaseV1, expected: RuntimeCasV1) -> RuntimeWriteGuardV1 {
    RuntimeWriteGuardV1 {
        lease: lease.proof.clone(),
        expected,
    }
}

async fn observation(
    db: &DatabaseConnection,
    lease: &RuntimeWriterLeaseV1,
    expected: RuntimeCasV1,
) -> RuntimeObservationV1 {
    RuntimeObservationV1 {
        schema_version: 1,
        project_id: lease.project_id,
        binding_id: lease.binding_id,
        guard: guard(lease, expected),
        service_uid: Uuid::new_v4().to_string(),
        resource_generation: counter(2),
        observed_resource_generation: counter(2),
        revision_uid: Uuid::new_v4().to_string(),
        revision_name: "writer-revision-00001".into(),
        release: digest(),
        endpoint: "https://runtime.example.test:8443".into(),
        route_ready: true,
        observed_at_unix_ms: now(db).await,
    }
}

async fn registration(
    db: &DatabaseConnection,
    lease: &RuntimeWriterLeaseV1,
    expected: RuntimeCasV1,
) -> RuntimeInstanceRegistrationV1 {
    RuntimeInstanceRegistrationV1 {
        schema_version: 1,
        project_id: lease.project_id,
        binding_id: lease.binding_id,
        instance_id: Uuid::new_v4(),
        guard: guard(lease, expected),
        state: RuntimeInstanceStateV1::Current,
        pod_uid: Uuid::new_v4().to_string(),
        revision_uid: Uuid::new_v4().to_string(),
        namespace: "apparatus-test".into(),
        service_account: "runtime".into(),
        service_account_uid: Uuid::new_v4().to_string(),
        release: digest(),
        observed_at_unix_ms: now(db).await,
    }
}

#[tokio::test]
#[serial]
async fn lease_is_idempotent_bounded_and_fenced_after_expiry() {
    let (fixture, _, _, _, _) = common::setup_test_server().await.expect("real server");
    let db = fixture.db();
    let port: &dyn RuntimeControlPort = &SqlRuntimeControl::new(db.clone());
    let project = Uuid::new_v4();
    let binding = Uuid::new_v4();
    seed_binding(&db, project, binding, "active").await;
    let projector = caller(RuntimeServiceRoleV1::Projector);
    let acquired = lease(port, project, binding, cas()).await;
    assert_eq!(acquired.proof.writer_fence, counter(1));
    assert_eq!(
        acquired.expires_at_unix_ms.get() - acquired.issued_at_unix_ms.get(),
        10_000
    );
    let mut request = RuntimeWriterLeaseRequestV1 {
        schema_version: 1,
        holder_id: acquired.proof.holder_id,
        expected: cas(),
        renew: None,
    };
    assert!(
        port.writer_lease(&projector, project, binding, &request)
            .await
            .expect("replay")
            == acquired
    );
    request.holder_id = Uuid::new_v4();
    assert_code(
        port.writer_lease(&projector, project, binding, &request)
            .await,
        RuntimeErrorCodeV1::RuntimeLeaseHeld,
    );
    request.holder_id = acquired.proof.holder_id;
    request.renew = Some(acquired.proof.clone());
    let renewed = port
        .writer_lease(&projector, project, binding, &request)
        .await
        .expect("renew");
    assert!(renewed.proof == acquired.proof);
    assert!(renewed.expires_at_unix_ms >= acquired.expires_at_unix_ms);
    request.renew.as_mut().expect("proof").writer_fence = counter(99);
    assert_code(
        port.writer_lease(&projector, project, binding, &request)
            .await,
        RuntimeErrorCodeV1::RuntimeFenceConflict,
    );
    request.renew = Some(acquired.proof.clone());
    request.renew.as_mut().expect("proof").lease_id = Uuid::new_v4();
    assert_code(
        port.writer_lease(&projector, project, binding, &request)
            .await,
        RuntimeErrorCodeV1::RuntimeLeaseLost,
    );
    // Move both persisted timestamps, keeping the production 10-second CHECK.
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE apparatus_runtime_bindings SET lease_issued_at_ms=lease_issued_at_ms-20000,lease_expires_at_ms=lease_expires_at_ms-20000 WHERE project_id=$1 AND binding_id=$2",
        [project.into(),binding.into()])).await.expect("expire without sleep");
    request.renew = Some(acquired.proof.clone());
    assert_code(
        port.writer_lease(&projector, project, binding, &request)
            .await,
        RuntimeErrorCodeV1::RuntimeLeaseLost,
    );
    let expired_registration = registration(&db, &acquired, cas()).await;
    assert_code(
        port.register_instance(
            &projector,
            project,
            binding,
            expired_registration.instance_id,
            &expired_registration,
        )
        .await,
        RuntimeErrorCodeV1::RuntimeLeaseLost,
    );
    let takeover = lease(port, project, binding, cas()).await;
    assert_eq!(takeover.proof.writer_fence, counter(2));
    assert_ne!(takeover.proof.lease_id, acquired.proof.lease_id);
    assert_code(
        port.writer_lease(
            &caller(RuntimeServiceRoleV1::Lazaret),
            project,
            binding,
            &request,
        )
        .await,
        RuntimeErrorCodeV1::RuntimeServiceForbidden,
    );
    request.expected.generation = counter(2);
    request.renew = None;
    assert_code(
        port.writer_lease(&projector, project, binding, &request)
            .await,
        RuntimeErrorCodeV1::RuntimeStaleState,
    );
    request.expected = cas();
    request.schema_version = 2;
    assert_code(
        port.writer_lease(&projector, project, binding, &request)
            .await,
        RuntimeErrorCodeV1::RuntimeUnsupportedVersion,
    );
    request.schema_version = 1;
    request.holder_id = Uuid::nil();
    assert_code(
        port.writer_lease(&projector, project, binding, &request)
            .await,
        RuntimeErrorCodeV1::RuntimeInvalidRequest,
    );
}

#[tokio::test]
#[serial]
async fn observation_replay_preserves_cas_and_uid_and_rejections_do_not_write() {
    let (fixture, _, _, _, _) = common::setup_test_server().await.expect("real server");
    let db = fixture.db();
    let port: &dyn RuntimeControlPort = &SqlRuntimeControl::new(db.clone());
    let project = Uuid::new_v4();
    let binding = Uuid::new_v4();
    seed_binding(&db, project, binding, "active").await;
    let acquired = lease(port, project, binding, cas()).await;
    let projector = caller(RuntimeServiceRoleV1::Projector);
    let request = observation(&db, &acquired, cas()).await;
    let ack = port
        .report_observation(&projector, project, binding, &request)
        .await
        .expect("first projection");
    assert_eq!(ack.cas.projection_version, counter(2));
    assert!(
        port.report_observation(&projector, project, binding, &request)
            .await
            .expect("identical stale replay")
            .cas
            == ack.cas
    );
    let mut changed = request.clone();
    changed.endpoint = "https://changed.example.test".into();
    assert_code(
        port.report_observation(&projector, project, binding, &changed)
            .await,
        RuntimeErrorCodeV1::RuntimeStaleState,
    );
    changed.guard.expected = ack.cas.clone();
    let updated = port
        .report_observation(&projector, project, binding, &changed)
        .await
        .expect("fresh CAS update");
    assert_eq!(updated.cas.projection_version, counter(3));
    let baseline = changed.clone();
    let cases = [
        ("uid", RuntimeErrorCodeV1::RuntimeFenceConflict),
        ("generation", RuntimeErrorCodeV1::RuntimeStaleState),
        ("lease", RuntimeErrorCodeV1::RuntimeLeaseLost),
        ("fence", RuntimeErrorCodeV1::RuntimeFenceConflict),
        ("old", RuntimeErrorCodeV1::RuntimeProjectionUnavailable),
        ("future", RuntimeErrorCodeV1::RuntimeProjectionUnavailable),
        ("version", RuntimeErrorCodeV1::RuntimeUnsupportedVersion),
        ("path", RuntimeErrorCodeV1::RuntimeInvalidRequest),
        ("route", RuntimeErrorCodeV1::RuntimeInvalidRequest),
        ("release", RuntimeErrorCodeV1::RuntimeStaleState),
    ];
    for (case, expected) in cases {
        let mut invalid = baseline.clone();
        invalid.guard.expected = updated.cas.clone();
        match case {
            "uid" => invalid.service_uid = Uuid::new_v4().to_string(),
            "generation" => invalid.guard.expected.generation = counter(2),
            "lease" => invalid.guard.lease.lease_id = Uuid::new_v4(),
            "fence" => invalid.guard.lease.writer_fence = counter(99),
            "old" => invalid.observed_at_unix_ms = RuntimeUnixMsV1::new(0).expect("epoch"),
            "future" => {
                invalid.observed_at_unix_ms =
                    RuntimeUnixMsV1::new(now(&db).await.get() + 60_000).expect("future");
            }
            "version" => invalid.schema_version = 2,
            "path" => invalid.project_id = Uuid::new_v4(),
            "route" => invalid.observed_resource_generation = counter(1),
            "release" => {
                invalid.release = RuntimeDigestV1::new(format!("sha256:{}", "b".repeat(64)))
                    .expect("different release");
            }
            _ => unreachable!("closed cases"),
        }
        assert_code(
            port.report_observation(&projector, project, binding, &invalid)
                .await,
            expected,
        );
    }
    assert_code(
        port.report_observation(
            &caller(RuntimeServiceRoleV1::Lazaret),
            project,
            binding,
            &request,
        )
        .await,
        RuntimeErrorCodeV1::RuntimeServiceForbidden,
    );
    let snapshot = port
        .get_binding(
            &projector,
            project,
            binding,
            &RuntimeInstanceSelectorV1::ByInstanceId {
                instance_id: Uuid::new_v4(),
            },
        )
        .await
        .expect("read committed projection");
    assert!(snapshot.cas == updated.cas);
    let stored = snapshot.observation.expect("projection retained");
    assert_eq!(stored.service_uid, request.service_uid);
    assert_eq!(stored.endpoint, baseline.endpoint);
    assert!(stored.valid_until_unix_ms > stored.observed_at_unix_ms);
}

#[tokio::test]
#[serial]
async fn pod_mapping_is_immutable_and_revocation_cannot_be_undone() {
    let (fixture, _, _, _, _) = common::setup_test_server().await.expect("real server");
    let db = fixture.db();
    let port: &dyn RuntimeControlPort = &SqlRuntimeControl::new(db.clone());
    let project = Uuid::new_v4();
    let binding = Uuid::new_v4();
    seed_binding(&db, project, binding, "active").await;
    let acquired = lease(port, project, binding, cas()).await;
    let projector = caller(RuntimeServiceRoleV1::Projector);
    let lazaret = caller(RuntimeServiceRoleV1::Lazaret);
    let request = registration(&db, &acquired, cas()).await;
    let id = request.instance_id;
    let ack = port
        .register_instance(&projector, project, binding, id, &request)
        .await
        .expect("register Pod");
    assert_eq!(ack.cas.projection_version, counter(2));
    assert!(
        port.register_instance(&projector, project, binding, id, &request)
            .await
            .expect("idempotent replay")
            .cas
            == ack.cas
    );
    let mut changed = request.clone();
    changed.guard.expected = ack.cas.clone();
    changed.pod_uid = Uuid::new_v4().to_string();
    assert_code(
        port.register_instance(&projector, project, binding, id, &changed)
            .await,
        RuntimeErrorCodeV1::RuntimeIdReused,
    );
    changed = request.clone();
    changed.instance_id = Uuid::new_v4();
    assert_code(
        port.register_instance(&projector, project, binding, changed.instance_id, &changed)
            .await,
        RuntimeErrorCodeV1::RuntimeIdReused,
    );
    changed = request.clone();
    changed.observed_at_unix_ms =
        RuntimeUnixMsV1::new(request.observed_at_unix_ms.get() - 1).expect("different observation");
    assert_code(
        port.register_instance(&projector, project, binding, id, &changed)
            .await,
        RuntimeErrorCodeV1::RuntimeStaleState,
    );
    changed.guard.expected = ack.cas.clone();
    let refreshed = port
        .register_instance(&projector, project, binding, id, &changed)
        .await
        .expect("refresh same Pod");
    let revoke = RuntimeInstanceRevokeRequestV1 {
        schema_version: 1,
        expected: refreshed.cas,
        instance_id: id,
    };
    let mut stale_revoke = revoke.clone();
    stale_revoke.expected = cas();
    assert_code(
        port.revoke_instance(&lazaret, project, binding, id, &stale_revoke)
            .await,
        RuntimeErrorCodeV1::RuntimeStaleState,
    );
    assert_code(
        port.revoke_instance(&projector, project, binding, id, &revoke)
            .await,
        RuntimeErrorCodeV1::RuntimeServiceForbidden,
    );
    let revoked = port
        .revoke_instance(&lazaret, project, binding, id, &revoke)
        .await
        .expect("revoke known Pod");
    assert!(
        port.revoke_instance(&lazaret, project, binding, id, &revoke)
            .await
            .expect("revocation replay")
            .cas
            == revoked.cas
    );
    changed.guard.expected = revoked.cas.clone();
    assert_code(
        port.register_instance(&projector, project, binding, id, &changed)
            .await,
        RuntimeErrorCodeV1::RuntimeIdentityRevoked,
    );
    let snapshot = port
        .get_binding(
            &lazaret,
            project,
            binding,
            &RuntimeInstanceSelectorV1::ByPodUid {
                pod_uid: request.pod_uid.clone(),
            },
        )
        .await
        .expect("retained mapping");
    let items = snapshot.instances.expect("explicit selection").items;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].instance_id, id);
    assert_eq!(items[0].state, RuntimeInstanceStateV1::Tombstone);
    assert_eq!(
        items[0].pod.as_ref().expect("Pod preserved").pod_uid,
        request.pod_uid
    );
    assert!(items[0].closed_at_unix_ms.is_some());
    let unknown = Uuid::new_v4();
    let ack = port
        .revoke_instance(
            &lazaret,
            project,
            binding,
            unknown,
            &RuntimeInstanceRevokeRequestV1 {
                schema_version: 1,
                expected: revoked.cas,
                instance_id: unknown,
            },
        )
        .await
        .expect("unknown identity fence");
    let snapshot = port
        .get_binding(
            &lazaret,
            project,
            binding,
            &RuntimeInstanceSelectorV1::ByInstanceId {
                instance_id: unknown,
            },
        )
        .await
        .expect("legacy fence read");
    assert!(snapshot.cas == ack.cas);
    let items = snapshot.instances.expect("selection").items;
    assert_eq!(items[0].kind, RuntimeInstanceKindV1::LegacyFence);
    assert_eq!(items[0].state, RuntimeInstanceStateV1::Tombstone);
    assert!(items[0].pod.is_none());
}

#[tokio::test]
#[serial]
async fn binding_and_instance_pages_are_scoped_stable_and_authenticated() {
    let (fixture, _, _, _, _) = common::setup_test_server().await.expect("real server");
    let db = fixture.db();
    let key = Arc::new(
        RuntimeCursorKey::new(format!("rcl-v1-{}", Uuid::new_v4()), rand::random())
            .expect("ephemeral cursor key"),
    );
    let port: &dyn RuntimeControlPort =
        &SqlRuntimeControl::new(db.clone()).with_cursor_key(Arc::clone(&key));
    let projector = caller(RuntimeServiceRoleV1::Projector);
    let project = Uuid::new_v4();
    let mut ids = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
    ids.sort();
    for (binding, state) in ids.into_iter().zip(["active", "draining", "tombstone"]) {
        seed_binding(&db, project, binding, state).await;
    }
    let page = port
        .list_bindings(&projector, project, None, Some(1))
        .await
        .expect("first page");
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].binding_id, ids[0]);
    assert_eq!(page.items[0].state, RuntimeBindingStateV1::Active);
    let cursor = page.next_cursor.expect("continuation");
    assert_code(
        port.list_bindings(&projector, Uuid::new_v4(), Some(&cursor), Some(1))
            .await,
        RuntimeErrorCodeV1::RuntimeCursorInvalid,
    );
    assert_code(
        port.list_bindings(
            &caller(RuntimeServiceRoleV1::Lazaret),
            project,
            Some(&cursor),
            Some(1),
        )
        .await,
        RuntimeErrorCodeV1::RuntimeCursorInvalid,
    );
    assert_code(
        port.list_bindings(&projector, project, Some(&cursor), Some(2))
            .await,
        RuntimeErrorCodeV1::RuntimeCursorInvalid,
    );
    let mut tampered = cursor.clone();
    tampered.0.push('x');
    assert_code(
        port.list_bindings(&projector, project, Some(&tampered), Some(1))
            .await,
        RuntimeErrorCodeV1::RuntimeCursorInvalid,
    );
    let page = port
        .list_bindings(&projector, project, Some(&cursor), Some(1))
        .await
        .expect("second page");
    assert_eq!(page.items[0].binding_id, ids[1]);
    assert_eq!(page.items[0].state, RuntimeBindingStateV1::Draining);
    let page = port
        .list_bindings(&projector, project, page.next_cursor.as_ref(), Some(1))
        .await
        .expect("last page includes tombstone");
    assert_eq!(page.items[0].binding_id, ids[2]);
    assert_eq!(page.items[0].state, RuntimeBindingStateV1::Tombstone);
    assert!(page.next_cursor.is_none());
    assert!(port
        .list_bindings(&projector, Uuid::new_v4(), None, None)
        .await
        .expect("empty project")
        .items
        .is_empty());
    for size in [0, 201] {
        assert_code(
            port.list_bindings(&projector, project, None, Some(size))
                .await,
            RuntimeErrorCodeV1::RuntimeInvalidRequest,
        );
    }
    assert_code(
        SqlRuntimeControl::new(db.clone())
            .list_bindings(&projector, project, None, Some(1))
            .await,
        RuntimeErrorCodeV1::RuntimeKeyUnavailable,
    );
    let acquired = lease(port, project, ids[0], cas()).await;
    let mut expected = cas();
    let mut instances = Vec::new();
    for _ in 0..2 {
        let request = registration(&db, &acquired, expected).await;
        expected = port
            .register_instance(&projector, project, ids[0], request.instance_id, &request)
            .await
            .expect("register replica")
            .cas;
        instances.push(request.instance_id);
    }
    instances.sort();
    let snapshot = port
        .get_binding(
            &projector,
            project,
            ids[0],
            &RuntimeInstanceSelectorV1::Page {
                cursor: None,
                page_size: Some(1),
            },
        )
        .await
        .expect("first replica page");
    let first = snapshot.instances.expect("page");
    assert_eq!(first.items[0].instance_id, instances[0]);
    assert!(first.items[0].valid_until_unix_ms.expect("validity") > first.read_at_unix_ms);
    let cursor = first.next_cursor.expect("replica cursor");
    let snapshot = port
        .get_binding(
            &projector,
            project,
            ids[0],
            &RuntimeInstanceSelectorV1::Page {
                cursor: Some(cursor.clone()),
                page_size: Some(1),
            },
        )
        .await
        .expect("second replica page");
    let last = snapshot.instances.expect("page");
    assert_eq!(last.items[0].instance_id, instances[1]);
    assert!(last.next_cursor.is_none());
    assert_code(
        port.get_binding(
            &projector,
            project,
            ids[1],
            &RuntimeInstanceSelectorV1::Page {
                cursor: Some(cursor),
                page_size: Some(1),
            },
        )
        .await,
        RuntimeErrorCodeV1::RuntimeCursorInvalid,
    );
    assert_code(
        port.get_binding(
            &projector,
            project,
            ids[0],
            &RuntimeInstanceSelectorV1::Page {
                cursor: None,
                page_size: Some(0),
            },
        )
        .await,
        RuntimeErrorCodeV1::RuntimeInvalidRequest,
    );
    assert_code(
        port.get_binding(
            &projector,
            project,
            Uuid::new_v4(),
            &RuntimeInstanceSelectorV1::ByInstanceId {
                instance_id: Uuid::new_v4(),
            },
        )
        .await,
        RuntimeErrorCodeV1::RuntimeBindingNotFound,
    );
    // The test owns the injected key: sign a valid historical cursor without
    // tampering with its MAC, sleeping for its TTL, or changing production time.
    let current = now(&db).await.get();
    let issued = RuntimeUnixMsV1::new(current - 60_001).expect("historical issue");
    let expired = RuntimeUnixMsV1::new(current - 1).expect("historical expiry");
    let cursor = key
        .encode(&RuntimeListCursorPayloadV1 {
            schema_version: 1,
            scan_id: Uuid::new_v4(),
            project_id: project,
            principal: projector.role.principal().into(),
            after_binding_id: Uuid::nil(),
            upper_binding_id: ids[2],
            page_size: 1,
            issued_at_unix_ms: issued,
            expires_at_unix_ms: expired,
        })
        .expect("authenticated expired binding cursor");
    assert_code(
        port.list_bindings(&projector, project, Some(&cursor), Some(1))
            .await,
        RuntimeErrorCodeV1::RuntimeCursorExpired,
    );
    let cursor = key
        .encode_instance(&RuntimeInstanceCursorPayloadV1 {
            schema_version: 1,
            scan_id: Uuid::new_v4(),
            project_id: project,
            binding_id: ids[0],
            principal: projector.role.principal().into(),
            after_instance_id: Uuid::nil(),
            upper_instance_id: instances[1],
            page_size: 1,
            issued_at_unix_ms: issued,
            expires_at_unix_ms: expired,
        })
        .expect("authenticated expired instance cursor");
    assert_code(
        port.get_binding(
            &projector,
            project,
            ids[0],
            &RuntimeInstanceSelectorV1::Page {
                cursor: Some(cursor),
                page_size: Some(1),
            },
        )
        .await,
        RuntimeErrorCodeV1::RuntimeCursorExpired,
    );
}

#[tokio::test]
#[serial]
async fn business_fence_closes_instances_and_preserves_tombstone_on_replay() {
    let (fixture, _, _, _, _) = common::setup_test_server().await.expect("real server");
    let db = fixture.db();
    let port: &dyn RuntimeControlPort = &SqlRuntimeControl::new(db.clone());
    let project = Uuid::new_v4();
    let binding = Uuid::new_v4();
    seed_binding(&db, project, binding, "active").await;
    let acquired = lease(port, project, binding, cas()).await;
    let projector = caller(RuntimeServiceRoleV1::Projector);
    let request = registration(&db, &acquired, cas()).await;
    port.register_instance(&projector, project, binding, request.instance_id, &request)
        .await
        .expect("register");
    // Transaction ownership and commit are explicit; this adapter test does not
    // claim to exercise a business handler's project/domain lock acquisition.
    let txn = db.begin().await.expect("business transaction");
    close_binding_in_transaction(&txn, project, binding, counter(2), counter(1), true)
        .await
        .expect("fence");
    txn.commit().await.expect("commit fence");
    let delayed = observation(&db, &acquired, cas()).await;
    assert_code(
        port.report_observation(&projector, project, binding, &delayed)
            .await,
        RuntimeErrorCodeV1::RuntimeBindingClosed,
    );
    let selector = RuntimeInstanceSelectorV1::ByInstanceId {
        instance_id: request.instance_id,
    };
    let closed = port
        .get_binding(&projector, project, binding, &selector)
        .await
        .expect("read fence");
    assert_eq!(closed.state, RuntimeBindingStateV1::Tombstone);
    assert_eq!(closed.cas.generation, counter(2));
    assert_eq!(closed.cas.revocation_epoch, counter(1));
    assert_eq!(
        closed.instances.as_ref().expect("instances").items[0].state,
        RuntimeInstanceStateV1::Tombstone
    );
    assert_code(
        port.register_instance(&projector, project, binding, request.instance_id, &request)
            .await,
        RuntimeErrorCodeV1::RuntimeLeaseLost,
    );
    let txn = db.begin().await.expect("retry transaction");
    close_binding_in_transaction(&txn, project, binding, counter(2), counter(1), false)
        .await
        .expect("replay never downgrades tombstone");
    close_binding_in_transaction(&txn, project, Uuid::new_v4(), counter(1), counter(1), false)
        .await
        .expect("unmaterialized is no-op");
    txn.commit().await.expect("commit replay");
    let replay = port
        .get_binding(&projector, project, binding, &selector)
        .await
        .expect("read replay");
    assert!(replay.cas == closed.cas);
    assert_eq!(replay.state, closed.state);
    let txn = db.begin().await.expect("stale transaction");
    assert_code(
        close_binding_in_transaction(&txn, project, binding, counter(1), counter(1), false).await,
        RuntimeErrorCodeV1::RuntimeStaleState,
    );
    txn.rollback().await.expect("rollback stale write");
    assert!(
        port.get_binding(&projector, project, binding, &selector)
            .await
            .expect("unchanged after rollback")
            .cas
            == closed.cas
    );
}

async fn seed_grants(db: &DatabaseConnection, binding: Uuid) {
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE projects SET status='active' WHERE id=(SELECT project_id FROM project_components WHERE id=$1)",
        [binding.into()])).await.expect("activate fixture project");
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE project_components SET status='active' WHERE id=$1",
        [binding.into()],
    ))
    .await
    .expect("activate fixture component");
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO apparatus_bindings (component_id,source,digest,desired_generation,grant_revision,declared_capabilities)
         VALUES ($1,'managed',$2,1,1,'[\"runtime.invoke\",\"storage.kv.read\"]'::jsonb)",
        [binding.into(), digest().as_str().into()])).await.expect("business binding");
    for capability in ["runtime.invoke", "storage.kv.read"] {
        db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO apparatus_capability_consents (component_id,capability,status,grant_revision) VALUES ($1,$2,'consented',1)",
            [binding.into(),capability.into()])).await.expect("real consent");
    }
}

fn rehash(request: &mut RuntimeBeginEffectRequestV1) {
    request.effect_sha256 = RuntimeEffectHashInputV1 {
        schema_version: 1,
        domain: EFFECT_HASH_DOMAIN.into(),
        operation_sha256: request.context.operation_sha256.clone(),
        execution_id: request.context.execution_id,
        effect_id: request.effect_id,
        instance_id: request.context.instance_id,
        invoke: request.invoke.clone(),
    }
    .sha256()
    .expect("canonical effect hash");
}

#[tokio::test]
#[serial]
async fn effect_reservation_requires_live_grants_and_never_replays_io() {
    let (fixture, _, _, _, _) = common::setup_test_server().await.expect("real server");
    let db = fixture.db();
    let (project_fixture, _, source_component) =
        fixtures::DbFixtures::create_project_with_component(&db, Uuid::new_v4(), "taskboard")
            .await
            .expect("business project");
    let project = project_fixture.id();
    let target_component = fixtures::DbFixtures::component()
        .for_project(project)
        .component_type("wiki")
        .commit(db.clone())
        .await
        .expect("target component");
    let source = source_component.id();
    let target = target_component.id();
    let port: &dyn RuntimeControlPort = &SqlRuntimeControl::new(db.clone());
    let projector = caller(RuntimeServiceRoleV1::Projector);
    let lazaret = caller(RuntimeServiceRoleV1::Lazaret);
    let mut instance_ids = Vec::new();
    for binding in [source, target] {
        seed_binding(&db, project, binding, "active").await;
        seed_grants(&db, binding).await;
        let acquired = lease(port, project, binding, cas()).await;
        let registration = registration(&db, &acquired, cas()).await;
        port.register_instance(
            &projector,
            project,
            binding,
            registration.instance_id,
            &registration,
        )
        .await
        .expect("current identity");
        instance_ids.push(registration.instance_id);
    }
    let deadline = RuntimeUnixMsV1::new(now(&db).await.get() + 60_000).expect("requested deadline");
    let mut request = RuntimeBeginEffectRequestV1 {
        schema_version: 1,
        context: RuntimeExecutionContextV1 {
            schema_version: 1,
            execution_id: Uuid::new_v4(),
            instance_id: instance_ids[1],
            authorization_id: Uuid::new_v4(),
            project_id: project,
            source: RuntimeSourceV1 {
                instance: instance_ids[0],
                binding: source,
                release: digest(),
                generation: counter(1),
                grant_revision: counter(1),
            },
            source_revocation_epoch: counter(0),
            target: RuntimeTargetV1 {
                binding_id: target,
                release: digest(),
                generation: counter(1),
                grant_revision: counter(1),
                revocation_epoch: counter(0),
            },
            operation_sha256: digest(),
            deadline_unix_ms: deadline,
        },
        effect_id: Uuid::new_v4(),
        effect_sha256: digest(),
        invoke: RuntimeCapabilityInvokeV1 {
            binding_id: target,
            operation_id: Uuid::new_v4(),
            operation: "kv.get".into(),
            params: serde_json::json!({"key":"coverage"}),
        },
    };
    rehash(&mut request);
    assert_code(
        port.begin_effect(&projector, project, target, &request)
            .await,
        RuntimeErrorCodeV1::RuntimeServiceForbidden,
    );
    let mut invalid = request.clone();
    invalid.effect_sha256 =
        RuntimeDigestV1::new(format!("sha256:{}", "b".repeat(64))).expect("different hash");
    assert_code(
        port.begin_effect(&lazaret, project, target, &invalid).await,
        RuntimeErrorCodeV1::RuntimeHashMismatch,
    );
    invalid = request.clone();
    invalid.context.target.generation = counter(2);
    assert_code(
        port.begin_effect(&lazaret, project, target, &invalid).await,
        RuntimeErrorCodeV1::RuntimeStaleState,
    );
    invalid = request.clone();
    invalid.context.deadline_unix_ms =
        RuntimeUnixMsV1::new(now(&db).await.get() - 1).expect("past deadline");
    assert_code(
        port.begin_effect(&lazaret, project, target, &invalid).await,
        RuntimeErrorCodeV1::RuntimeDeadlineExceeded,
    );
    let reservation = port
        .begin_effect(&lazaret, project, target, &request)
        .await
        .expect("durable reservation");
    assert!(reservation.deadline_unix_ms < deadline);
    assert_eq!(reservation.effect_id, request.effect_id);
    assert_code(
        port.begin_effect(&lazaret, project, target, &request).await,
        RuntimeErrorCodeV1::RuntimeReplay,
    );
    let mut changed = request.clone();
    changed.invoke.params = serde_json::json!({"key":"different"});
    rehash(&mut changed);
    assert_code(
        port.begin_effect(&lazaret, project, target, &changed).await,
        RuntimeErrorCodeV1::RuntimeIdReused,
    );
    changed = request.clone();
    changed.effect_id = Uuid::new_v4();
    changed.context.authorization_id = Uuid::new_v4();
    rehash(&mut changed);
    assert_code(
        port.begin_effect(&lazaret, project, target, &changed).await,
        RuntimeErrorCodeV1::RuntimeIdReused,
    );
    let completion = RuntimeCompleteEffectRequestV1 {
        schema_version: 1,
        reservation,
        completion_state: RuntimeEffectCompletionStateV1::Completed,
    };
    let ack = port
        .complete_effect(&lazaret, project, target, request.effect_id, &completion)
        .await
        .expect("completion");
    assert!(
        port.complete_effect(&lazaret, project, target, request.effect_id, &completion)
            .await
            .expect("idempotent completion")
            .cas
            == ack.cas
    );
    let mut contradicted = completion.clone();
    contradicted.completion_state = RuntimeEffectCompletionStateV1::Indeterminate;
    assert_code(
        port.complete_effect(&lazaret, project, target, request.effect_id, &contradicted)
            .await,
        RuntimeErrorCodeV1::RuntimeIdReused,
    );
    contradicted = completion.clone();
    contradicted.reservation.execution_id = Uuid::new_v4();
    assert_code(
        port.complete_effect(&lazaret, project, target, request.effect_id, &contradicted)
            .await,
        RuntimeErrorCodeV1::RuntimeIdReused,
    );
    // Revoke the target capability in the writer DB: a new effect must re-read
    // it, not reuse the previous successful authorization/reservation.
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE apparatus_capability_consents SET status='revoked' WHERE component_id=$1 AND capability='storage.kv.read'", [target.into()])).await.expect("revoke consent");
    changed = request.clone();
    changed.effect_id = Uuid::new_v4();
    rehash(&mut changed);
    assert_code(
        port.begin_effect(&lazaret, project, target, &changed).await,
        RuntimeErrorCodeV1::RuntimeCallerForbidden,
    );
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE apparatus_capability_consents SET status='consented' WHERE component_id=$1 AND capability='storage.kv.read'", [target.into()])).await.expect("restore consent before identity gate");
    let target_snapshot = port
        .get_binding(
            &lazaret,
            project,
            target,
            &RuntimeInstanceSelectorV1::ByInstanceId {
                instance_id: instance_ids[1],
            },
        )
        .await
        .expect("current target CAS");
    port.revoke_instance(
        &lazaret,
        project,
        target,
        instance_ids[1],
        &RuntimeInstanceRevokeRequestV1 {
            schema_version: 1,
            expected: target_snapshot.cas,
            instance_id: instance_ids[1],
        },
    )
    .await
    .expect("revoke target identity");
    assert_code(
        port.begin_effect(&lazaret, project, target, &changed).await,
        RuntimeErrorCodeV1::RuntimeIdentityRevoked,
    );
    let txn = db.begin().await.expect("close target");
    close_binding_in_transaction(&txn, project, target, counter(1), counter(1), true)
        .await
        .expect("tombstone target");
    txn.commit().await.expect("commit target closure");
    assert_code(
        port.begin_effect(&lazaret, project, target, &changed).await,
        RuntimeErrorCodeV1::RuntimeBindingClosed,
    );
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT count(*)::bigint AS total FROM apparatus_runtime_effects WHERE project_id=$1 AND binding_id=$2", [project.into(),target.into()])).await.expect("effect journal").expect("count row");
    assert_eq!(row.try_get::<i64>("", "total").expect("count"), 1);
}

async fn blocked_transaction_start(db: &DatabaseConnection, holder_pid: i32) -> i64 {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let row = db
                .query_one(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT floor(extract(epoch FROM xact_start)*1000)::bigint AS started_ms
                 FROM pg_stat_activity WHERE $1 = ANY(pg_blocking_pids(pid)) LIMIT 1",
                    [holder_pid.into()],
                ))
                .await
                .expect("observe fixture row-lock waiter");
            if let Some(row) = row {
                return row
                    .try_get("", "started_ms")
                    .expect("waiting transaction clock");
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("writer must actually reach the row lock")
}

#[tokio::test]
#[serial]
async fn concurrent_leases_are_exclusive_and_clock_is_read_after_row_lock() {
    let (fixture, _, _, _, _) = common::setup_test_server().await.expect("real server");
    let db = fixture.db();
    let port = Arc::new(SqlRuntimeControl::new(db.clone()));
    let project = Uuid::new_v4();
    let binding = Uuid::new_v4();
    seed_binding(&db, project, binding, "active").await;
    let projector = caller(RuntimeServiceRoleV1::Projector);
    let first = RuntimeWriterLeaseRequestV1 {
        schema_version: 1,
        holder_id: Uuid::new_v4(),
        expected: cas(),
        renew: None,
    };
    let mut second = first.clone();
    second.holder_id = Uuid::new_v4();
    let (one, two) = tokio::join!(
        port.writer_lease(&projector, project, binding, &first),
        port.writer_lease(&projector, project, binding, &second),
    );
    let winner = match (one, two) {
        (Ok(winner), Err(loser)) | (Err(loser), Ok(winner)) => {
            assert_eq!(loser.code, RuntimeErrorCodeV1::RuntimeLeaseHeld);
            winner
        }
        _ => panic!("concurrent distinct holders must produce exactly one winner"),
    };
    assert_eq!(winner.proof.writer_fence, counter(1));
    let replay = RuntimeWriterLeaseRequestV1 {
        holder_id: winner.proof.holder_id,
        ..first.clone()
    };
    assert!(
        port.writer_lease(&projector, project, binding, &replay)
            .await
            .expect("persisted winner")
            == winner
    );

    // Establish a real blocking relationship in Postgres, rather than relying
    // on a scheduler race or a sleep. A transaction-start clock would predate
    // release; a missing SELECT FOR UPDATE would clock before the UPDATE waits.
    let blocked_binding = Uuid::new_v4();
    seed_binding(&db, project, blocked_binding, "active").await;
    let hold = db.begin().await.expect("row-lock owner");
    hold.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT binding_id FROM apparatus_runtime_bindings WHERE project_id=$1 AND binding_id=$2 FOR UPDATE",
        [project.into(), blocked_binding.into()])).await.expect("hold exact binding").expect("binding exists");
    let pid = hold
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT pg_backend_pid() AS pid",
        ))
        .await
        .expect("holder PID query")
        .expect("holder PID row")
        .try_get::<i32>("", "pid")
        .expect("holder PID");
    let writer = Arc::clone(&port);
    let mut pending = tokio::spawn(async move {
        writer
            .writer_lease(
                &caller(RuntimeServiceRoleV1::Projector),
                project,
                blocked_binding,
                &first,
            )
            .await
    });
    let started_ms = tokio::select! {
        started = blocked_transaction_start(&db, pid) => started,
        _ = &mut pending => panic!("lease must wait for the held writer row"),
    };
    let release_clock = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let current = now(&db).await;
            if current.get() > started_ms {
                break current;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("writer wall clock must advance");
    hold.commit().await.expect("release writer row");
    let granted = tokio::time::timeout(Duration::from_secs(5), pending)
        .await
        .expect("unblocked lease")
        .expect("writer task")
        .expect("lease after lock");
    assert!(granted.issued_at_unix_ms >= release_clock);
    assert_eq!(
        granted.expires_at_unix_ms.get() - granted.issued_at_unix_ms.get(),
        10_000
    );
    assert_eq!(granted.proof.writer_fence, counter(1));
}
