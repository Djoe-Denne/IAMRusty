//! CORE-C2 real PostgreSQL cleanup actor retry/stop/logging protocol.
//! Root's two-handle standalone/monolith supervision is a separate integration gate.
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

use iam_application::background::oauth_transaction_cleanup::{
    OAuthTransactionCleanup, OAuthTransactionCleanupPolicy,
};
use iam_infra::repository::oauth_transaction_write::SeaOrmOAuthTransactionWriteRepository;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait};
use serial_test::serial;
use std::{
    io::Write,
    sync::{Arc, Mutex},
    time::Duration,
};
use tracing::instrument::WithSubscriber;

#[derive(Clone, Default)]
struct Capture {
    bytes: Arc<Mutex<Vec<u8>>>,
    changed: Arc<tokio::sync::Notify>,
}
impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.bytes
            .lock()
            .expect("capture lock")
            .extend_from_slice(bytes);
        self.changed.notify_waiters();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Capture {
    type Writer = Self;
    fn make_writer(&'a self) -> Self {
        self.clone()
    }
}
impl Capture {
    async fn event(&self, event: &str) -> bool {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let notified = self.changed.notified();
                if String::from_utf8_lossy(&self.bytes.lock().expect("capture lock"))
                    .contains(event)
                {
                    break;
                }
                notified.await;
            }
        })
        .await
        .is_ok()
    }
    fn redacted(&self) -> bool {
        !String::from_utf8_lossy(&self.bytes.lock().expect("capture lock")).contains("SentinelABC")
    }
}

async fn expired_row(db: &sea_orm::DatabaseConnection) -> uuid::Uuid {
    let id = uuid::Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO oauth_transactions(id,nonce_hash,state_hash,browser_nonce_hash,provider,operation,redirect_uri,expires_at,pkce_verifier) VALUES ($1,$2,$3,$4,'github','login','http://127.0.0.1/callback',NOW()-INTERVAL '1 minute','SentinelABC-expired-PKCE-verifier')",
        [id.into(), id.as_bytes().repeat(2).into(), id.as_bytes().repeat(2).into(), id.as_bytes().repeat(2).into()]))
        .await.expect("expired PKCE fixture");
    id
}

#[tokio::test]
#[serial]
async fn real_cleanup_failure_is_observable_redacted_and_retries_after_writer_recovers() {
    let (fixture, _, _) = common::setup_test_server().await.expect("Postgres harness");
    fixture_cleanup::run(&fixture, async {
    let db = fixture.db();
    let id = expired_row(db.as_ref()).await;
    let name = format!("test_cleanup_{}", uuid::Uuid::new_v4().simple());
    db.execute_unprepared(&format!("CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF OLD.id='{id}' THEN RAISE EXCEPTION 'SentinelABC-test-storage-error'; END IF; RETURN OLD; END $$"))
        .await.expect("fixture-owned purge error seam");
    if db.execute_unprepared(&format!("CREATE TRIGGER {name} BEFORE DELETE ON oauth_transactions FOR EACH ROW EXECUTE FUNCTION {name}()")).await.is_err() {
        db.execute_unprepared(&format!("DROP FUNCTION {name}()")).await.expect("cleanup unused seam");
        panic!("unable to install fixture error seam");
    }
    let actor = OAuthTransactionCleanup::new(
        Arc::new(SeaOrmOAuthTransactionWriteRepository::new(db.clone())),
        OAuthTransactionCleanupPolicy {
            interval: Duration::from_millis(100),
            batch_size: 1,
        },
    )
    .expect("typed bounded cleanup policy");
    let capture = Capture::default();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_ansi(false)
        .with_writer(capture.clone())
        .finish();
    let (stop, receive) = tokio::sync::watch::channel(false);
    let mut task =
        owned_task::spawn(async move { actor.run(receive).await }.with_subscriber(subscriber));
    let failed = capture.event("oauth_cleanup_failure").await;
    let removed = db
        .execute_unprepared(&format!(
            "DROP TRIGGER {name} ON oauth_transactions; DROP FUNCTION {name}()"
        ))
        .await;
    let recovered = capture.event("oauth_cleanup_success").await;
    let _ = stop.send(true);
    let joined = tokio::time::timeout(Duration::from_secs(5), &mut task).await;
    if joined.is_err() {
        task.abort();
        let _ = task.await;
    }
    assert!(removed.is_ok(), "fixture seam cleanup failed");
    assert!(
        failed,
        "cleanup failure must be observable, not inferred from elapsed sleep"
    );
    assert!(recovered, "cleanup recovery must be observable");
    joined
        .expect("bounded actor join")
        .expect("actor task")
        .expect("actor shutdown");
    assert!(
        capture.redacted(),
        "expired verifier and database error payload must not appear in DEBUG logs"
    );
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT COUNT(*) AS count FROM oauth_transactions WHERE id=$1",
            [id.into()],
        ))
        .await
        .expect("purge recovery")
        .expect("count row");
    assert_eq!(row.try_get::<i64>("", "count").expect("count"), 0);
    }).await;
}

#[tokio::test]
#[serial]
async fn stop_joins_cleanup_while_real_primary_io_is_blocked() {
    let (fixture, _, _) = common::setup_test_server().await.expect("Postgres harness");
    fixture_cleanup::run(&fixture, async {
        let db = fixture.db();
        expired_row(db.as_ref()).await;
        let separate = writer_barrier::independent_writer(db.as_ref()).await;
        let held = separate.begin().await.expect("independent primary lock");
        let pid: i32 = held
            .query_one(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT pg_backend_pid() AS pid".to_owned(),
            ))
            .await
            .expect("PID")
            .expect("PID row")
            .try_get("", "pid")
            .expect("PID");
        held.execute_unprepared("LOCK TABLE oauth_transactions IN ACCESS EXCLUSIVE MODE")
            .await
            .expect("causal I/O block");
        let actor = OAuthTransactionCleanup::new(
            Arc::new(SeaOrmOAuthTransactionWriteRepository::new(db.clone())),
            OAuthTransactionCleanupPolicy {
                interval: Duration::from_secs(60),
                batch_size: 1,
            },
        )
        .expect("cleanup actor");
        let (stop, receive) = tokio::sync::watch::channel(false);
        let mut task = owned_task::spawn(async move { actor.run(receive).await });
        writer_barrier::wait_for_blocked(db.as_ref(), pid, 1).await;
        stop.send(true).expect("stop during actual writer I/O");
        let joined = tokio::time::timeout(Duration::from_secs(5), &mut task).await;
        if joined.is_err() {
            task.abort();
            let _ = task.await;
        }
        held.rollback()
            .await
            .expect("release fixture-owned table lock even on join failure");
        joined
            .expect("cleanup must join before releasing I/O barrier")
            .expect("actor task")
            .expect("actor result");
    })
    .await;
}
