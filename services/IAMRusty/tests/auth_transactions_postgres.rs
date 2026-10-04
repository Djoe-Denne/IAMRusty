//! E-2..E-5 + CORE-C2: real PostgreSQL, causal locks, actual migration/ports.
mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "support/owned_task.rs"]
mod owned_task;
mod utils;
#[path = "support/writer_barrier.rs"]
mod writer_barrier;

use fixtures::DbFixtures;
use iam_domain::{
    entity::{
        oauth_transaction::{
            ConsumeOAuthTransaction, OAuthOperation, OAuthTransaction, OAuthTransactionError,
        },
        provider::Provider,
        token::RefreshToken,
    },
    port::repository::{
        AuthenticationSessionWriter, OAuthTransactionWriteRepository, RefreshTokenWriteRepository,
    },
};
use iam_infra::repository::{
    authentication_session::SeaOrmAuthenticationSessionWriter,
    oauth_transaction_write::SeaOrmOAuthTransactionWriteRepository,
    refresh_token_write::RefreshTokenWriteRepositoryImpl,
};
use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement, TransactionTrait};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

fn refresh(user: Uuid) -> RefreshToken {
    RefreshToken {
        id: Uuid::new_v4(),
        user_id: user,
        token: Uuid::new_v4().to_string(),
        is_valid: true,
        created_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
    }
}
fn transaction() -> OAuthTransaction {
    let nonce = Uuid::new_v4().to_string();
    let hash = iam_domain::entity::oauth_transaction::hash_oauth_bytes(nonce.as_bytes());
    OAuthTransaction {
        id: Uuid::new_v4(),
        nonce_hash: hash,
        state_hash: hash,
        browser_nonce_hash: hash,
        provider: Provider::parse_slug("github").expect("provider slug"),
        operation: OAuthOperation::Login,
        redirect_uri: "http://127.0.0.1:8081/iam/api/auth/github/callback".into(),
        expires_at: chrono::Utc::now().timestamp() + 600,
        consumed_at: None,
        pkce_verifier: Some("fixed-test-verifier-not-a-live-credential".into()),
    }
}
fn consume(record: &OAuthTransaction) -> ConsumeOAuthTransaction {
    ConsumeOAuthTransaction {
        nonce_hash: record.nonce_hash,
        state_hash: record.state_hash,
        browser_nonce_hash: record.browser_nonce_hash,
        provider: record.provider.clone(),
        operation: record.operation.clone(),
        redirect_uri: record.redirect_uri.clone(),
        expires_at: record.expires_at,
    }
}

#[tokio::test]
#[serial]
async fn e2_same_refresh_rotation_has_one_success_one_record_not_found_and_no_orphan() {
    let (fixture, _, _) = common::setup_test_server().await.expect("Postgres harness");
    fixture_cleanup::run(&fixture, async {
    let db = fixture.db();
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("user");
    let writer = Arc::new(RefreshTokenWriteRepositoryImpl::new(db.clone()));
    let old = refresh(user.id());
    writer
        .create(old.clone())
        .await
        .expect("real initial session");
    let separate = writer_barrier::independent_writer(db.as_ref()).await;
    let (held, holder) = writer_barrier::hold(&separate, "users", user.id()).await;
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let writer = writer.clone();
        let next = refresh(user.id());
        let old_id = old.id;
        tasks.push(owned_task::spawn(
            async move { writer.rotate(old_id, next).await },
        ));
    }
    writer_barrier::wait_for_blocked(db.as_ref(), holder, 2).await;
    held.commit().await.expect("release rotation barrier");
    let first = tasks.remove(0).await.expect("rotation task one");
    let second = tasks.remove(0).await.expect("rotation task two");
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    let (winner, loser) = if first.is_ok() {
        (first, second)
    } else {
        (second, first)
    };
    assert!(matches!(loser, Err(DbErr::RecordNotFound(_))));
    let winner = winner.expect("rotation winner");
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT COUNT(*) AS total, COUNT(*) FILTER (WHERE id=$2 AND is_valid) AS winner, COUNT(*) FILTER (WHERE id=$3) AS old FROM refresh_tokens WHERE user_id=$1",
        [user.id().into(), winner.id.into(), old.id.into()])).await.expect("rotation state").expect("count row");
    assert_eq!(row.try_get::<i64>("", "total").expect("total"), 1);
    assert_eq!(row.try_get::<i64>("", "winner").expect("winner"), 1);
    assert_eq!(row.try_get::<i64>("", "old").expect("old"), 0);
    }).await;
}

#[tokio::test]
#[serial]
async fn e3_consume_uses_writer_lock_one_committed_capability_and_erases_db_verifier() {
    let (fixture, _, _) = common::setup_test_server().await.expect("Postgres harness");
    fixture_cleanup::run(&fixture, async {
    let db = fixture.db();
    let writer = Arc::new(SeaOrmOAuthTransactionWriteRepository::new(db.clone()));
    let record = transaction();
    writer.create(&record).await.expect("real transaction");
    let separate = writer_barrier::independent_writer(db.as_ref()).await;
    let (held, holder) = writer_barrier::hold(&separate, "oauth_transactions", record.id).await;
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let writer = writer.clone();
        let input = consume(&record);
        tasks.push(owned_task::spawn(async move { writer.consume(&input).await }));
    }
    writer_barrier::wait_for_blocked(db.as_ref(), holder, 2).await;
    held.commit().await.expect("release consume barrier");
    let first = tasks
        .remove(0)
        .await
        .expect("consumer one")
        .expect("writer one");
    let second = tasks
        .remove(0)
        .await
        .expect("consumer two")
        .expect("writer two");
    assert_eq!(
        usize::from(first.is_some()) + usize::from(second.is_some()),
        1
    );
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT consumed_at IS NOT NULL AS consumed, pkce_verifier IS NULL AS erased FROM oauth_transactions WHERE id=$1", [record.id.into()]))
        .await.expect("committed consume").expect("transaction row");
    assert!(row.try_get::<bool>("", "consumed").expect("consumed"));
    assert!(row.try_get::<bool>("", "erased").expect("erased"));
    assert!(writer
        .consume(&consume(&record))
        .await
        .expect("replay")
        .is_none());
    let mut expired = transaction();
    writer
        .create(&expired)
        .await
        .expect("transaction before expiry");
    expired.expires_at = chrono::Utc::now().timestamp() - 60;
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE oauth_transactions SET expires_at=to_timestamp($2) WHERE id=$1",
        [expired.id.into(), (expired.expires_at as f64).into()],
    ))
    .await
    .expect("expire real row");
    assert!(writer
        .consume(&consume(&expired))
        .await
        .expect("expired consume")
        .is_none());
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT consumed_at IS NULL AS untouched, pkce_verifier IS NOT NULL AS retained FROM oauth_transactions WHERE id=$1", [expired.id.into()]))
        .await.expect("expired state").expect("expired row remains");
    assert!(row.try_get::<bool>("", "untouched").expect("untouched"));
    assert!(row
        .try_get::<bool>("", "retained")
        .expect("retained until purge"));
    }).await;
}

#[tokio::test]
#[serial]
async fn e4_initial_schema_refuses_two_active_platform_keys() {
    use iammigration::{MigratorTrait, SchemaManager};
    let (fixture, _, _) = common::setup_test_server().await.expect("Postgres harness");
    fixture_cleanup::run(&fixture, async {
    let tx = fixture
        .db()
        .begin()
        .await
        .expect("fixture schema transaction");
    let schema = format!("test_auth_migration_{}", Uuid::new_v4().simple());
    tx.execute_unprepared(&format!("CREATE SCHEMA {schema}; SET LOCAL search_path TO {schema}"))
        .await.expect("isolated initial schema");
    let migration = iammigration::Migrator::migrations()
        .into_iter()
        .find(|m| m.name() == "m20220101_000001_initial_schema")
        .expect("actual migration registered");
    migration.up(&SchemaManager::new(&tx)).await.expect("actual initial schema");
    let result = tx.execute_unprepared("INSERT INTO signing_keys(id,kid,algorithm,trust_scope,issuer,provider_type,provider_key_ref,public_key,status) SELECT gen_random_uuid(), 'test-platform-' || n::text, 'RS256', 'platform', 'https://iam.example.test/iam', 'pem_file', 'test-ref', 'test-public-key', 'active' FROM generate_series(1,2) AS n").await;
    let correct_failure = result
        .as_ref()
        .err()
        .is_some_and(|e| e.to_string().contains("signing_keys_one_active_platform"));
    tx.rollback()
        .await
        .expect("remove fixture-owned schema even on failed migration");
    assert!(
        correct_failure,
        "initial schema must enforce one active platform key, not fail on an unrelated constraint"
    );
    }).await;
}

#[tokio::test]
#[serial]
async fn e4_login_target_check_rejects_real_existing_user_target() {
    let (fixture, _, _) = common::setup_test_server().await.expect("Postgres harness");
    fixture_cleanup::run(&fixture, async {
    let db = fixture.db();
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("existing target");
    let tx = db.begin().await.expect("CHECK transaction");
    let result = tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO oauth_transactions(id,nonce_hash,state_hash,browser_nonce_hash,provider,operation,target_user_id,redirect_uri,expires_at) VALUES ($1,$2,$3,$4,'github','login',$5,'http://127.0.0.1/callback',NOW()+INTERVAL '10 minutes')",
        [Uuid::new_v4().into(), vec![1u8;32].into(), vec![2u8;32].into(), vec![3u8;32].into(), user.id().into()])).await;
    let check_failure = result
        .as_ref()
        .err()
        .is_some_and(|e| e.to_string().contains("check constraint"));
    tx.rollback().await.expect("rollback invalid fixture row");
    assert!(
        check_failure,
        "Login with target must fail CHECK, not FK or missing schema"
    );
    }).await;
}

#[tokio::test]
#[serial]
async fn e5_stale_registration_password_cannot_write_username_or_refresh() {
    let (fixture, _, _) = common::setup_test_server().await.expect("Postgres harness");
    fixture_cleanup::run(&fixture, async {
    let db = fixture.db();
    let user = DbFixtures::user()
        .password_hash("current-writer-password-proof")
        .commit(db.clone())
        .await
        .expect("incomplete account");
    assert!(user.username().is_none());
    let writer = SeaOrmAuthenticationSessionWriter::new(db.clone());
    let result = writer
        .complete_registration(
            user.id(),
            Some("pre-reset-stale-proof".into()),
            "mustnotpersist".into(),
            Some(refresh(user.id())),
        )
        .await;
    assert!(matches!(
        result,
        Err(iam_domain::error::DomainError::InvalidToken)
    ));
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT username IS NULL AS unchanged, (SELECT COUNT(*) FROM refresh_tokens WHERE user_id=$1) AS sessions FROM users WHERE id=$1", [user.id().into()]))
        .await.expect("registration effects").expect("user row");
    assert!(row.try_get::<bool>("", "unchanged").expect("username"));
    assert_eq!(row.try_get::<i64>("", "sessions").expect("sessions"), 0);
    }).await;
}

#[tokio::test]
#[serial]
async fn cleanup_cap_skip_locked_and_two_replicas_preserve_valid_transactions() {
    let (fixture, _, _) = common::setup_test_server().await.expect("Postgres harness");
    fixture_cleanup::run(&fixture, async {
    let db = fixture.db();
    let writer = Arc::new(SeaOrmOAuthTransactionWriteRepository::new(db.clone()));
    let mut expired_ids = Vec::new();
    for consumed in [false, true, false] {
        let record = transaction();
        writer.create(&record).await.expect("real BEGIN row");
        if consumed {
            assert!(writer
                .consume(&consume(&record))
                .await
                .expect("consume")
                .is_some());
        }
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE oauth_transactions SET expires_at=NOW()-INTERVAL '1 minute' WHERE id=$1",
            [record.id.into()],
        ))
        .await
        .expect("expire fixture row");
        expired_ids.push(record.id);
    }
    let valid = transaction();
    writer.create(&valid).await.expect("valid BEGIN");
    let valid_consumed = transaction();
    writer
        .create(&valid_consumed)
        .await
        .expect("valid consumed BEGIN");
    writer
        .consume(&consume(&valid_consumed))
        .await
        .expect("valid consume");
    for cap in [0, 1001, u32::MAX] {
        assert!(matches!(
            writer.purge_expired(cap).await,
            Err(OAuthTransactionError::InvalidTransaction)
        ));
    }
    let separate = writer_barrier::independent_writer(db.as_ref()).await;
    let (held, _) = writer_barrier::hold(&separate, "oauth_transactions", expired_ids[0]).await;
    let (one, two) = tokio::join!(writer.purge_expired(1), writer.purge_expired(1));
    assert_eq!(
        one.expect("purger one") + two.expect("purger two"),
        2,
        "cooperating bounded purgers must skip the locked expired row"
    );
    held.commit().await.expect("release expired row");
    assert_eq!(writer.purge_expired(1).await.expect("remaining batch"), 1);
    let row = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT COUNT(*) AS total, COUNT(*) FILTER (WHERE expires_at<=clock_timestamp()) AS expired FROM oauth_transactions".to_owned()))
        .await.expect("purge effects").expect("count row");
    assert_eq!(row.try_get::<i64>("", "total").expect("valid rows"), 2);
    assert_eq!(
        row.try_get::<i64>("", "expired")
            .expect("expired rows/verifiers"),
        0
    );
    assert!(writer
        .consume(&consume(&valid))
        .await
        .expect("valid preserved")
        .is_some());
    assert!(writer
        .consume(&consume(&valid_consumed))
        .await
        .expect("valid consumed replay")
        .is_none());
    }).await;
}
