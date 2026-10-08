//! CORE-C1 real-primary-read proof; signer await ordering is B's unit seam.
mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "../../../workers/ext-authz/tests/support/registry.rs"]
mod registry;
mod utils;
#[path = "support/writer_barrier.rs"]
mod writer_barrier;

use iam_domain::{entity::signing_key::SigningKeyStatus, port::repository::SigningKeyRegistry};
use iam_infra::repository::SeaOrmSigningKeyRegistry;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait};
use serial_test::serial;

fn lifecycle_registry(
    db: std::sync::Arc<sea_orm::DatabaseConnection>,
    ttl: u64,
) -> SeaOrmSigningKeyRegistry {
    SeaOrmSigningKeyRegistry::new(
        db,
        std::sync::Arc::new(
            iam_domain::entity::signing_key::SigningKeyLifecyclePolicy::new().unwrap(),
        ),
        ttl,
    )
    .unwrap()
}

#[tokio::test]
#[serial]
async fn publisher_rematerializes_on_deadline_before_responding() {
    let (fixture, base, client) = common::setup_test_server().await.unwrap();
    fixture_cleanup::run(&fixture,async {
        let (writer,ttl)=common::fixture_signing_registry(&fixture).unwrap();
        let root=writer.find_active_platform_key().await.unwrap().unwrap();
        for dirty in [false,true] {
            let mut key=pending_candidate(uuid::Uuid::new_v4(),"expiry");key.status=SigningKeyStatus::Active;
            writer.insert(&key).await.unwrap();
            let mut retiring=writer.find_by_kid(&key.kid).await.unwrap().unwrap();retiring.status=SigningKeyStatus::Retiring;
            writer.update(&retiring).await.unwrap();
            let cached=writer.jwks_publication_snapshot().await.unwrap();
            assert!(cached.publication.jwks().keys.iter().any(|entry|entry.kid==key.kid));
            assert!(cached.next_expiration.is_some());
            let retention=i64::try_from(ttl).unwrap()+60;
            let tx=fixture.db().begin().await.unwrap();
            // Controlled SQL time arrangement, no sleep or disabled trigger.
            // The stored DTO remains the previously validated Retiring payload,
            // consistent with its explicitly earlier materialization clock. The
            // first case clears dirty intentionally: DEADLINE alone must refresh.
            tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "UPDATE signing_keys SET updated_at=(statement_timestamp()-make_interval(secs=>$2::double precision)-INTERVAL '5 seconds') AT TIME ZONE 'UTC' WHERE id=$1",[key.id.into(),retention.into()])).await.unwrap();
            tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "UPDATE signing_jwks_publication SET as_of=(SELECT (updated_at AT TIME ZONE 'UTC')+make_interval(secs=>$2::double precision)-INTERVAL '5 seconds' FROM signing_keys WHERE id=$1),next_expiration=(SELECT (updated_at AT TIME ZONE 'UTC')+make_interval(secs=>$2::double precision) FROM signing_keys WHERE id=$1),dirty=$3 WHERE singleton=1",[key.id.into(),retention.into(),dirty.into()])).await.unwrap();
            tx.commit().await.unwrap();
            let before=fixture.db().query_one(Statement::from_string(DatabaseBackend::Postgres,"SELECT revision,dirty,payload,as_of,next_expiration,statement_timestamp() AS now FROM signing_jwks_publication WHERE singleton=1")).await.unwrap().unwrap();
            assert_eq!(before.try_get::<i64>("","revision").unwrap() as u64,cached.revision);
            assert_eq!(before.try_get::<bool>("","dirty").unwrap(),dirty);
            let as_of:chrono::DateTime<chrono::FixedOffset>=before.try_get("","as_of").unwrap();
            let deadline:chrono::DateTime<chrono::FixedOffset>=before.try_get("","next_expiration").unwrap();
            let now:chrono::DateTime<chrono::FixedOffset>=before.try_get("","now").unwrap();
            assert!(as_of<deadline && deadline<now,"valid old materialization, expired DB deadline");
            assert_eq!(before.try_get::<String>("","payload").unwrap(),cached.publication.payload());
            // Crucially NO registry snapshot getter between arrangement and HTTP.
            let response=client.get(format!("{base}/.well-known/jwks.json")).send().await.unwrap();assert_eq!(response.status(),200);
            let actual:serde_json::Value=response.json().await.unwrap();
            let expected=iam_domain::entity::token::JwkSet::from_registry_keys_checked(std::slice::from_ref(&root)).unwrap();
            assert_eq!(actual,serde_json::to_value(expected).unwrap(),"expired Retiring must disappear in the handler's FIRST response, not only a later DB getter");
            let after=fixture.db().query_one(Statement::from_string(DatabaseBackend::Postgres,"SELECT revision,dirty,payload,next_expiration,access_token_ttl,retire_skew FROM signing_jwks_publication WHERE singleton=1")).await.unwrap().unwrap();
            assert_eq!(after.try_get::<i64>("","revision").unwrap() as u64,cached.revision+1);assert!(!after.try_get::<bool>("","dirty").unwrap());
            assert!(after.try_get::<Option<chrono::DateTime<chrono::FixedOffset>>>("","next_expiration").unwrap().is_none());
            assert_eq!(after.try_get::<i64>("","access_token_ttl").unwrap() as u64,ttl);assert_eq!(after.try_get::<i64>("","retire_skew").unwrap(),60);
            assert_eq!(serde_json::from_str::<serde_json::Value>(&after.try_get::<String>("","payload").unwrap()).unwrap(),actual);
            let retained=writer.find_by_kid(&key.kid).await.unwrap().unwrap();assert_eq!(retained.status,SigningKeyStatus::Retiring,"expiry must preserve immutable historical rows");
            assert_eq!(writer.find_active_platform_key().await.unwrap().unwrap(),root);
        }
    }).await;
}

#[tokio::test]
#[serial]
async fn initial_migration_up_down_up_removes_scope_objects_in_a_disposable_schema() {
    use iammigration::{Migrator, MigratorTrait, SchemaManager};
    let (fixture, _, _) = common::setup_test_server().await.unwrap();
    fixture_cleanup::run(&fixture, async {
        // A brand-new schema INSIDE a rolled-back transaction. Never call
        // Migrator::down/refresh against the fixture's already migrated public DB.
        let tx = fixture.db().begin().await.unwrap();
        let schema = format!("iam_migration_roundtrip_{}", uuid::Uuid::new_v4().simple());
        tx.execute_unprepared(&format!("CREATE SCHEMA {schema}")).await.unwrap();
        tx.execute_unprepared(&format!("SET LOCAL search_path TO {schema}")).await.unwrap();
        let manager = SchemaManager::new(&tx);
        let mut migrations = Migrator::migrations();
        assert_eq!(migrations.len(), 1, "the flattened initial schema remains the migration authority");
        let migration = migrations.remove(0);
        let absence_sql = || Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT to_regclass($1)::text IS NULL AS scope_absent, to_regclass($2)::text IS NULL AS prepublication_absent, to_regprocedure($3)::text IS NULL AS function_absent, to_regclass($4)::text IS NULL AS public_entries_absent, to_regclass($5)::text IS NULL AS materialized_snapshot_absent, to_regprocedure($6)::text IS NULL AS prepared_invalidation_function_absent",
            [format!("{schema}.signing_scope_epochs").into(), format!("{schema}.signing_key_prepublications").into(), format!("{schema}.iam_advance_signing_scope_epoch()").into(), format!("{schema}.signing_public_entries").into(), format!("{schema}.signing_jwks_publication").into(),format!("{schema}.iam_invalidate_signing_publication()").into()]);
        let tables_sql = || Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT array_agg(c.relname ORDER BY c.relname)::text AS tables FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname=$1 AND c.relkind='r'", [schema.clone().into()]);
        migration.up(&manager).await.unwrap();
        let before = tx.query_one(absence_sql()).await.unwrap().unwrap();
        for field in ["scope_absent", "prepublication_absent", "function_absent", "public_entries_absent", "materialized_snapshot_absent","prepared_invalidation_function_absent"] { assert!(!before.try_get::<bool>("", field).unwrap()); }
        let tables: String = tx.query_one(tables_sql()).await.unwrap().unwrap().try_get("", "tables").unwrap();
        assert!(tables.contains("signing_keys")); assert!(tables.contains("signing_scope_epochs")); assert!(tables.contains("signing_key_prepublications"));
        assert!(tables.contains("signing_public_entries")); assert!(tables.contains("signing_jwks_publication"));
        let initial_snapshot=tx.query_one(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT revision,dirty,payload,access_token_ttl,retire_skew,next_expiration FROM signing_jwks_publication WHERE singleton=1")).await.unwrap().unwrap();
        assert_eq!(initial_snapshot.try_get::<i64>("","revision").unwrap(),0);
        assert!(initial_snapshot.try_get::<bool>("","dirty").unwrap());
        assert_eq!(initial_snapshot.try_get::<String>("","payload").unwrap(),"{\"keys\":[]}");
        assert_eq!(initial_snapshot.try_get::<i64>("","access_token_ttl").unwrap(),0,"unmaterialized policy must not invent TTL900");
        assert_eq!(initial_snapshot.try_get::<i64>("","retire_skew").unwrap(),60);
        assert!(initial_snapshot.try_get::<Option<chrono::DateTime<chrono::FixedOffset>>>("","next_expiration").unwrap().is_none());
        migration.down(&manager).await.unwrap();
        let after = tx.query_one(absence_sql()).await.unwrap().unwrap();
        for field in ["scope_absent", "prepublication_absent", "function_absent", "public_entries_absent", "materialized_snapshot_absent","prepared_invalidation_function_absent"] { assert!(after.try_get::<bool>("", field).unwrap(), "down must remove {field}"); }
        migration.up(&manager).await.unwrap();
        let rebuilt = tx.query_one(absence_sql()).await.unwrap().unwrap();
        for field in ["scope_absent", "prepublication_absent", "function_absent", "public_entries_absent", "materialized_snapshot_absent","prepared_invalidation_function_absent"] { assert!(!rebuilt.try_get::<bool>("", field).unwrap()); }
        assert_eq!(tx.query_one(tables_sql()).await.unwrap().unwrap().try_get::<String>("", "tables").unwrap(), tables);
        let trigger = tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT count(*)::bigint AS count FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname=$1 AND t.tgname='signing_keys_scope_epoch'", [schema.clone().into()])).await.unwrap().unwrap();
        assert_eq!(trigger.try_get::<i64>("", "count").unwrap(), 1);
        let prepared_trigger=tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT count(*)::bigint AS count FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname=$1 AND t.tgname='signing_public_entries_publication_dirty'",[schema.clone().into()])).await.unwrap().unwrap();
        assert_eq!(prepared_trigger.try_get::<i64>("","count").unwrap(),1);
        let indexes=tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT count(*)::bigint AS count FROM pg_indexes WHERE schemaname=$1 AND indexname IN ('signing_keys_slot_publication','signing_keys_slot_owner')",[schema.into()])).await.unwrap().unwrap();
        assert_eq!(indexes.try_get::<i64>("","count").unwrap(),2);
        tx.rollback().await.unwrap(); // cleanup includes the never-persisted schema
    }).await;
}

fn pending_candidate(org: uuid::Uuid, label: &str) -> iam_domain::entity::signing_key::SigningKey {
    let mut key = registry::registry_key(SigningKeyStatus::Pending, Some(org));
    key.kid = iam_domain::entity::signing_key::opaque_kid();
    key.provider_key_ref = format!("{org}/{label}.pem");
    key.public_key = include_str!("../config/keys/test-platform.pub").into();
    key
}

fn cryptographic_probe() -> iam_infra::signing::ProviderBindingProbe {
    iam_infra::signing::ProviderBindingProbe(std::sync::Arc::new(
        iam_infra::signing::PemSigningProvider::new(
            include_str!("../config/keys/test-platform.pem"),
            include_str!("../config/keys/test-platform.pub"),
        )
        .unwrap(),
    ))
}

fn require_pending(
    preparation: iam_domain::entity::signing_key::SigningKeyPreparation,
) -> iam_domain::entity::signing_key::PreparedSigningTransition {
    match preparation {
        iam_domain::entity::signing_key::SigningKeyPreparation::Pending(pending) => pending,
        iam_domain::entity::signing_key::SigningKeyPreparation::Unchanged(_) => {
            panic!("new binding must be durably prepared, not a no-op")
        }
    }
}

fn assert_epoch_conflict(error: &iam_domain::error::DomainError) {
    assert!(
        matches!(
            error,
            iam_domain::error::DomainError::SigningKeyAdmissionDenied {
                reason: iam_domain::entity::signing_key::SigningKeyAdmissionReason::EpochConflict,
                ..
            }
        ),
        "expected a scope CAS conflict, got {error:?}"
    );
}

#[tokio::test]
#[serial]
async fn pending_publication_is_durable_resume_is_idempotent_and_promotion_preserves_admission_history(
) {
    use iam_domain::{
        entity::signing_key::{SigningKeyPreparation, SigningScope},
        port::OrganizationSignerProbe,
    };
    let (fixture, _, _) = common::setup_test_server().await.unwrap();
    fixture_cleanup::run(&fixture, async {
        let primary = lifecycle_registry(fixture.db(), 173);
        let org = uuid::Uuid::new_v4();
        let scope = SigningScope::organization(org);
        let mut active = pending_candidate(org, "old"); active.status = SigningKeyStatus::Active;
        primary.insert(&active).await.unwrap();
        let before = primary.signing_scope_snapshot(&scope).await.unwrap();
        let candidate = pending_candidate(org, "next");
        let proof = cryptographic_probe().prove(candidate.clone(), before).await.unwrap();
        let prepared = require_pending(primary.prepare_signing_key(proof).await.unwrap());
        assert_eq!(prepared.key.status, SigningKeyStatus::Pending);
        assert!(prepared.revision > 0);
        let snapshot = primary.jwks_publication_snapshot().await.unwrap();
        let pending = snapshot.publication.jwks().keys.iter().find(|key| key.kid == candidate.kid).unwrap();
        assert_eq!(pending.status, Some(SigningKeyStatus::Pending));
        assert!(!primary.confirm_active_for_emission(&prepared.key).await.unwrap());
        assert_eq!(primary.find_by_kid(&active.kid).await.unwrap().unwrap().status, SigningKeyStatus::Active);
        let history_sql = || Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT count(*)::bigint AS count, min(lifecycle_admitted_at) AS first, max(lifecycle_admitted_at) AS last FROM signing_keys WHERE organization_id=$1", [org.into()]);
        let old_history = fixture.db().query_one(history_sql()).await.unwrap().unwrap();
        let old_count: i64 = old_history.try_get("", "count").unwrap();
        let old_last: chrono::DateTime<chrono::FixedOffset> = old_history.try_get("", "last").unwrap();
        assert_eq!(old_count, 2);

        // New repository instance reads only committed primary durable evidence.
        let restarted = lifecycle_registry(fixture.db(), 173);
        let recovered = restarted.signing_scope_snapshot(&scope).await.unwrap();
        let durable = recovered.pending.clone().unwrap();
        assert_eq!(durable.key, prepared.key);
        assert_eq!(durable.revision, prepared.revision);
        let resumed = require_pending(restarted.prepare_signing_key(
            cryptographic_probe().prove(candidate.clone(), recovered).await.unwrap()).await.unwrap());
        assert_eq!(resumed.key, prepared.key);
        assert_eq!(resumed.revision, prepared.revision);
        let mut forged = resumed.clone(); forged.revision += 1;
        assert_epoch_conflict(&restarted.promote_signing_key(&forged).await.unwrap_err());
        assert_eq!(restarted.find_by_kid(&active.kid).await.unwrap().unwrap().status, SigningKeyStatus::Active);
        let committed = restarted.promote_signing_key(&resumed).await.unwrap();
        assert_eq!(committed.id, candidate.id);
        assert_eq!(committed.kid, candidate.kid);
        assert_eq!(committed.status, SigningKeyStatus::Active);
        assert!(restarted.confirm_active_for_emission(&committed).await.unwrap());
        assert_eq!(restarted.find_by_kid(&active.kid).await.unwrap().unwrap().status, SigningKeyStatus::Retiring);
        let after = restarted.signing_scope_snapshot(&scope).await.unwrap();
        assert!(after.pending.is_none()); assert!(after.revision > resumed.revision);
        let mut same = candidate; same.id = uuid::Uuid::new_v4(); same.kid = iam_domain::entity::signing_key::opaque_kid();
        let SigningKeyPreparation::Unchanged(noop) = restarted.prepare_signing_key(
            cryptographic_probe().prove(same, after.clone()).await.unwrap()).await.unwrap() else { panic!("identical binding must remain unchanged"); };
        assert_eq!(noop, committed);
        assert_eq!(restarted.signing_scope_snapshot(&scope).await.unwrap().revision, after.revision);
        let history = fixture.db().query_one(history_sql()).await.unwrap().unwrap();
        assert_eq!(history.try_get::<i64>("", "count").unwrap(), old_count);
        assert_eq!(history.try_get::<chrono::DateTime<chrono::FixedOffset>>("", "last").unwrap(), old_last);

        // Non-default TTL173+skew60: exact primary publication filter; never900.
        fixture.db().execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "UPDATE signing_keys SET updated_at=statement_timestamp()-INTERVAL '200 seconds' WHERE id=$1", [active.id.into()])).await.unwrap();
        assert!(restarted.jwks_publication_snapshot().await.unwrap().publication.jwks().keys.iter().any(|key| key.kid == active.kid));
        fixture.db().execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "UPDATE signing_keys SET updated_at=statement_timestamp()-INTERVAL '250 seconds' WHERE id=$1", [active.id.into()])).await.unwrap();
        assert!(!restarted.jwks_publication_snapshot().await.unwrap().publication.jwks().keys.iter().any(|key| key.kid == active.kid));
    }).await;
}

#[tokio::test]
#[serial]
async fn independent_writer_disable_invalidates_delayed_probe_even_for_an_empty_scope() {
    use iam_domain::{entity::signing_key::SigningScope, port::OrganizationSignerProbe};
    struct DelayedProbe {
        entered: std::sync::Arc<tokio::sync::Notify>,
        release: std::sync::Arc<tokio::sync::Notify>,
    }
    #[async_trait::async_trait]
    impl OrganizationSignerProbe for DelayedProbe {
        async fn challenge(
            &self,
            key: &iam_domain::entity::signing_key::SigningKey,
        ) -> Result<(), iam_domain::error::DomainError> {
            self.entered.notify_one();
            self.release.notified().await;
            cryptographic_probe().challenge(key).await
        }
    }
    let (fixture, _, _) = common::setup_test_server().await.unwrap();
    fixture_cleanup::run(&fixture, async {
        let separate =
            std::sync::Arc::new(writer_barrier::independent_writer(fixture.db().as_ref()).await);
        for has_active in [false, true] {
            let primary = std::sync::Arc::new(lifecycle_registry(fixture.db(), 173));
            let org = uuid::Uuid::new_v4();
            let scope = SigningScope::organization(org);
            if has_active {
                let mut key = pending_candidate(org, "old");
                key.status = SigningKeyStatus::Active;
                primary.insert(&key).await.unwrap();
            }
            let before = primary.signing_scope_snapshot(&scope).await.unwrap();
            let candidate = pending_candidate(org, "delayed");
            let entered = std::sync::Arc::new(tokio::sync::Notify::new());
            let release = std::sync::Arc::new(tokio::sync::Notify::new());
            let probe = DelayedProbe {
                entered: entered.clone(),
                release: release.clone(),
            };
            let actor = primary.clone();
            let old_revision = before.revision;
            let continuation = tokio::spawn(async move {
                actor
                    .prepare_signing_key(probe.prove(candidate, before).await?)
                    .await
            });
            entered.notified().await;
            let revoker = lifecycle_registry(separate.clone(), 173);
            // A timeout is a test failure, not a sleep/warmup: detects network-await under SQL locks.
            tokio::time::timeout(
                std::time::Duration::from_secs(5),
                revoker.revoke_signing_scope(&scope),
            )
            .await
            .expect("probe must not hold writer lock")
            .unwrap();
            let disabled = primary.signing_scope_snapshot(&scope).await.unwrap();
            assert!(disabled.revision > old_revision);
            assert!(disabled.active.is_none());
            release.notify_one();
            assert_epoch_conflict(&continuation.await.unwrap().unwrap_err());
            assert!(primary
                .signing_scope_snapshot(&scope)
                .await
                .unwrap()
                .pending
                .is_none());
            assert!(!primary
                .jwks_publication_snapshot()
                .await
                .unwrap()
                .publication
                .jwks()
                .keys
                .iter()
                .any(|key| key.organization_id == Some(org)));
        }
    })
    .await;
}

#[tokio::test]
#[serial]
async fn two_prepared_writers_have_one_cas_winner_and_disable_prevents_pending_resurrection() {
    use iam_domain::{entity::signing_key::SigningScope, port::OrganizationSignerProbe};
    let (fixture, _, _) = common::setup_test_server().await.unwrap();
    fixture_cleanup::run(&fixture, async {
        let primary = lifecycle_registry(fixture.db(), 173);
        let separate =
            std::sync::Arc::new(writer_barrier::independent_writer(fixture.db().as_ref()).await);
        let other = lifecycle_registry(separate, 173);
        let scope = SigningScope::organization(uuid::Uuid::new_v4());
        let org = scope.organization_id.unwrap();
        let before = primary.signing_scope_snapshot(&scope).await.unwrap();
        let a = cryptographic_probe()
            .prove(pending_candidate(org, "a"), before.clone())
            .await
            .unwrap();
        let b = cryptographic_probe()
            .prove(pending_candidate(org, "b"), before)
            .await
            .unwrap();
        let (a, b) = tokio::join!(primary.prepare_signing_key(a), other.prepare_signing_key(b));
        let prepared = match (a, b) {
            (Ok(a), Err(error)) => {
                assert_epoch_conflict(&error);
                require_pending(a)
            }
            (Err(error), Ok(b)) => {
                assert_epoch_conflict(&error);
                require_pending(b)
            }
            _ => panic!("exactly one scope CAS admission must succeed"),
        };
        let mut wrong_binding = prepared.clone();
        wrong_binding.key.provider_key_ref.push_str("-forged");
        assert_epoch_conflict(
            &primary
                .promote_signing_key(&wrong_binding)
                .await
                .unwrap_err(),
        );
        assert_eq!(
            primary
                .find_by_kid(&prepared.key.kid)
                .await
                .unwrap()
                .unwrap()
                .status,
            SigningKeyStatus::Pending
        );
        other.revoke_signing_scope(&scope).await.unwrap();
        assert_epoch_conflict(&primary.promote_signing_key(&prepared).await.unwrap_err());
        let after = primary.signing_scope_snapshot(&scope).await.unwrap();
        assert!(after.revision > prepared.revision);
        assert!(after.active.is_none());
        assert!(after.pending.is_none());
        assert_eq!(
            primary
                .find_by_kid(&prepared.key.kid)
                .await
                .unwrap()
                .unwrap()
                .status,
            SigningKeyStatus::Revoked
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn access_and_registration_fences_reject_primary_disable_during_public_or_sign_io() {
    use iam_domain::{
        entity::{
            signing_key::SigningScope,
            token::{JwkSet, JwtKeyPair},
        },
        port::{
            service::{AuthTokenService, RegistrationTokenService},
            OrganizationSignerProbe, SigningCapabilities, SigningProvider,
        },
    };
    use iam_infra::{
        signing::PemSigningProvider,
        token::{JwtTokenService, RegistrationTokenServiceImpl},
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    struct BarrierSigner {
        inner: PemSigningProvider,
        public_barrier: bool,
        entered: tokio::sync::Notify,
        release: tokio::sync::Notify,
        signatures: AtomicUsize,
    }
    #[async_trait::async_trait]
    impl SigningProvider for BarrierSigner {
        async fn public_key(&self) -> Result<String, iam_domain::error::DomainError> {
            if self.public_barrier {
                self.entered.notify_one();
                self.release.notified().await;
            }
            self.inner.public_key().await
        }
        async fn sign_digest(
            &self,
            digest: &[u8],
        ) -> Result<Vec<u8>, iam_domain::error::DomainError> {
            if !self.public_barrier {
                self.entered.notify_one();
                self.release.notified().await;
            }
            self.signatures.fetch_add(1, Ordering::SeqCst);
            self.inner.sign_digest(digest).await
        }
        fn capabilities(&self) -> SigningCapabilities {
            self.inner.capabilities()
        }
    }
    let (fixture, _, _) = common::setup_test_server().await.unwrap();
    fixture_cleanup::run(&fixture, async {
        let primary = Arc::new(lifecycle_registry(fixture.db(), 173));
        let other = lifecycle_registry(Arc::new(writer_barrier::independent_writer(fixture.db().as_ref()).await), 173);
        let scope = SigningScope::platform();
        for registration_flow in 0..3 {
            for public_barrier in [false, true] {
                primary.revoke_signing_scope(&scope).await.unwrap();
                let mut candidate = registry::registry_key(SigningKeyStatus::Pending, None);
                candidate.kid = iam_domain::entity::signing_key::opaque_kid();
                candidate.public_key = include_str!("../config/keys/test-platform.pub").into();
                let before = primary.signing_scope_snapshot(&scope).await.unwrap();
                let pending = require_pending(primary.prepare_signing_key(cryptographic_probe().prove(candidate, before).await.unwrap()).await.unwrap());
                let active = primary.promote_signing_key(&pending).await.unwrap();
                let signer = Arc::new(BarrierSigner { inner: PemSigningProvider::new(include_str!("../config/keys/test-platform.pem"), active.public_key.clone()).unwrap(), public_barrier,
                    entered: tokio::sync::Notify::new(), release: tokio::sync::Notify::new(), signatures: AtomicUsize::new(0) });
                // This isolated fixture uses an explicitly provisioned PEM signer.
                // It must reach the I/O barrier rather than fail the production PEM guard.
                let codec = Arc::new(JwtTokenService::with_rsa(JwtKeyPair { private_key: String::new(), public_key: active.public_key.clone(), kid: "stale-boot-kid".into() }, 173)
                    .with_local_pem_allowed(true).with_issuer_audience(&active.issuer, "aiforall")
                    .with_signing_provider(signer.clone(), "stale-boot-kid", &active.issuer, JwkSet { keys: vec![] })
                    .with_signing_registry(primary.clone()));
                let registration = RegistrationTokenServiceImpl::new(codec.clone()).unwrap();
                let issuance = async {
                    let user = uuid::Uuid::new_v4();
                    match registration_flow {
                        0 => codec.generate_access_token(user).await.map(|token| token.token).map_err(|error| error.to_string()),
                        1 => registration.generate_registration_token(user, "fixture@example.test".into()).await.map_err(|error| error.to_string()),
                        _ => registration.generate_oauth_registration_token(user, "fixture@example.test".into(), iam_domain::entity::registration_token::ProviderInfo {
                            email: "fixture@example.test".into(), avatar: None, suggested_username: "fixture".into() }).await.map_err(|error| error.to_string()),
                    }
                };
                let result = tokio::time::timeout(std::time::Duration::from_secs(10), async {
                    tokio::pin!(issuance);
                    // An early issuance error must fail the test, not leave the
                    // transition waiting forever for an unreachable callback.
                    tokio::select! {
                        result = &mut issuance => {
                            let error = result.expect_err("no token may escape before the signer barrier");
                            panic!("issuance failed before signer barrier (flow={registration_flow}, public_barrier={public_barrier}): {error}");
                        }
                        () = signer.entered.notified() => {}
                    }
                    tokio::time::timeout(std::time::Duration::from_secs(5), other.revoke_signing_scope(&scope)).await.expect("emission I/O must not hold writer locks").unwrap();
                    assert!(!primary.confirm_active_for_emission(&active).await.unwrap());
                    signer.release.notify_one();
                    issuance.await
                }).await.expect("signer barrier and final emission fence must complete within 10 seconds");
                assert!(result.is_err(), "neither access nor either registration flow may emit a disabled primary binding");
                assert_eq!(signer.signatures.load(Ordering::SeqCst), 1, "a valid candidate reaches the final writer fence");
                assert!(primary.signing_scope_snapshot(&scope).await.unwrap().active.is_none());
            }
        }
    }).await;
}

#[tokio::test]
#[serial]
async fn emission_fence_reads_fresh_committed_primary_state_not_the_initial_active_object() {
    let mut key = registry::registry_key(SigningKeyStatus::Active, None);
    key.kid = iam_domain::entity::signing_key::opaque_kid();
    let (fixture, _, _) = common::setup_test_server_with_signing_keys(&[key.clone()])
        .await
        .expect("writer registry fixture");
    fixture_cleanup::run(&fixture, async {
    let db = fixture.db();
    let primary = SeaOrmSigningKeyRegistry::new(
        db.clone(),
        std::sync::Arc::new(
            iam_domain::entity::signing_key::SigningKeyLifecyclePolicy::new()
                .expect("valid signing lifecycle policy"),
        ),
        iam_configuration::load_config_part::<iam_configuration::JwtConfig>("jwt")
            .expect("fixture JWT config")
            .expiration_seconds,
    )
    .expect("valid writer registry");
    let initial = primary
        .find_by_kid(&key.kid)
        .await
        .expect("initial primary read")
        .expect("Active row");
    assert!(primary
        .confirm_active_for_emission(&initial)
        .await
        .expect("positive fence"));
    let separate = writer_barrier::independent_writer(db.as_ref()).await;
    for status in ["retiring", "revoked"] {
        let transition = separate
            .begin()
            .await
            .expect("independent transition transaction");
        let result = transition
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE signing_keys SET status=$2 WHERE id=$1",
                [initial.id.into(), status.into()],
            ))
            .await
            .expect("transition");
        assert_eq!(result.rows_affected(), 1);
        transition
            .commit()
            .await
            .expect("transition commits before fence L");
        assert!(
            initial.status == SigningKeyStatus::Active,
            "initial snapshot deliberately remains stale"
        );
        assert!(
            !primary
                .confirm_active_for_emission(&initial)
                .await
                .expect("fresh post-transition fence"),
            "committed primary transition must override the initial Active object"
        );
    }
    separate.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE signing_keys SET status='active',credential_ref='changed-test-binding' WHERE id=$1", [initial.id.into()]))
        .await.expect("binding transition");
    assert!(!primary
        .confirm_active_for_emission(&initial)
        .await
        .expect("binding fence"));
    separate
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE signing_keys SET credential_ref=NULL WHERE id=$1",
            [initial.id.into()],
        ))
        .await
        .expect("restore exact binding");
    assert!(primary
        .confirm_active_for_emission(&initial)
        .await
        .expect("exact Active epoch recovery"));
    // Arrange the missing-row fault atomically while respecting the prepared
    // public entry's FK. Keep triggers enabled and preserve the absent-row fence.
    let removal = separate.begin().await.expect("independent removal transaction");
    let prepared = removal.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "DELETE FROM signing_public_entries WHERE signing_key_id=$1",
        [initial.id.into()],
    )).await.expect("remove prepared public entry");
    assert_eq!(prepared.rows_affected(), 1);
    let removed = removal
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "DELETE FROM signing_keys WHERE id=$1",
            [initial.id.into()],
        ))
        .await
        .expect("remove primary epoch");
    assert_eq!(removed.rows_affected(), 1);
    removal.commit().await.expect("removal commits before absent-row fence");
    assert!(!primary
        .confirm_active_for_emission(&initial)
        .await
        .expect("absent row fence"));
    }).await;
}
