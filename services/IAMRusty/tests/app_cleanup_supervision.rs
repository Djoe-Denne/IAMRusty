//! Integration6: real root must expose and stop both owned background handles.
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

use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait};
use serial_test::serial;
use std::time::Duration;

#[tokio::test]
#[serial]
async fn actual_app_root_signals_and_joins_outbox_and_cleanup_during_primary_io() {
    let (fixture, _, _) = common::setup_test_server()
        .await
        .expect("owned real-core fixture");
    fixture_cleanup::run(&fixture, async {
        let db = fixture.db();
        let mut security = iam_configuration::security::SecurityConfig::default();
        security.mode = iam_configuration::security::SecurityMode::IsolatedTest;
        let app = common::build_test_iam_app(&fixture, security)
            .await
            .expect("actual root with default cleanup policy");
        let separate = writer_barrier::independent_writer(db.as_ref()).await;
        let held = separate.begin().await.expect("primary I/O barrier");
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
            .expect("block actual cleanup SQL");
        let mut handles: Vec<_> = app
            .start_background_tasks()
            .into_iter()
            .map(owned_task::OwnedTask::new)
            .collect();
        let has_both = handles.len() == 2;
        writer_barrier::wait_for_blocked(db.as_ref(), pid, 1).await;
        let signaled =
            tokio::time::timeout(Duration::from_secs(5), app.stop_background_tasks()).await;
        let mut joined = true;
        for mut handle in handles.drain(..) {
            match tokio::time::timeout(Duration::from_secs(5), &mut handle).await {
                Ok(Ok(Ok(()))) => {}
                _ => {
                    joined = false;
                    handle.abort();
                    let _ = handle.await;
                }
            }
        }
        held.rollback()
            .await
            .expect("release fixture I/O barrier before assertions");
        assert!(has_both, "root must retain BOTH outbox and cleanup handles");
        assert!(
            signaled.is_ok(),
            "root cooperative stop must not block indefinitely"
        );
        assert!(
        joined,
        "both owned handles must terminate while cleanup I/O is blocked; never detached/pop-only"
    );
    })
    .await;
}
