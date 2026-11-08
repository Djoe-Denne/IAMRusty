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
            ConsumeOAuthTransaction, ConsumedOAuthTransaction, OAuthOperation, OAuthTransaction,
            OAuthTransactionError,
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

// G-2/G-3: canonical base64url encoding of 32 zero octets, not a live secret.
const AUTHENTICATION_NONCE: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

// G-3: reuse RustyCog's DB/container ownership without starting an IAM app or
// resetting the shared database. The test migrates a fresh owned schema below.
struct UpgradeDatabaseDescriptor;

#[async_trait::async_trait]
impl common::ServiceTestDescriptor<common::TestFixture> for UpgradeDatabaseDescriptor {
    type Config = iam_configuration::AppConfig;

    async fn build_app(
        &self,
        _config: Self::Config,
        _server: iam_configuration::ServerConfig,
    ) -> anyhow::Result<()> {
        anyhow::bail!("upgrade fixture must not build the service app")
    }

    async fn run_app(
        &self,
        _config: Self::Config,
        _server: iam_configuration::ServerConfig,
    ) -> anyhow::Result<()> {
        anyhow::bail!("upgrade fixture must not start the service app")
    }

    async fn run_migrations_up(&self, _db: &sea_orm::DatabaseConnection) -> anyhow::Result<()> {
        // The test invokes the real registered Migrator inside its fresh schema.
        Ok(())
    }

    async fn run_migrations_down(&self, _db: &sea_orm::DatabaseConnection) -> anyhow::Result<()> {
        // TestDatabase calls this unconditionally: never reset to fake a preimage.
        Ok(())
    }

    fn has_db(&self) -> bool {
        true
    }
    fn has_sqs(&self) -> bool {
        false
    }
    fn has_openfga(&self) -> bool {
        false
    }
}

async fn upgrade_snapshot(db: &impl ConnectionTrait, remove_nonce: bool) -> String {
    let transaction_projection = if remove_nonce {
        "to_jsonb(t)-'authentication_nonce'"
    } else {
        "to_jsonb(t)"
    };
    let sql = format!(
        "SELECT jsonb_build_object(
            'transactions',(SELECT jsonb_agg({transaction_projection} ORDER BY id) FROM oauth_transactions t),
            'users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
            'tokens',(SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM provider_tokens p),
            'emails',(SELECT jsonb_agg(to_jsonb(e) ORDER BY id) FROM user_emails e)
        )::text AS snapshot"
    );
    db.query_one(Statement::from_string(DatabaseBackend::Postgres, sql))
        .await
        .expect("G-3: snapshot fixture-owned rows")
        .expect("snapshot row")
        .try_get("", "snapshot")
        .expect("snapshot text")
}

async fn schema_integrity_snapshot(db: &impl ConnectionTrait) -> String {
    let row = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT jsonb_build_object(
            'indexes',(SELECT jsonb_agg(indexdef ORDER BY tablename,indexname) FROM pg_indexes WHERE schemaname=current_schema()),
            'constraints',(SELECT jsonb_agg(jsonb_build_array(c.relname,k.conname,pg_get_constraintdef(k.oid)) ORDER BY c.relname,k.conname)
                FROM pg_constraint k JOIN pg_class c ON c.oid=k.conrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname=current_schema())
        )::text AS snapshot".to_owned()))
        .await.expect("G-3: schema integrity snapshot").expect("schema snapshot row");
    row.try_get("", "snapshot").expect("schema snapshot text")
}

// G-3: exercises registration, not a hand-written ALTER or initial-schema copy.
#[tokio::test]
#[serial]
async fn registered_upgrade_preserves_legacy_transactions_accounts_and_constraints() {
    use iammigration::{Migrator, MigratorTrait};
    let fixture = common::TestFixture::new(Arc::new(UpgradeDatabaseDescriptor))
        .await
        .expect("owned DB fixture without service app or automatic reset");
    fixture_cleanup::run(&fixture, async {
        let db = fixture.db();
        let tx = db.begin().await.expect("owned upgrade schema transaction");
        let schema = format!("test_nonce_upgrade_{}", Uuid::new_v4().simple());
        tx.execute_unprepared(&format!("CREATE SCHEMA {schema}; SET LOCAL search_path TO {schema}"))
            .await.expect("fresh schema, not a reset of an existing database");
        Migrator::up(&tx, Some(1)).await.expect("registered initial migration only");
        let before_column = tx.query_one(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT COUNT(*) AS total FROM information_schema.columns WHERE table_schema=current_schema() AND table_name='oauth_transactions' AND column_name='authentication_nonce'".to_owned()))
            .await.expect("pre-upgrade column query").expect("column count row");
        assert_eq!(before_column.try_get::<i64>("", "total").expect("column count"), 0,
            "G-3: initial migration must remain the real preimage without the nonce column");
        let user = Uuid::new_v4();
        tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO users(id,username) VALUES ($1,'upgrade-fixture-user')", [user.into()]))
            .await.expect("legacy user sentinel");
        tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO provider_tokens(user_id,provider,provider_user_id,access_token,refresh_token) VALUES ($1,'github','000042','upgrade-fixture-access','upgrade-fixture-refresh')", [user.into()]))
            .await.expect("legacy identity/token sentinel");
        tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO user_emails(id,user_id,email,is_primary,is_verified) VALUES ($1,$2,'upgrade-fixture@example.test',true,true)", [Uuid::new_v4().into(), user.into()]))
            .await.expect("legacy email sentinel");
        tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO oauth_transactions(id,nonce_hash,state_hash,browser_nonce_hash,provider,operation,target_user_id,redirect_uri,pkce_verifier,expires_at,consumed_at)
             SELECT gen_random_uuid(),decode(repeat(lpad(to_hex(n),2,'0'),32),'hex'),decode(repeat(lpad(to_hex(n+32),2,'0'),32),'hex'),decode(repeat(lpad(to_hex(n+64),2,'0'),32),'hex'),
                'github',CASE n%3 WHEN 1 THEN 'login' WHEN 2 THEN 'link' ELSE 'relink' END,
                CASE WHEN n%3=1 THEN NULL ELSE $1::uuid END,'https://iam.example.test/callback',
                CASE WHEN n>3 THEN NULL ELSE 'upgrade-fixture-verifier' END,
                NOW()+CASE WHEN n%2=0 THEN INTERVAL '-1 minute' ELSE INTERVAL '5 minutes' END,
                CASE WHEN n>3 THEN NOW() ELSE NULL END FROM generate_series(1,6) n", [user.into()]))
            .await.expect("legacy login/link/relink active/expired/consumed rows");
        let before = upgrade_snapshot(&tx, false).await;
        let integrity = schema_integrity_snapshot(&tx).await;
        Migrator::up(&tx, None).await.expect("apply pending registered additive migration");
        assert!(before == upgrade_snapshot(&tx, true).await,
            "G-3: upgrade must preserve UUIDs, links, tokens, emails and all legacy transaction values");
        assert!(integrity == schema_integrity_snapshot(&tx).await,
            "G-3: upgrade must preserve existing indexes and constraints");
        let column = tx.query_one(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT data_type,is_nullable,column_default FROM information_schema.columns WHERE table_schema=current_schema() AND table_name='oauth_transactions' AND column_name='authentication_nonce'".to_owned()))
            .await.expect("upgraded column metadata").expect("G-3: registered migration added nonce");
        assert_eq!(column.try_get::<String>("", "data_type").expect("type"), "text");
        assert_eq!(column.try_get::<String>("", "is_nullable").expect("nullable"), "YES");
        assert!(column.try_get::<Option<String>>("", "column_default").expect("default").is_none());
        let rows = tx.query_one(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT COUNT(*) AS total,COUNT(*) FILTER (WHERE authentication_nonce IS NULL) AS legacy_null FROM oauth_transactions".to_owned()))
            .await.expect("upgrade rows").expect("count row");
        assert_eq!(rows.try_get::<i64>("", "total").expect("rows"), 6);
        assert_eq!(rows.try_get::<i64>("", "legacy_null").expect("NULL rows"), 6);
        let history = tx.query_one(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT COUNT(*) AS total FROM seaql_migrations".to_owned()))
            .await.expect("real migration history").expect("history count");
        let migration_count = i64::try_from(Migrator::migrations().len()).expect("migration count fits");
        assert!(migration_count >= 2, "G-3: an additive migration must follow the initial schema");
        assert_eq!(history.try_get::<i64>("", "total").expect("applied migrations"), migration_count);
        tx.rollback().await.expect("remove only this fixture-owned transactional schema");
    }).await;
}

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
        authentication_nonce: Some(AUTHENTICATION_NONCE.into()),
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
    let (fixture, _, _) = Box::pin(common::setup_test_server())
        .await
        .expect("Postgres harness");
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
    let (fixture, _, _) = Box::pin(common::setup_test_server())
        .await
        .expect("Postgres harness");
    fixture_cleanup::run(&fixture, async {
    let db = fixture.db();
    let writer = Arc::new(SeaOrmOAuthTransactionWriteRepository::new(db.clone()));
    let record = transaction();
    writer.create(&record).await.expect("real transaction");
    let separate = writer_barrier::independent_writer(db.as_ref()).await;
    let persisted = separate.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT authentication_nonce=$2 AS nonce_matches, pkce_verifier=$3 AS verifier_matches FROM oauth_transactions WHERE id=$1",
        [record.id.into(), AUTHENTICATION_NONCE.into(), record.pkce_verifier.clone().into()]))
        .await.expect("G-2: independent durable BEGIN read").expect("persisted transaction");
    assert!(persisted.try_get::<bool>("", "nonce_matches").expect("durable nonce"));
    assert!(persisted.try_get::<bool>("", "verifier_matches").expect("durable verifier"));
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
    let winner = first.or(second).expect("G-2: exactly one writer result");
    assert_eq!(winner.id, record.id, "G-2: capability comes from the locked row");
    assert!(winner.authentication_nonce == record.authentication_nonce,
        "G-2: winner retains the exact original nonce, not a generated replacement");
    assert!(winner.pkce_verifier == record.pkce_verifier,
        "G-2: winner retains the original verifier");
    let row = separate.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT consumed_at IS NOT NULL AS consumed, pkce_verifier IS NULL AS erased, authentication_nonce IS NULL AS nonce_erased FROM oauth_transactions WHERE id=$1", [record.id.into()]))
        .await.expect("committed consume").expect("transaction row");
    assert!(row.try_get::<bool>("", "consumed").expect("consumed"));
    assert!(row.try_get::<bool>("", "erased").expect("erased"));
    assert!(row.try_get::<bool>("", "nonce_erased").expect("G-2: nonce erased before return"));
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
        [
            expired.id.into(),
            f64::from(
                i32::try_from(expired.expires_at).expect("unix seconds fit i32"),
            )
            .into(),
        ],
    ))
    .await
    .expect("expire real row");
    assert!(writer
        .consume(&consume(&expired))
        .await
        .expect("expired consume")
        .is_none());
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT consumed_at IS NULL AS untouched, pkce_verifier IS NOT NULL AS retained, authentication_nonce=$2 AS nonce_retained FROM oauth_transactions WHERE id=$1", [expired.id.into(), AUTHENTICATION_NONCE.into()]))
        .await.expect("expired state").expect("expired row remains");
    assert!(row.try_get::<bool>("", "untouched").expect("untouched"));
    assert!(row
        .try_get::<bool>("", "retained")
        .expect("retained until purge"));
    assert!(row.try_get::<bool>("", "nonce_retained").expect("G-2: expired nonce retained until cleanup"));
    }).await;
}

// G-2: each mismatch must preserve BOTH secrets and leave a later authentic
// consume possible. A false-positive WHERE predicate would burn the valid row.
#[tokio::test]
#[serial]
async fn mismatched_bindings_preserve_nonce_and_verifier_until_authentic_consume() {
    let (fixture, _, _) = Box::pin(common::setup_test_server())
        .await
        .expect("Postgres harness");
    fixture_cleanup::run(&fixture, async {
        let db = fixture.db();
        let writer = SeaOrmOAuthTransactionWriteRepository::new(db.clone());
        let record = transaction();
        writer.create(&record).await.expect("valid BEGIN row");
        let separate = writer_barrier::independent_writer(db.as_ref()).await;
        for binding in ["nonce", "state", "browser", "provider", "operation", "redirect", "expiry"] {
            let mut input = consume(&record);
            match binding {
                "nonce" => input.nonce_hash[0] ^= 1,
                "state" => input.state_hash[0] ^= 1,
                "browser" => input.browser_nonce_hash[0] ^= 1,
                "provider" => input.provider = Provider::parse_slug("gitlab").expect("provider"),
                "operation" => input.operation = OAuthOperation::Link { user_id: Uuid::new_v4() },
                "redirect" => input.redirect_uri.push_str("?untrusted=1"),
                "expiry" => input.expires_at -= 1,
                _ => unreachable!("fixed binding cases"),
            }
            assert!(writer.consume(&input).await.expect("mismatched consume").is_none(),
                "G-2: mismatched {binding} must not consume");
            let row = separate.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "SELECT consumed_at IS NULL AS untouched, authentication_nonce=$2 AS nonce_retained, pkce_verifier=$3 AS verifier_retained FROM oauth_transactions WHERE id=$1",
                [record.id.into(), AUTHENTICATION_NONCE.into(), record.pkce_verifier.clone().into()]))
                .await.expect("independent mismatch read").expect("row retained");
            for column in ["untouched", "nonce_retained", "verifier_retained"] {
                assert!(row.try_get::<bool>("", column).expect("binding invariant"),
                    "G-2: {binding} mismatch violated {column}");
            }
        }
        assert!(ConsumedOAuthTransaction::consume(&writer, consume(&record)).await.is_ok(),
            "G-2: authentic callback still obtains a capability after all mismatches");
    }).await;
}

// G-3 / contract §4 Q-2: refuse only AFTER committed consume/purge, never
// synthesize a nonce or retain the verifier for another callback.
#[tokio::test]
#[serial]
async fn legacy_and_corrupt_nonce_are_committed_and_purged_before_capability_refusal() {
    let (fixture, _, _) = Box::pin(common::setup_test_server())
        .await
        .expect("Postgres harness");
    fixture_cleanup::run(&fixture, async {
        let db = fixture.db();
        let writer = SeaOrmOAuthTransactionWriteRepository::new(db.clone());
        let separate = writer_barrier::independent_writer(db.as_ref()).await;
        let cases = [
            ("legacy NULL", None),
            ("empty", Some("")),
            ("short", Some("not-a-32-byte-nonce")),
            ("wrong alphabet", Some("+AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")),
            ("padding", Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")),
            ("noncanonical trailing bits", Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAB")),
        ];
        for (case, nonce) in cases {
            let record = transaction();
            writer.create(&record).await.expect("valid BEGIN before fixture corruption");
            // Explicit fixture corruption also supports writers that require a
            // nonce for new BEGINs; legacy NULL is not a new production default.
            db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "UPDATE oauth_transactions SET authentication_nonce=$2 WHERE id=$1",
                [record.id.into(), nonce.map(str::to_owned).into()]))
                .await.expect("arrange legacy/corrupt fixture row");
            let result = ConsumedOAuthTransaction::consume(&writer, consume(&record)).await;
            assert!(matches!(result, Err(OAuthTransactionError::InvalidTransaction)),
                "G-3: {case} must not grant a callback capability");
            let row = separate.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "SELECT consumed_at IS NOT NULL AS consumed, authentication_nonce IS NULL AS nonce_erased, pkce_verifier IS NULL AS verifier_erased FROM oauth_transactions WHERE id=$1",
                [record.id.into()])).await.expect("independent post-refusal read").expect("replay marker retained");
            for column in ["consumed", "nonce_erased", "verifier_erased"] {
                assert!(row.try_get::<bool>("", column).expect("legacy invariant"),
                    "G-3: {case} violated committed {column}");
            }
            assert!(matches!(
                ConsumedOAuthTransaction::consume(&writer, consume(&record)).await,
                Err(OAuthTransactionError::InvalidTransaction)
            ), "G-3: {case} replay must remain denied");
            assert!(writer.consume(&consume(&record)).await.expect("writer replay").is_none(),
                "G-3: {case} leaves no consumable writer row");
        }
    }).await;
}

#[tokio::test]
#[serial]
async fn e4_initial_schema_refuses_two_active_platform_keys() {
    use iammigration::{MigratorTrait, SchemaManager};
    let (fixture, _, _) = Box::pin(common::setup_test_server())
        .await
        .expect("Postgres harness");
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
    let (fixture, _, _) = Box::pin(common::setup_test_server())
        .await
        .expect("Postgres harness");
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
    let (fixture, _, _) = Box::pin(common::setup_test_server())
        .await
        .expect("Postgres harness");
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
    let (fixture, _, _) = Box::pin(common::setup_test_server())
        .await
        .expect("Postgres harness");
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
    // G-3: bounded cleanup removes whole expired rows, but neither strips the
    // valid BEGIN's secrets nor restores the valid consumed row's secrets.
    let row = separate.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT (SELECT authentication_nonce=$3 AND pkce_verifier IS NOT NULL AND consumed_at IS NULL FROM oauth_transactions WHERE id=$1) AS valid_retained,
                (SELECT authentication_nonce IS NULL AND pkce_verifier IS NULL AND consumed_at IS NOT NULL FROM oauth_transactions WHERE id=$2) AS consumed_purged",
        [valid.id.into(), valid_consumed.id.into(), AUTHENTICATION_NONCE.into()]))
        .await.expect("G-3: independent post-cleanup secrets read").expect("valid row states");
    assert!(row.try_get::<bool>("", "valid_retained").expect("G-3: valid secrets retained"));
    assert!(row.try_get::<bool>("", "consumed_purged").expect("G-3: consumed secrets stay erased"));
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
