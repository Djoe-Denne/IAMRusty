use crate::{common, fixtures::DbFixtures};
use futures::FutureExt as _;
use reqwest::StatusCode;
use sea_orm::{ConnectionTrait as _, DatabaseBackend, Statement};
use serde_json::{json, Value};
use serial_test::serial;
use std::{panic::AssertUnwindSafe, time::Duration};

#[path = "../fixtures/logical_postgres.rs"]
mod logical_postgres;
use logical_postgres::{epoch, scalar_bool, wait_epoch, LogicalPostgres};
#[path = "password_recovery_replica_qualification.rs"]
mod qualification;
#[path = "password_recovery_projection_upgrade.rs"]
mod upgrade;

#[tokio::test]
#[serial]
async fn recovery_commit_reaches_real_pg15_subscriber() {
    let mut pg = LogicalPostgres::start()
        .await
        .expect("start owned logical PG15");
    let outcome = AssertUnwindSafe(tokio::time::timeout(
        Duration::from_secs(180),
        exercise_contract(&mut pg),
    ))
    .catch_unwind()
    .await;
    let cleanup = pg.cleanup().await;
    if cleanup.is_err() {
        eprintln!("owned logical PG15 cleanup failed; primary must reconcile tracked IDs");
    }
    match outcome {
        Err(panic) => std::panic::resume_unwind(panic),
        Ok(result) => {
            result.expect("logical-replication scenario exceeded its 180-second deadline");
        }
    }
    cleanup.expect("join owned IAM listeners and stop/remove tracked PG15");
}

async fn exercise_contract(pg: &mut LogicalPostgres) {
    const CHECKPOINT_TOKEN_ENV: &str = "IAM_LOGICAL_REPLICA_FIXTURE_CHECKPOINT_TOKEN";
    pg.initialize()
        .await
        .expect("initialize isolated PG15 databases");
    pg.prove_transport()
        .await
        .expect("prove control replication before product assertions");
    let writer = pg.writer.as_ref().expect("initialized writer");
    let advanced = upgrade::verify_upgrade_and_downgrade(writer).await;
    let checkpoint_token = uuid::Uuid::new_v4().simple().to_string();
    std::env::set_var(CHECKPOINT_TOKEN_ENV, &checkpoint_token);
    let result = Box::pin(common::setup_test_server_on_writer(
        writer.clone(),
        &pg.writer_url(),
        CHECKPOINT_TOKEN_ENV,
    ))
    .await;
    std::env::remove_var(CHECKPOINT_TOKEN_ENV);
    let (base, _) = result.expect("start real IAM on owned logical writer");
    let user = DbFixtures::create_user_with_email_password(
        writer,
        "logical-recovery@example.com",
        "OldLogical1!",
        Some("logical_recovery"),
    )
    .await
    .expect("seed account through existing fixture");
    assert_eq!(user.model().auth_epoch, "0");
    let proof = DbFixtures::password_reset_token()
        .valid(user.id())
        .commit(writer.clone())
        .await
        .expect("seed recovery proof");
    let source_exists = scalar_bool(
        writer,
        "SELECT to_regclass('public.iam_session_liveness_epochs') IS NOT NULL AS ok",
    )
    .await
    .expect("inspect production source without undefined-table failure");
    if source_exists {
        let advanced = advanced.expect("registered upgrade seeded an advanced account");
        assert!(
            scalar_bool(
                writer,
                "SELECT to_regclass('public.iam_session_liveness_checkpoints') IS NOT NULL AS ok"
            )
            .await
            .expect("inspect checkpoint source"),
            "ADR 0317 requires the checkpoint relation in the same publication"
        );
        pg.subscribe_projection()
            .await
            .expect("initialize trusted production subscription");
        let subscriber = pg.subscriber.as_ref().expect("subscriber");
        assert_eq!(
            epoch(subscriber, advanced)
                .await
                .expect("read migrated subscriber account")
                .as_deref(),
            Some("43")
        );
        upgrade::verify_admission_grants(pg, advanced).await;
        assert_eq!(
            epoch(pg.subscriber.as_ref().expect("subscriber"), user.id())
                .await
                .expect("read bootstrap epoch")
                .as_deref(),
            Some("0"),
            "ordinary initial synchronization must include the existing epoch-zero account"
        );
    }

    confirm_and_check_writer(writer, &base, proof.token(), user.id()).await;
    // Do not return/skip when the source is absent: reach this exact
    // behavioral assertion AFTER the independent transport and HTTP controls.
    let projected = if source_exists {
        epoch(writer, user.id())
            .await
            .expect("read production source epoch")
    } else {
        None
    };
    assert_eq!(
        projected.as_deref(),
        Some("1"),
        "ADR 0317: transport is proven and users.auth_epoch=1, but the production source \
             projection is absent or stale; subscriber delivery cannot be established"
    );
    let subscriber = pg.subscriber.as_ref().expect("subscriber");
    assert!(
        wait_epoch(subscriber, user.id())
            .await
            .expect("observe ordinary apply"),
        "committed reset did not advance the real subscriber to epoch 1 within 20 seconds"
    );
    assert_eq!(
        epoch(subscriber, user.id())
            .await
            .expect("read applied epoch")
            .as_deref(),
        Some("1"),
        "the production epoch must reach the real PG15 subscriber"
    );
    qualification::rollback_pause_resume(pg, &base, user.id(), &checkpoint_token).await;
    upgrade::verify_deleted_account(pg, advanced.expect("backfilled advanced account")).await;
}

async fn confirm_and_check_writer(
    writer: &sea_orm::DatabaseConnection,
    base: &str,
    token: &str,
    subject: uuid::Uuid,
) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("build bounded HTTP client");
    let confirmation = client
        .post(format!("{base}/api/auth/password/reset-confirm"))
        .json(&json!({"token": token, "new_password": "NewLogical2!"}))
        .send()
        .await
        .expect("confirm recovery through real IAM HTTP");
    assert_eq!(confirmation.status(), StatusCode::OK);
    assert_eq!(
        confirmation
            .json::<Value>()
            .await
            .expect("read confirmation"),
        json!({"message": "Password has been successfully reset"}),
        "recovery must succeed without issuing a session"
    );
    let account = writer
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT auth_epoch FROM public.users WHERE id=$1",
            [subject.into()],
        ))
        .await
        .expect("read committed account")
        .expect("account exists");
    assert_eq!(
        account
            .try_get::<String>("", "auth_epoch")
            .expect("read account epoch"),
        "1",
        "successful HTTP recovery must commit the writer epoch"
    );
}
