//! Exercise real admission proof verification and transactional consumption
//! with a process-only key and a typed maintenance facts fixture.

use super::{assert_code, counter, digest, execute, fenced, issued, now, Cutover};
use crate::{caller, lease, observation};
use apparatus_contracts::{
    digest::{canonical_json, digest_bytes},
    runtime::{
        runtime_serving_name, OpenRuntimeBindingV1, PrepareRuntimeBindingV1,
        RuntimeAdmissionActionV1, RuntimeAdmissionAttestationV1, RuntimeAdmissionChallengeV1,
        RuntimeBindingPreparationPermitV1, RuntimeCasV1, RuntimeDigestV1, RuntimeErrorCodeV1,
        RuntimeObservationV1, RuntimeServiceRoleV1, RuntimeUnixMsV1, ADMISSION_AUDIENCE,
        ADMISSION_ISSUER, ADMISSION_PROOF_TTL_MS, ADMISSION_PURPOSE, ADMISSION_TYP,
    },
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use manifesto_application::{
    RuntimeAdmissionClosureAck, RuntimeMaintenanceAction as Action,
    RuntimeMaintenanceCommandHandler, RuntimeMaintenanceReply,
};
use manifesto_infra::apparatus_runtime::{
    admission_proof::{PinnedRuntimeAdmissionKey, PinnedRuntimeAdmissionProofVerifier},
    runtime_control::SqlRuntimeControl,
};
use rsa::{
    pkcs1v15::SigningKey,
    sha2::{Digest as _, Sha256},
    signature::{SignatureEncoding as _, Signer as _},
    traits::PublicKeyParts as _,
    RsaPrivateKey, RsaPublicKey,
};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serde_json::{json, Value};
use serial_test::serial;
use std::sync::{Arc, OnceLock};
use uuid::Uuid;

const ENVELOPE: &str = "fixture.example.test/runtime/envelopes";
const IMAGE: &str = "fixture.example.test/runtime/images";

fn private_key() -> &'static RsaPrivateKey {
    static KEY: OnceLock<RsaPrivateKey> = OnceLock::new();
    KEY.get_or_init(|| RsaPrivateKey::new(&mut rand::rngs::OsRng, 3072).expect("fixture RSA key"))
}

struct Admission {
    kid: String,
    signing_key: SigningKey<Sha256>,
    verifier: Arc<PinnedRuntimeAdmissionProofVerifier>,
    record: Uuid,
}

impl Admission {
    fn new(cutover: &Cutover) -> Self {
        let key = private_key();
        let public = RsaPublicKey::from(key);
        let n = URL_SAFE_NO_PAD.encode(public.n().to_bytes_be());
        let thumbprint = format!(r#"{{"e":"AQAB","kty":"RSA","n":"{n}"}}"#);
        let kid = format!(
            "ara-v1-{}",
            URL_SAFE_NO_PAD.encode(Sha256::digest(thumbprint))
        );
        let verifier = PinnedRuntimeAdmissionProofVerifier::new(
            vec![PinnedRuntimeAdmissionKey {
                authority_id: cutover.authority,
                installation_id: cutover.installation,
                public_key: public,
            }],
            vec![ENVELOPE.to_owned(), IMAGE.to_owned()],
        )
        .expect("pin only the test authority's public key");
        Self {
            kid,
            signing_key: SigningKey::new(key.clone()),
            verifier: Arc::new(verifier),
            record: Uuid::new_v4(),
        }
    }

    fn handler(
        &self,
        cutover: &Cutover,
        db: &Arc<DatabaseConnection>,
    ) -> RuntimeMaintenanceCommandHandler {
        cutover.handler_with_proof(
            db,
            None,
            cutover.installed_at,
            Some(digest('d')),
            self.verifier.clone(),
        )
    }

    async fn claims(
        &self,
        db: &DatabaseConnection,
        challenge: &RuntimeAdmissionChallengeV1,
        revision: u64,
    ) -> RuntimeAdmissionAttestationV1 {
        let observed = now(db).await;
        let valid_until = RuntimeUnixMsV1::new(
            (observed.get() + ADMISSION_PROOF_TTL_MS).min(challenge.expires_at_unix_ms.get()),
        )
        .expect("bounded proof deadline");
        RuntimeAdmissionAttestationV1 {
            schema_version: 1,
            iss: ADMISSION_ISSUER.to_owned(),
            aud: ADMISSION_AUDIENCE.to_owned(),
            purpose: ADMISSION_PURPOSE.to_owned(),
            jti: challenge.challenge_id,
            iat: observed.get() / 1000,
            exp: valid_until.get() / 1000,
            challenge: challenge.clone(),
            record_id: self.record,
            admission_revision: counter(revision),
            policy_id: "apparatus-p4-conformance/v1".to_owned(),
            report_sha256: digest('e'),
            envelope_reference: format!("{ENVELOPE}@{}", digest('f').as_str()),
            cri_image: format!("{IMAGE}@{}", challenge.release.as_str()),
            observed_at_unix_ms: observed,
            valid_until_unix_ms: valid_until,
        }
    }

    fn sign(&self, claims: &RuntimeAdmissionAttestationV1) -> String {
        let header = json!({"alg":"RS256", "typ":ADMISSION_TYP, "kid":self.kid});
        // These fixtures use only ASCII member names/strings and safe integers,
        // so sorted compact JSON is also the required JCS representation.
        let compact_part = |value: &Value| {
            URL_SAFE_NO_PAD.encode(canonical_json(value).expect("canonical fixture JSON"))
        };
        let payload = serde_json::to_value(claims).expect("fixture claims");
        let signed = format!("{}.{}", compact_part(&header), compact_part(&payload));
        let signature = self.signing_key.sign(signed.as_bytes());
        format!("{signed}.{}", URL_SAFE_NO_PAD.encode(signature.to_bytes()))
    }
}

fn new_release() -> RuntimeDigestV1 {
    RuntimeDigestV1::new(digest_bytes(Uuid::new_v4().as_bytes()).to_string())
        .expect("independent test release")
}

async fn consent(db: &DatabaseConnection, cutover: &Cutover) {
    let capabilities: Vec<String> = serde_json::from_str(include_str!(
        "../fixtures/runtime_maintenance_capabilities.json"
    ))
    .expect("maintenance capability fixture");
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE apparatus_bindings SET declared_capabilities=$2 WHERE component_id=$1",
        [cutover.plan.binding_id.into(), json!(capabilities).into()],
    ))
    .await
    .expect("declare runtime invocation");
    let revision =
        i64::try_from(cutover.plan.expected_grant_revision.get()).expect("fixture grant revision");
    for capability in capabilities {
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO apparatus_capability_consents (component_id,capability,status,grant_revision)
             VALUES ($1,$2,'consented',$3)",
            [cutover.plan.binding_id.into(), capability.into(), revision.into()],
        ))
        .await
        .expect("grant runtime invocation");
    }
}

fn prepare(
    cutover: &Cutover,
    expected: &RuntimeCasV1,
    challenge_id: Uuid,
    proof: String,
) -> Action {
    Action::Prepare {
        plan: cutover.plan.clone(),
        command: PrepareRuntimeBindingV1 {
            schema_version: 1,
            migration_id: cutover.plan.migration_id,
            project_id: cutover.plan.project_id,
            binding_id: cutover.plan.binding_id,
            expected: expected.clone(),
            drain_evidence_sha256: digest('d'),
        },
        challenge_id,
        proof,
    }
}

fn resume(action: Action) -> Action {
    let Action::Prepare {
        plan,
        command,
        challenge_id,
        proof,
    } = action
    else {
        panic!("resume requires a prepared command fixture");
    };
    Action::ResumePrepare {
        plan,
        command,
        challenge_id,
        proof,
    }
}

fn block(cutover: &Cutover, record_id: Uuid, revision: u64) -> Action {
    Action::BlockRelease {
        plan: cutover.plan.clone(),
        record_id,
        revision: counter(revision),
    }
}

async fn commit_block(
    handler: &RuntimeMaintenanceCommandHandler,
    cutover: &Cutover,
    admission: &Admission,
) -> RuntimeAdmissionClosureAck {
    let RuntimeMaintenanceReply::Closure(ack) =
        execute(handler, block(cutover, admission.record, 2))
            .await
            .expect("commit release block")
    else {
        panic!("expected release closure acknowledgement")
    };
    assert_eq!(ack.authority_id, cutover.authority);
    assert_eq!(ack.release, cutover.plan.release);
    assert_eq!(ack.record_id, admission.record);
    assert_eq!(ack.maximum_revision, counter(2));
    ack
}

async fn permit(
    handler: &RuntimeMaintenanceCommandHandler,
    action: Action,
) -> RuntimeBindingPreparationPermitV1 {
    let RuntimeMaintenanceReply::Permit(value) =
        execute(handler, action).await.expect("commit Prepare")
    else {
        panic!("Prepare should return a permit");
    };
    value
}

async fn admission_journal(db: &DatabaseConnection, cutover: &Cutover) -> Value {
    db.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT COALESCE(jsonb_agg(jsonb_build_object('challenge', j.challenge_id,
             'fingerprint', encode(j.jws_sha256,'hex'), 'record', j.record_id,
             'revision', j.admission_revision)
             ORDER BY j.admission_revision, c.action DESC), '[]') AS history
         FROM apparatus_runtime_admission_journal j
         JOIN apparatus_runtime_admission_challenges c USING (challenge_id)
         WHERE c.migration_id = $1",
        [cutover.plan.migration_id.into()],
    ))
    .await
    .expect("read proof journal")
    .expect("one proof history row")
    .try_get("", "history")
    .expect("JSON proof history")
}

fn journal_entry(proof: &str, claims: &RuntimeAdmissionAttestationV1) -> Value {
    json!({
        "challenge": claims.jti,
        "fingerprint": format!("{:x}", Sha256::digest(proof)),
        "record": claims.record_id,
        "revision": claims.admission_revision.get()
    })
}

fn open_intent(
    cutover: &Cutover,
    cas: &RuntimeCasV1,
    observation: Option<&RuntimeObservationV1>,
) -> Action {
    Action::Challenge {
        plan: cutover.plan.clone(),
        action: RuntimeAdmissionActionV1::Open,
        expected: cas.clone(),
        service_uid: observation.map(|entry| entry.service_uid.clone()),
        revision_uid: observation.map(|entry| entry.revision_uid.clone()),
    }
}

fn open(cutover: &Cutover, challenge: &RuntimeAdmissionChallengeV1, proof: String) -> Action {
    Action::Open {
        plan: cutover.plan.clone(),
        command: OpenRuntimeBindingV1 {
            schema_version: 1,
            migration_id: cutover.plan.migration_id,
            project_id: cutover.plan.project_id,
            binding_id: cutover.plan.binding_id,
            expected: challenge.expected.clone(),
            service_uid: challenge.service_uid.clone().expect("Open service UID"),
            revision_uid: challenge.revision_uid.clone().expect("Open revision UID"),
            profile_lock_sha256: challenge.profile_lock_sha256.clone(),
        },
        challenge_id: challenge.challenge_id,
        proof,
    }
}

async fn project_for_open(
    db: &Arc<DatabaseConnection>,
    cutover: &Cutover,
    handler: &RuntimeMaintenanceCommandHandler,
    prepared: RuntimeCasV1,
) -> RuntimeAdmissionChallengeV1 {
    let writer = SqlRuntimeControl::new(db.clone());
    let plan = &cutover.plan;
    let acquired = lease(&writer, plan.project_id, plan.binding_id, prepared.clone()).await;
    let mut seen = observation(db, &acquired, prepared).await;
    seen.release = plan.release.clone();
    "https://wrong.example.test/".clone_into(&mut seen.endpoint);
    let projector = caller(RuntimeServiceRoleV1::Projector);
    let wrong_host = writer
        .report_observation(&projector, plan.project_id, plan.binding_id, &seen)
        .await
        .expect("persist an observation unsuitable for Open");
    let state = cutover.state(db).await;
    assert_code(
        execute(handler, open_intent(cutover, &wrong_host.cas, Some(&seen))).await,
        RuntimeErrorCodeV1::RuntimeProjectionUnavailable,
    );
    assert_eq!(cutover.state(db).await, state);

    seen.endpoint = format!(
        "https://{}.apparatus-plugins.svc.cluster.local/",
        runtime_serving_name(plan.project_id, plan.binding_id)
    );
    seen.guard.expected = wrong_host.cas;
    seen.observed_at_unix_ms = now(db).await;
    let right_host = writer
        .report_observation(&projector, plan.project_id, plan.binding_id, &seen)
        .await
        .expect("persist route matching the locked profile");
    let RuntimeMaintenanceReply::Challenge(challenge) =
        execute(handler, open_intent(cutover, &right_host.cas, Some(&seen)))
            .await
            .expect("Open intent")
    else {
        panic!("expected bound Open challenge")
    };
    assert_eq!(
        challenge.service_uid.as_deref(),
        Some(seen.service_uid.as_str())
    );
    assert_eq!(
        challenge.revision_uid.as_deref(),
        Some(seen.revision_uid.as_str())
    );
    challenge
}

async fn prepare_with_consent(
    db: &Arc<DatabaseConnection>,
    cutover: &Cutover,
    admission: &Admission,
    handler: &RuntimeMaintenanceCommandHandler,
) -> RuntimeBindingPreparationPermitV1 {
    let cas = fenced(handler, cutover).await;
    let old = issued(handler, cutover, &cas).await;
    let challenge = issued(handler, cutover, &cas).await;
    let baseline = cutover.state(db).await;
    let missing_consent = prepare(cutover, &cas, challenge.challenge_id, String::new());
    assert_code(
        execute(handler, missing_consent).await,
        RuntimeErrorCodeV1::RuntimeCallerForbidden,
    );
    assert_eq!(cutover.state(db).await, baseline);
    consent(db, cutover).await;

    for (id, signed_challenge, expected) in [
        (old.challenge_id, &old, RuntimeErrorCodeV1::RuntimeReplay),
        (
            challenge.challenge_id,
            &old,
            RuntimeErrorCodeV1::RuntimeInvalidProof,
        ),
    ] {
        let claims = admission.claims(db, signed_challenge, 1).await;
        let action = prepare(cutover, &cas, id, admission.sign(&claims));
        assert_code(execute(handler, action).await, expected);
        assert_eq!(cutover.state(db).await, baseline);
    }
    assert_eq!(admission_journal(db, cutover).await, json!([]));
    let claims = admission.claims(db, &challenge, 1).await;
    let proof = admission.sign(&claims);
    let expected = journal_entry(&proof, &claims);
    let prepared = permit(
        handler,
        prepare(cutover, &cas, challenge.challenge_id, proof),
    )
    .await;
    assert_eq!(prepared.binding_id, cutover.plan.binding_id);
    assert_eq!(prepared.cas.projection_version, counter(2));
    assert_eq!(prepared.cas.revocation_epoch, cas.revocation_epoch);
    assert_eq!(admission_journal(db, cutover).await, json!([expected]));
    let state = cutover.state(db).await;
    assert_eq!(state["business"], "disabled");
    assert_eq!(state["binding"]["cutover_phase"], "prepared");
    assert_eq!(state["binding"]["state"], "closed");
    prepared
}

#[tokio::test]
#[serial]
async fn signed_open_activates_the_component_and_release_block_disables_it() {
    let (fixture, _, _, _, _) = crate::common::setup_test_server()
        .await
        .expect("real server");
    let db = fixture.db();
    let cutover = Cutover::arrange_release(&db, "disabled", new_release()).await;
    let admission = Admission::new(&cutover);
    let handler = admission.handler(&cutover, &db);
    let prepared = prepare_with_consent(&db, &cutover, &admission, &handler).await;
    assert_code(
        execute(&handler, open_intent(&cutover, &prepared.cas, None)).await,
        RuntimeErrorCodeV1::RuntimeInvalidRequest,
    );
    let challenge = project_for_open(&db, &cutover, &handler, prepared.cas).await;
    let mut claims = admission.claims(&db, &challenge, 1).await;
    claims.report_sha256 = digest('9');
    let before = cutover.state(&db).await;
    let journal = admission_journal(&db, &cutover).await;
    assert_code(
        execute(
            &handler,
            open(&cutover, &challenge, admission.sign(&claims)),
        )
        .await,
        RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted,
    );
    assert_eq!(cutover.state(&db).await, before);
    assert_eq!(admission_journal(&db, &cutover).await, journal);
    assert!(before["activated_at"].is_null());
    assert!(before["disabled_at"].is_null());

    let claims = admission.claims(&db, &challenge, 1).await;
    let proof = admission.sign(&claims);
    let expected = journal_entry(&proof, &claims);
    let RuntimeMaintenanceReply::Cas(opened) =
        execute(&handler, open(&cutover, &challenge, proof.clone()))
            .await
            .expect("commit Open against migrated component schema")
    else {
        panic!("Open should return a CAS")
    };
    assert_eq!(
        opened.projection_version,
        counter(challenge.expected.projection_version.get() + 1)
    );
    assert_eq!(opened.revocation_epoch, challenge.expected.revocation_epoch);
    let state = cutover.state(&db).await;
    assert_eq!(state["business"], "active");
    assert!(state["activated_at"].is_string());
    assert!(state["disabled_at"].is_null());
    assert_eq!(state["binding"]["state"], "active");
    assert_eq!(state["binding"]["cutover_phase"], "opened");
    assert_eq!(
        state["binding"]["cutover_journal"][2]["challenge_id"],
        json!(challenge.challenge_id)
    );
    let proof_history = json!([journal[0], expected]);
    assert_eq!(admission_journal(&db, &cutover).await, proof_history);
    assert_code(
        execute(&handler, open(&cutover, &challenge, proof)).await,
        RuntimeErrorCodeV1::RuntimeBindingClosed,
    );
    assert_eq!(cutover.state(&db).await, state);

    let ack = commit_block(&handler, &cutover, &admission).await;
    assert_eq!(
        ack.closed_bindings,
        [(cutover.plan.project_id, cutover.plan.binding_id)]
    );
    let closed = cutover.state(&db).await;
    assert_eq!(closed["business"], "disabled");
    assert_eq!(closed["activated_at"], state["activated_at"]);
    assert!(closed["disabled_at"].is_string());
    assert_eq!(closed["binding"]["state"], "closed");
    assert_eq!(closed["binding"]["cutover_phase"], "fenced");
    assert!(closed["binding"]["lease_id"].is_null());
    assert_eq!(
        closed["binding"]["revocation_epoch"],
        opened.revocation_epoch.get() + 1
    );
    assert_eq!(admission_journal(&db, &cutover).await, proof_history);
    commit_block(&handler, &cutover, &admission).await;
    assert_eq!(cutover.state(&db).await, closed);
}

async fn assert_blocked_admission(
    db: &Arc<DatabaseConnection>,
    cutover: &Cutover,
    admission: &Admission,
    handler: &RuntimeMaintenanceCommandHandler,
    cas: &RuntimeCasV1,
) {
    let challenge = issued(handler, cutover, cas).await;
    let unchanged = cutover.state(db).await;
    for (revision, explicit_resume) in [(2, true), (3, false)] {
        let claims = admission.claims(db, &challenge, revision).await;
        let action = prepare(
            cutover,
            cas,
            challenge.challenge_id,
            admission.sign(&claims),
        );
        let action = if explicit_resume {
            resume(action)
        } else {
            action
        };
        assert_code(
            execute(handler, action).await,
            RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted,
        );
        assert_eq!(cutover.state(db).await, unchanged);
    }
    let mut history = admission_journal(db, cutover)
        .await
        .as_array()
        .expect("proof history")
        .clone();
    let claims = admission.claims(db, &challenge, 3).await;
    let proof = admission.sign(&claims);
    let entry = journal_entry(&proof, &claims);
    let action = prepare(cutover, cas, challenge.challenge_id, proof);
    let prepared = permit(handler, resume(action)).await;
    assert_eq!(
        prepared.cas.projection_version,
        counter(cas.projection_version.get() + 1)
    );
    let state = cutover.state(db).await;
    assert_eq!(state["guard"]["blocked"], false);
    assert_eq!(state["guard"]["reconciliation_required"], false);
    assert_eq!(state["guard"]["maximum_revision"], 3);
    assert_eq!(state["binding"]["cutover_phase"], "prepared");
    assert_eq!(state["business"], "disabled");
    history.push(entry);
    assert_eq!(admission_journal(db, cutover).await, json!(history));
}

#[tokio::test]
#[serial]
async fn release_block_before_fencing_requires_newer_explicit_resume() {
    let (fixture, _, _, _, _) = crate::common::setup_test_server()
        .await
        .expect("real server");
    let db = fixture.db();
    let cutover = Cutover::arrange_release(&db, "disabled", new_release()).await;
    let admission = Admission::new(&cutover);
    let handler = admission.handler(&cutover, &db);
    consent(&db, &cutover).await;
    let ack = commit_block(&handler, &cutover, &admission).await;
    assert_eq!(ack.closed_bindings, [] as [(Uuid, Uuid); 0]);
    let state = cutover.state(&db).await;
    assert!(state["binding"].is_null());
    assert_eq!(state["challenges"], json!([]));
    assert_eq!(state["guard"]["blocked"], true);
    assert_eq!(state["guard"]["reconciliation_required"], true);
    assert_code(
        execute(&handler, block(&cutover, Uuid::new_v4(), 2)).await,
        RuntimeErrorCodeV1::RuntimeIdReused,
    );
    assert_eq!(cutover.state(&db).await, state);
    let cas = fenced(&handler, &cutover).await;
    assert_eq!(cutover.state(&db).await["guard"], state["guard"]);
    assert_blocked_admission(&db, &cutover, &admission, &handler, &cas).await;
}

#[tokio::test]
#[serial]
async fn release_block_invalidates_pending_open_and_explicit_resume_reprepares() {
    let (fixture, _, _, _, _) = crate::common::setup_test_server()
        .await
        .expect("real server");
    let db = fixture.db();
    let cutover = Cutover::arrange_release(&db, "disabled", new_release()).await;
    let admission = Admission::new(&cutover);
    let handler = admission.handler(&cutover, &db);
    let prepared = prepare_with_consent(&db, &cutover, &admission, &handler).await;
    let pending = project_for_open(&db, &cutover, &handler, prepared.cas).await;
    let ack = commit_block(&handler, &cutover, &admission).await;
    assert_eq!(
        ack.closed_bindings,
        [(cutover.plan.project_id, cutover.plan.binding_id)]
    );
    let state = cutover.state(&db).await;
    let invalidated = state["challenges"]
        .as_array()
        .expect("intent list")
        .iter()
        .find(|row| row["challenge_id"] == json!(pending.challenge_id))
        .expect("pending Open row");
    assert_eq!(invalidated["invalidated"], true);
    assert!(invalidated["consumed_at_unix_ms"].is_null());
    assert!(state["binding"]["lease_id"].is_null());
    assert_eq!(state["guard"]["blocked"], true);
    assert_eq!(state["business"], "disabled");
    assert!(state["disabled_at"].is_null());
    let RuntimeMaintenanceReply::Inspection(blocked) = execute(&handler, cutover.inspect())
        .await
        .expect("inspect blocked binding")
    else {
        panic!("expected inspection after block")
    };
    assert_eq!(blocked.phase.as_deref(), Some("fenced"));
    assert_eq!(
        blocked.cas.revocation_epoch,
        counter(pending.expected.revocation_epoch.get() + 1)
    );
    assert_blocked_admission(&db, &cutover, &admission, &handler, &blocked.cas).await;
}
