//! Local maintenance command integration over the migrated, disposable writer.
//! The typed test channel supplies fixture data; this is not operator or D
//! authentication, mTLS, or Kubernetes attestation proof.

use super::{assert_code, counter, fixtures, now};
use apparatus_contracts::runtime::{
    FenceRuntimeBindingV1, RuntimeAdmissionActionV1, RuntimeAdmissionChallengeV1,
    RuntimeBindingCutoverPlanV1, RuntimeCasV1, RuntimeControlErrorV1, RuntimeDigestV1,
    RuntimeErrorCodeV1, RuntimeLegacyFenceInstallationAckV1, RuntimeUnixMsV1,
    ADMISSION_CHALLENGE_TTL_MS,
};
use manifesto_application::{
    RuntimeMaintenanceAction, RuntimeMaintenanceCommand, RuntimeMaintenanceCommandHandler,
    RuntimeMaintenanceInspection, RuntimeMaintenanceReply,
};
use manifesto_infra::apparatus_runtime::{
    admission_proof::RejectRuntimeAdmissionProofVerifier,
    runtime_control::maintenance::{
        RuntimeMaintenanceAct, RuntimeMaintenanceChannel, RuntimeMaintenanceFacts,
        RuntimeMaintenanceProfile, SqlRuntimeMaintenance,
    },
};
use rustycog::command::CommandHandler;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serde_json::{json, Value};
use serial_test::serial;
use std::{future::Future, pin::Pin, sync::Arc};
use uuid::Uuid;

struct FixtureChannel {
    operator: Uuid,
    ack: RuntimeLegacyFenceInstallationAckV1,
}

impl RuntimeMaintenanceChannel for FixtureChannel {
    fn authorize<'a>(
        &'a self,
        act: RuntimeMaintenanceAct,
        plan: &'a RuntimeBindingCutoverPlanV1,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<RuntimeMaintenanceFacts, RuntimeControlErrorV1>> + Send + 'a,
        >,
    > {
        Box::pin(async move {
            assert!(matches!(
                act,
                RuntimeMaintenanceAct::Inspect
                    | RuntimeMaintenanceAct::Fence
                    | RuntimeMaintenanceAct::Challenge(_)
            ));
            assert_eq!(plan.project_id, self.ack.project_id);
            assert_eq!(plan.binding_id, self.ack.binding_id);
            Ok(RuntimeMaintenanceFacts {
                operator_id: self.operator,
                installation_ack: self.ack.clone(),
                drain_evidence_sha256: None,
            })
        })
    }
}

struct Cutover {
    plan: RuntimeBindingCutoverPlanV1,
    authority: Uuid,
    installation: Uuid,
    operator: Uuid,
    installed_at: RuntimeUnixMsV1,
}

impl Cutover {
    async fn arrange(db: &Arc<DatabaseConnection>, component_state: &str) -> Self {
        let (project, _, component) =
            fixtures::DbFixtures::create_project_with_component(db, Uuid::new_v4(), "taskboard")
                .await
                .expect("business project and component");
        let release = super::digest();
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE projects SET status = 'active' WHERE id = $1",
            [project.id().into()],
        ))
        .await
        .expect("arrange active project");
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE project_components SET status = $2 WHERE id = $1",
            [component.id().into(), component_state.into()],
        ))
        .await
        .expect("arrange business component state");
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO apparatus_bindings
             (component_id, source, digest, desired_generation, grant_revision, declared_capabilities)
             VALUES ($1, 'managed', $2, 7, 3, '[]'::jsonb)",
            [component.id().into(), release.as_str().into()],
        ))
        .await
        .expect("arrange business binding without a runtime row");
        Self {
            plan: RuntimeBindingCutoverPlanV1 {
                schema_version: 1,
                migration_id: Uuid::new_v4(),
                project_id: project.id(),
                binding_id: component.id(),
                expected_generation: counter(7),
                expected_grant_revision: counter(3),
                release,
                profile: "writer-maintenance-test".into(),
                profile_lock_sha256: digest('b'),
                inventory_sha256: digest('c'),
            },
            authority: Uuid::new_v4(),
            installation: Uuid::new_v4(),
            operator: Uuid::new_v4(),
            installed_at: now(db).await,
        }
    }

    fn handler(
        &self,
        db: &Arc<DatabaseConnection>,
        observed_cas: Option<RuntimeCasV1>,
        installed_at: RuntimeUnixMsV1,
    ) -> RuntimeMaintenanceCommandHandler {
        let channel = FixtureChannel {
            operator: self.operator,
            ack: RuntimeLegacyFenceInstallationAckV1 {
                migration_id: self.plan.migration_id,
                project_id: self.plan.project_id,
                binding_id: self.plan.binding_id,
                inventory_sha256: self.plan.inventory_sha256.clone(),
                installed_at_unix_ms: installed_at,
                observed_writer_cas: observed_cas,
            },
        };
        let writer = SqlRuntimeMaintenance::disabled(Arc::clone(db))
            .with_verified_channel(
                RuntimeMaintenanceProfile {
                    authority_id: self.authority,
                    installation_id: self.installation,
                    profile: self.plan.profile.clone(),
                    profile_lock_sha256: self.plan.profile_lock_sha256.clone(),
                    endpoint_scheme: "https".into(),
                    endpoint_port: 443,
                },
                Arc::new(channel),
                Arc::new(RejectRuntimeAdmissionProofVerifier),
            )
            .expect("valid test-owned maintenance composition");
        RuntimeMaintenanceCommandHandler::new(Arc::new(writer))
    }

    fn fence(&self) -> RuntimeMaintenanceAction {
        RuntimeMaintenanceAction::Fence(FenceRuntimeBindingV1 {
            plan: self.plan.clone(),
        })
    }

    fn inspect(&self) -> RuntimeMaintenanceAction {
        RuntimeMaintenanceAction::Inspect(self.plan.clone())
    }

    fn challenge(
        &self,
        action: RuntimeAdmissionActionV1,
        expected: &RuntimeCasV1,
        service_uid: Option<Uuid>,
    ) -> RuntimeMaintenanceAction {
        RuntimeMaintenanceAction::Challenge {
            plan: self.plan.clone(),
            action,
            expected: expected.clone(),
            service_uid: service_uid.map(|id| id.to_string()),
            revision_uid: None,
        }
    }

    async fn state(&self, db: &DatabaseConnection) -> Value {
        db.query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT jsonb_build_object(
                'business', (SELECT status FROM project_components WHERE id = $2),
                'binding', (SELECT to_jsonb(b) FROM apparatus_runtime_bindings b
                            WHERE project_id = $1 AND binding_id = $2),
                'guard', (SELECT to_jsonb(g) FROM apparatus_runtime_admission_guards g
                          WHERE authority_id = $3 AND release = $4),
                'challenges', (SELECT COALESCE(jsonb_agg(to_jsonb(c) ORDER BY challenge_id), '[]')
                               FROM apparatus_runtime_admission_challenges c
                               WHERE migration_id = $5)
             ) AS state",
            [
                self.plan.project_id.into(),
                self.plan.binding_id.into(),
                self.authority.into(),
                self.plan.release.as_str().into(),
                self.plan.migration_id.into(),
            ],
        ))
        .await
        .expect("read committed cutover state")
        .expect("one cutover state")
        .try_get("", "state")
        .expect("cutover state is JSON")
    }
}

fn digest(digit: char) -> RuntimeDigestV1 {
    RuntimeDigestV1::new(format!("sha256:{}", digit.to_string().repeat(64))).expect("test digest")
}

async fn execute(
    handler: &RuntimeMaintenanceCommandHandler,
    action: RuntimeMaintenanceAction,
) -> Result<RuntimeMaintenanceReply, RuntimeControlErrorV1> {
    handler
        .handle(RuntimeMaintenanceCommand::new(action))
        .await
        .expect("business result is not a command transport error")
}

async fn fenced(handler: &RuntimeMaintenanceCommandHandler, cutover: &Cutover) -> RuntimeCasV1 {
    let RuntimeMaintenanceReply::Cas(cas) = execute(handler, cutover.fence())
        .await
        .expect("commit fence")
    else {
        panic!("fence must return CAS");
    };
    cas
}

async fn issued(
    handler: &RuntimeMaintenanceCommandHandler,
    cutover: &Cutover,
    cas: &RuntimeCasV1,
) -> RuntimeAdmissionChallengeV1 {
    let RuntimeMaintenanceReply::Challenge(challenge) = execute(
        handler,
        cutover.challenge(RuntimeAdmissionActionV1::Prepare, cas, None),
    )
    .await
    .expect("issue Prepare challenge") else {
        panic!("challenge must return its intent");
    };
    challenge
}

#[tokio::test]
#[serial]
async fn maintenance_fence_persists_one_journal_entry_and_inspection_is_read_only() {
    let (fixture, _, _, _, _) = super::common::setup_test_server()
        .await
        .expect("real server");
    let db = fixture.db();
    let cutover = Cutover::arrange(&db, "disabled").await;
    let handler = cutover.handler(&db, None, cutover.installed_at);
    let untouched = cutover.state(&db).await;
    assert_code(
        execute(&handler, cutover.inspect()).await,
        RuntimeErrorCodeV1::RuntimeBindingNotFound,
    );
    assert_eq!(cutover.state(&db).await, untouched);

    let cas = fenced(&handler, &cutover).await;
    assert_eq!(cas.generation, counter(7));
    assert_eq!(cas.grant_revision, counter(3));
    assert_eq!(cas.revocation_epoch, counter(1));
    assert_eq!(cas.projection_version, counter(1));
    let state = cutover.state(&db).await;
    assert_eq!(state["business"], "disabled");
    assert_eq!(state["binding"]["state"], "closed");
    assert_eq!(state["binding"]["cutover_phase"], "fenced");
    assert_eq!(state["guard"]["blocked"], false);
    let journal = state["binding"]["cutover_journal"]
        .as_array()
        .expect("journal");
    assert_eq!(journal.len(), 1);
    assert_eq!(journal[0]["act"], "fence");
    assert_eq!(journal[0]["migration_id"], json!(cutover.plan.migration_id));
    assert_eq!(journal[0]["operator_id"], json!(cutover.operator));

    let replay = cutover.handler(&db, Some(cas.clone()), cutover.installed_at);
    assert!(fenced(&replay, &cutover).await == cas);
    let RuntimeMaintenanceReply::Inspection(RuntimeMaintenanceInspection {
        cas: inspected,
        phase,
        journal_json,
        consumed_challenges,
    }) = execute(&replay, cutover.inspect())
        .await
        .expect("inspect committed fence")
    else {
        panic!("inspection must return the recovery view");
    };
    assert!(inspected == cas);
    assert_eq!(phase.as_deref(), Some("fenced"));
    assert_eq!(
        serde_json::from_str::<Value>(&journal_json).expect("exact journal view"),
        state["binding"]["cutover_journal"]
    );
    assert_eq!(consumed_challenges, [] as [Uuid; 0]);
    assert_eq!(cutover.state(&db).await, state);
}

#[tokio::test]
#[serial]
async fn maintenance_refused_fence_rolls_back_guard_and_binding() {
    let (fixture, _, _, _, _) = super::common::setup_test_server()
        .await
        .expect("real server");
    let db = fixture.db();
    let active = Cutover::arrange(&db, "active").await;
    let before = active.state(&db).await;
    assert_code(
        execute(
            &active.handler(&db, None, active.installed_at),
            active.fence(),
        )
        .await,
        RuntimeErrorCodeV1::RuntimeBindingClosed,
    );
    assert_eq!(active.state(&db).await, before);
    assert!(before["guard"].is_null() && before["binding"].is_null());

    let disabled = Cutover::arrange(&db, "disabled").await;
    let before = disabled.state(&db).await;
    let future = RuntimeUnixMsV1::new(i64::MAX).expect("future receipt time");
    assert_code(
        execute(&disabled.handler(&db, None, future), disabled.fence()).await,
        RuntimeErrorCodeV1::RuntimeInvalidProof,
    );
    assert_eq!(disabled.state(&db).await, before);
    assert!(before["guard"].is_null() && before["binding"].is_null());
}

#[tokio::test]
#[serial]
async fn maintenance_challenge_reissue_invalidates_only_pending_prior_intent() {
    let (fixture, _, _, _, _) = super::common::setup_test_server()
        .await
        .expect("real server");
    let db = fixture.db();
    let cutover = Cutover::arrange(&db, "disabled").await;
    let handler = cutover.handler(&db, None, cutover.installed_at);
    let cas = fenced(&handler, &cutover).await;
    let before = cutover.state(&db).await;
    assert_eq!(before["challenges"], json!([]));
    let first = issued(&handler, &cutover, &cas).await;
    let after_first = cutover.state(&db).await;
    assert_eq!(
        after_first["challenges"]
            .as_array()
            .expect("one challenge")
            .len(),
        1
    );
    assert_eq!(after_first["challenges"][0]["invalidated"], false);

    let mut stale = cas.clone();
    stale.projection_version = counter(0);
    for (action, expected, uid, error) in [
        (
            RuntimeAdmissionActionV1::Prepare,
            &stale,
            None,
            RuntimeErrorCodeV1::RuntimeStaleState,
        ),
        (
            RuntimeAdmissionActionV1::Open,
            &cas,
            None,
            RuntimeErrorCodeV1::RuntimeStaleState,
        ),
        (
            RuntimeAdmissionActionV1::Prepare,
            &cas,
            Some(Uuid::new_v4()),
            RuntimeErrorCodeV1::RuntimeInvalidRequest,
        ),
    ] {
        assert_code(
            execute(&handler, cutover.challenge(action, expected, uid)).await,
            error,
        );
        assert_eq!(cutover.state(&db).await, after_first);
    }

    let second = issued(&handler, &cutover, &cas).await;
    assert_ne!(second.challenge_id, first.challenge_id);
    let after = cutover.state(&db).await;
    for key in ["business", "binding", "guard"] {
        assert_eq!(after[key], before[key]);
    }
    let rows = after["challenges"].as_array().expect("challenge rows");
    assert_eq!(rows.len(), 2);
    for (intent, invalidated) in [(first, true), (second, false)] {
        assert_eq!(intent.authority_id, cutover.authority);
        assert_eq!(intent.installation_id, cutover.installation);
        assert_eq!(intent.migration_id, cutover.plan.migration_id);
        assert!(intent.expected == cas);
        assert!(intent.service_uid.is_none() && intent.revision_uid.is_none());
        assert_eq!(
            intent.expires_at_unix_ms.get() - intent.issued_at_unix_ms.get(),
            ADMISSION_CHALLENGE_TTL_MS
        );
        let row = rows
            .iter()
            .find(|row| row["challenge_id"] == json!(intent.challenge_id))
            .expect("specific challenge persisted");
        assert_eq!(row["action"], "prepare");
        assert_eq!(row["invalidated"], invalidated);
        assert!(row["consumed_at_unix_ms"].is_null());
        assert_eq!(row["intention"], json!(intent));
    }
}
