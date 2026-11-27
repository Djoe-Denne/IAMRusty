//! Green qualification of transactional/ordered replication only, not admission freshness.

use super::logical_postgres::{epoch, execute, scalar_bool, LogicalPostgres};
use crate::fixtures::DbFixtures;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

pub(super) async fn rollback_pause_resume(pg: &LogicalPostgres, base: &str, subject: Uuid) {
    let writer = pg.writer.as_ref().expect("initialized writer");
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("bounded qualification client");
    // A real current session guarantees the purge seam has a row to reject.
    assert_eq!(
        client
            .post(format!("{base}/api/auth/login"))
            .json(&json!({
                "email": "logical-recovery@example.com",
                "password": "NewLogical2!"
            }))
            .send()
            .await
            .expect("create current session before rollback")
            .status(),
        StatusCode::OK
    );
    let proof = DbFixtures::password_reset_token()
        .valid(subject)
        .commit(writer.clone())
        .await
        .expect("seed proof retained across failed recovery");
    purge_failure_after_update(writer, &client, base, proof.token(), subject).await;
    assert!(
        scalar_bool(
            writer,
            &format!(
                "SELECT EXISTS (SELECT 1 FROM public.users \
                 WHERE id = '{subject}' AND auth_epoch = '1') AS ok"
            ),
        )
        .await
        .expect("read account after rollback"),
        "failed recovery must roll back the already-updated authoritative epoch"
    );
    assert_eq!(
        epoch(writer, subject)
            .await
            .expect("read source after rollback")
            .as_deref(),
        Some("1"),
        "the production source projection must roll back with the account"
    );
    let after_rollback = pg
        .publish_checkpoint()
        .await
        .expect("commit rollback ordering control");
    assert!(
        pg.wait_checkpoint_epoch(after_rollback, subject, 1)
            .await
            .expect("read ordered rollback snapshot"),
        "a later checkpoint must apply without the rolled-back epoch reaching the subscriber"
    );
    paused_retry(pg, &client, base, proof.token(), subject).await;
}

async fn paused_retry(
    pg: &LogicalPostgres,
    client: &Client,
    base: &str,
    token: &str,
    subject: Uuid,
) {
    let writer = pg.writer.as_ref().expect("initialized writer");
    pg.pause_projection()
        .await
        .expect("disable and join projection apply workers");
    let confirmation = client
        .post(format!("{base}/api/auth/password/reset-confirm"))
        .json(&json!({"token": token, "new_password": "ResumedLogical4!"}))
        .send()
        .await
        .expect("retry the same proof while apply is paused");
    assert_eq!(confirmation.status(), StatusCode::OK);
    let body: Value = confirmation
        .json()
        .await
        .expect("read resumed confirmation");
    assert_eq!(
        body.as_object().expect("confirmation object").len(),
        1,
        "the failed transaction must not consume the recovery proof or issue a session"
    );
    assert!(
        body["message"].as_str() == Some("Password has been successfully reset"),
        "the successful retry must retain the existing recovery message"
    );
    assert!(
        scalar_bool(
            writer,
            &format!(
                "SELECT EXISTS (SELECT 1 FROM public.users \
             WHERE id = '{subject}' AND auth_epoch = '2') AS ok"
            )
        )
        .await
        .expect("read committed second reset"),
        "the successful retry must commit epoch 2 on the IAM writer"
    );
    assert_eq!(
        epoch(writer, subject)
            .await
            .expect("read paused source")
            .as_deref(),
        Some("2"),
        "the committed source must advance while its subscriber is paused"
    );
    let after_reset = pg
        .publish_checkpoint()
        .await
        .expect("commit post-reset ordering control");
    assert!(
        pg.checkpoint_snapshot(after_reset, subject, 1, false)
            .await
            .expect("query reachable paused subscriber"),
        "a reachable paused subscriber must retain epoch 1 and lack the later checkpoint"
    );
    pg.resume_projection()
        .await
        .expect("resume owned projection subscription");
    assert!(
        pg.wait_checkpoint_epoch(after_reset, subject, 2)
            .await
            .expect("observe resumed ordinary apply"),
        "the post-reset checkpoint and epoch 2 must arrive in the same subscriber snapshot"
    );
    assert_eq!(
        epoch(pg.subscriber.as_ref().expect("subscriber"), subject)
            .await
            .expect("read resumed subscriber")
            .as_deref(),
        Some("2"),
        "resumed replication must deliver the actual committed reset"
    );
}

async fn purge_failure_after_update(
    writer: &sea_orm::DatabaseConnection,
    client: &Client,
    base: &str,
    token: &str,
    subject: Uuid,
) {
    // Reuse session_writer_concurrency's per-fixture BEFORE DELETE failure seam.
    // A sequence is deliberately nontransactional: it proves the authoritative
    // epoch had already advanced before the failure, even after rollback.
    let name = format!("test_replica_purge_{}", Uuid::new_v4().simple());
    execute(writer, &format!("CREATE SEQUENCE public.{name}_seen"))
        .await
        .expect("create rollback progress witness");
    execute(
        writer,
        &format!(
            "CREATE FUNCTION public.{name}() RETURNS trigger LANGUAGE plpgsql AS $$ \
         BEGIN IF OLD.user_id = '{subject}' THEN \
         IF (SELECT auth_epoch FROM public.users WHERE id = OLD.user_id) = '2' THEN \
         PERFORM nextval('public.{name}_seen'); END IF; \
         RAISE EXCEPTION 'fixture-owned session purge failure'; \
         END IF; RETURN OLD; END $$"
        ),
    )
    .await
    .expect("create owned purge failure function");
    execute(
        writer,
        &format!(
            "CREATE TRIGGER {name} BEFORE DELETE ON public.refresh_tokens \
         FOR EACH ROW EXECUTE FUNCTION public.{name}()"
        ),
    )
    .await
    .expect("install owned purge failure trigger");
    // Keep the result until cleanup; even an unexpected success or HTTP error
    // must not leave the deliberate failure trigger installed for the retry.
    let response = client
        .post(format!("{base}/api/auth/password/reset-confirm"))
        .json(&json!({"token": token, "new_password": "RejectedLogical3!"}))
        .send()
        .await;
    execute(
        writer,
        &format!("DROP TRIGGER {name} ON public.refresh_tokens"),
    )
    .await
    .expect("remove owned purge failure trigger");
    execute(writer, &format!("DROP FUNCTION public.{name}()"))
        .await
        .expect("remove owned purge failure function");
    let reached = scalar_bool(
        writer,
        &format!("SELECT is_called AS ok FROM public.{name}_seen"),
    )
    .await;
    execute(writer, &format!("DROP SEQUENCE public.{name}_seen"))
        .await
        .expect("remove rollback progress witness");
    assert!(
        reached.expect("read nontransactional progress witness"),
        "purge failure must be reached after the IAM account epoch was updated to 2"
    );
    assert!(
        !response
            .expect("receive deliberately failed recovery")
            .status()
            .is_success(),
        "the fixture-owned purge failure must reject recovery"
    );
}
