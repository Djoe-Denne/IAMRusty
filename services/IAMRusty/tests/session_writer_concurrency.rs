//! S8: actual PostgreSQL locks/rollback, not a mock SQL call sequence.
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
use iam_domain::{entity::token::RefreshToken, port::repository::AuthenticationSessionWriter};
use iam_infra::repository::authentication_session::SeaOrmAuthenticationSessionWriter;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
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

async fn count(db: &sea_orm::DatabaseConnection, user: Uuid) -> i64 {
    db.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT COUNT(*) AS count FROM refresh_tokens WHERE user_id = $1 AND is_valid",
        [user.into()],
    ))
    .await
    .expect("session read")
    .expect("count row")
    .try_get("", "count")
    .expect("count")
}

#[tokio::test]
#[serial]
async fn concurrent_resets_and_old_credential_issue_have_one_reset_and_no_surviving_old_session() {
    let (fixture, _, _) = common::setup_test_server().await.expect("Postgres harness");
    fixture_cleanup::run(&fixture, async {
    let db = fixture.db();
    let user = DbFixtures::user()
        .arthur()
        .password_hash("pre-reset-credential-hash")
        .commit(db.clone())
        .await
        .expect("user");
    let old = user.model().password_hash.clone();
    assert!(old.is_some(), "credential proof fixture required");
    let writer = Arc::new(SeaOrmAuthenticationSessionWriter::new(db.clone()));
    writer
        .issue(refresh(user.id()), old.clone())
        .await
        .expect("old session through real writer");
    DbFixtures::password_reset_token()
        .user_id(user.id())
        .raw_token("fixed-old-reset-token")
        .expires_at(chrono::Utc::now() + chrono::Duration::hours(1))
        .commit(db.clone())
        .await
        .expect("reset token fixture");
    let separate_pool = writer_barrier::independent_writer(db.as_ref()).await;
    let (held, holder) = writer_barrier::hold(&separate_pool, "users", user.id()).await;
    let barrier = Arc::new(tokio::sync::Barrier::new(3));
    let mut resets = Vec::new();
    for replacement in ["reset-winner-one", "reset-winner-two"] {
        let writer = writer.clone();
        let barrier = barrier.clone();
        let old = old.clone();
        let id = user.id();
        resets.push(owned_task::spawn(async move {
            barrier.wait().await;
            (
                replacement,
                writer
                    .reset_password(id, old, replacement.into(), None)
                    .await,
            )
        }));
    }
    let issuer = {
        let writer = writer.clone();
        let barrier = barrier.clone();
        let old = old.clone();
        let id = user.id();
        owned_task::spawn(async move {
            barrier.wait().await;
            writer.issue(refresh(id), old).await
        })
    };
    writer_barrier::wait_for_blocked(db.as_ref(), holder, 3).await;
    held.commit()
        .await
        .expect("release lock before reset/issue linearization");
    let one = resets.remove(0).await.expect("first reset task");
    let two = resets.remove(0).await.expect("second reset task");
    let _old_issue_result = issuer.await.expect("old issuance task");
    // Either issuance linearizes before reset and is purged, or its stale proof
    // loses after reset. No interleaving may leave a stale refresh alive.
    assert_eq!(usize::from(one.1.is_ok()) + usize::from(two.1.is_ok()), 1);
    let loser = if one.1.is_ok() { &two.1 } else { &one.1 };
    assert!(matches!(
        loser,
        Err(iam_domain::error::DomainError::InvalidToken)
    ));
    assert_eq!(count(db.as_ref(), user.id()).await, 0);
    let winner = if one.1.is_ok() { one.0 } else { two.0 };
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT password_hash, (SELECT COUNT(*) FROM password_reset_tokens WHERE user_id = $1) AS resets FROM users WHERE id = $1", [user.id().into()]))
        .await.expect("committed reset state").expect("user row");
    assert!(
        row.try_get::<String>("", "password_hash")
            .expect("password hash")
            == winner,
        "committed password must be exactly the reset winner"
    );
    assert_eq!(
        row.try_get::<i64>("", "resets").expect("reset token count"),
        0
    );
    assert!(
        writer.issue(refresh(user.id()), old).await.is_err(),
        "stale verified password cannot issue after reset"
    );
    writer
        .issue(refresh(user.id()), Some(winner.into()))
        .await
        .expect("genuine post-reset issuance");
    assert_eq!(count(db.as_ref(), user.id()).await, 1);
    }).await;
}

#[tokio::test]
#[serial]
async fn reset_purge_failure_rolls_back_password_and_all_existing_sessions() {
    let (fixture, _, _) = common::setup_test_server().await.expect("Postgres harness");
    fixture_cleanup::run(&fixture, async {
    let db = fixture.db();
    let user = DbFixtures::user()
        .arthur()
        .password_hash("pre-reset-credential-hash")
        .commit(db.clone())
        .await
        .expect("user");
    let old = user.model().password_hash.clone();
    let writer = SeaOrmAuthenticationSessionWriter::new(db.clone());
    writer
        .issue(refresh(user.id()), old.clone())
        .await
        .expect("first session");
    writer
        .issue(refresh(user.id()), old.clone())
        .await
        .expect("second session");
    let name = format!("test_purge_{}", Uuid::new_v4().simple());
    db.execute(Statement::from_string(DatabaseBackend::Postgres, format!(
        "CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF OLD.user_id = '{}' THEN RAISE EXCEPTION 'test session purge failure'; END IF; RETURN OLD; END $$", user.id())))
        .await.expect("fixture-owned SQL failure seam");
    let trigger = db.execute(Statement::from_string(DatabaseBackend::Postgres, format!(
        "CREATE TRIGGER {name} BEFORE DELETE ON refresh_tokens FOR EACH ROW EXECUTE FUNCTION {name}()"))).await;
    if trigger.is_err() {
        db.execute(Statement::from_string(
            DatabaseBackend::Postgres,
            format!("DROP FUNCTION {name}()"),
        ))
        .await
        .expect("cleanup unused seam");
        panic!("unable to install fixture-owned purge failure seam");
    }
    let reset = writer
        .reset_password(user.id(), old.clone(), "must-not-persist".into(), None)
        .await;
    // Remove test-owned objects before assertions, including the unexpected-success case.
    db.execute(Statement::from_string(
        DatabaseBackend::Postgres,
        format!("DROP TRIGGER {name} ON refresh_tokens"),
    ))
    .await
    .expect("cleanup trigger");
    db.execute(Statement::from_string(
        DatabaseBackend::Postgres,
        format!("DROP FUNCTION {name}()"),
    ))
    .await
    .expect("cleanup function");
    assert!(reset.is_err());
    assert_eq!(count(db.as_ref(), user.id()).await, 2);
    // Successful issue with old proof confirms the credential update also rolled back.
    writer
        .issue(refresh(user.id()), old)
        .await
        .expect("old credential remains current after rollback");
    assert_eq!(count(db.as_ref(), user.id()).await, 3);
    }).await;
}
