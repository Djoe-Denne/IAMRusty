use super::logical_postgres::{epoch, execute, scalar_bool, wait_bool, LogicalPostgres};
use iam_infra::repository::SeaOrmSessionLivenessChecker;
use iammigration::{Migrator, MigratorTrait as _};
use sea_orm::{ConnectionTrait as _, DatabaseConnection, TransactionTrait as _};
use std::sync::Arc;
use uuid::Uuid;

#[path = "writer_barrier.rs"]
mod writer_barrier;

pub(super) async fn verify_upgrade_and_downgrade(writer: &DatabaseConnection) -> Option<Uuid> {
    let Some(predecessors) = Migrator::migrations()
        .iter()
        .position(|migration| migration.name() == "m20261127_000001_session_liveness_projection")
    else {
        // Preserve the original red path: prove reset before asserting the missing projection.
        Migrator::up(writer, None)
            .await
            .expect("migrate baseline IAM");
        return None;
    };
    Migrator::up(
        writer,
        Some(u32::try_from(predecessors).expect("migration count")),
    )
    .await
    .expect("migrate the real pre-projection schema");

    let zero = Uuid::new_v4();
    let advanced = Uuid::new_v4();
    execute(
        writer,
        &format!(
            "INSERT INTO public.users(id, auth_epoch) VALUES ('{zero}', '0'), ('{advanced}', '42')"
        ),
    )
    .await
    .expect("seed existing accounts before installing the projection");
    Migrator::up(writer, Some(1))
        .await
        .expect("install projection and backfill");
    for (subject, expected) in [(zero, "0"), (advanced, "42")] {
        assert_eq!(
            epoch(writer, subject)
                .await
                .expect("read migration backfill")
                .as_deref(),
            Some(expected)
        );
    }

    let locked_writer = writer_barrier::independent_writer(writer).await;
    let observer = writer_barrier::independent_writer(writer).await;
    let (tx, holder) = writer_barrier::hold(&locked_writer, "users", advanced).await;
    let update = async {
        writer_barrier::wait_for_blocked(&observer, holder, 1).await;
        tx.execute_unprepared(&format!(
            "UPDATE public.users SET auth_epoch = '43' WHERE id = '{advanced}'"
        ))
        .await
        .expect("an in-flight account writer can finish while downgrade waits");
        tx.commit().await.expect("commit authoritative epoch");
    };
    let (down, ()) = tokio::join!(Migrator::down(writer, Some(1)), update);
    down.expect("downgrade completes after the writer without a lock upgrade deadlock");
    assert!(scalar_bool(
        writer,
        &format!(
            "SELECT to_regclass('public.iam_session_liveness_epochs') IS NULL \
             AND to_regclass('public.iam_session_liveness_checkpoints') IS NULL \
             AND EXISTS (SELECT 1 FROM public.users WHERE id = '{advanced}' AND auth_epoch = '43') AS ok"
        )
    )
    .await
    .expect("check downgrade removed only derived data"));
    Migrator::up(writer, Some(1))
        .await
        .expect("rebuild the projection from retained epochs");
    verify_checkpoint_retention(writer).await;
    Some(advanced)
}

async fn verify_checkpoint_retention(writer: &DatabaseConnection) {
    execute(
        writer,
        "INSERT INTO public.iam_session_liveness_checkpoints(challenge)
         SELECT gen_random_uuid() FROM generate_series(1, 129)",
    )
    .await
    .expect("seed pre-existing markers without an ordering queue");
    Migrator::up(writer, None)
        .await
        .expect("add bounded checkpoint retention");
    assert!(scalar_bool(
        writer,
        "SELECT count(*) = 128 AS ok FROM public.iam_session_liveness_checkpoints"
    )
    .await
    .expect("enroll and trim markers that preceded the migration"));
    Migrator::down(writer, Some(1))
        .await
        .expect("remove only the retention queue");
    Migrator::up(writer, None)
        .await
        .expect("re-enroll retained markers");

    execute(writer, "CREATE ROLE iam_fixture_checkpoint_writer NOLOGIN")
        .await
        .expect("create restricted IAM writer role in the owned cluster");
    let grants = include_str!("../../../../ops/deploy/iam-session-replica/iam-writer.sql")
        .replace(":\"iam_writer\"", "\"iam_fixture_checkpoint_writer\"");
    writer
        .execute_unprepared(&grants)
        .await
        .expect("apply runtime grants");
    let options = writer
        .get_postgres_connection_pool()
        .connect_options()
        .as_ref()
        .clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query("SET ROLE iam_fixture_checkpoint_writer")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with(options)
        .await
        .unwrap_or_else(|_| panic!("restricted writer connection failed (details redacted)"));
    let restricted = Arc::new(sea_orm::SqlxPostgresConnector::from_sqlx_postgres_pool(
        pool,
    ));
    let checkpoint_writer = SeaOrmSessionLivenessChecker::new(restricted, "fixture".into());
    let challenge = Uuid::new_v4();
    assert!(checkpoint_writer
        .write_checkpoint(challenge)
        .await
        .expect("restricted writer can commit"));
    assert!(scalar_bool(
        writer,
        &format!(
            "SELECT count(*) = 128 AND bool_or(challenge = '{challenge}') AS ok
             FROM public.iam_session_liveness_checkpoints"
        ),
    )
    .await
    .expect("old marker retires after a restricted writer adds a new one"));
}

pub(super) async fn verify_admission_grants(pg: &LogicalPostgres, advanced: Uuid) {
    let writer = pg.writer.as_ref().expect("initialized writer");
    let subscriber = pg.subscriber.as_ref().expect("initialized subscriber");
    assert!(scalar_bool(
        writer,
        "SELECT NOT has_table_privilege('iam_session_replication', 'public.users', 'SELECT') \
         AND NOT has_table_privilege('iam_session_admission', 'public.users', 'SELECT') \
         AND (SELECT count(*) = 2 FROM pg_publication_tables \
              WHERE pubname = 'iam_projection_publication' AND schemaname = 'public' \
              AND tablename IN ('iam_session_liveness_epochs', 'iam_session_liveness_checkpoints')) \
         AND (SELECT count(*) = 2 FROM pg_publication_tables \
              WHERE pubname = 'iam_projection_publication') AS ok",
    )
    .await
    .expect("verify source grants and exact publication"));
    let tx = subscriber.begin().await.expect("isolate admission role");
    tx.execute_unprepared("SET LOCAL ROLE iam_session_admission")
        .await
        .expect("switch to restricted subscriber role");
    tx.execute_unprepared(&format!(
        "SELECT auth_epoch FROM public.iam_session_liveness_epochs WHERE subject = '{advanced}'; \
         SELECT challenge FROM public.iam_session_liveness_checkpoints; \
         SELECT configured_generation, generation FROM public.iam_session_liveness_ready"
    ))
    .await
    .expect("admission role reads exactly the required relations");
    for relation in [
        "iam_session_liveness_epochs",
        "iam_session_liveness_checkpoints",
    ] {
        let update = tx.begin().await.expect("isolate expected denial");
        let error = update
            .execute_unprepared(&format!("DELETE FROM public.{relation} WHERE false"))
            .await
            .expect_err("subscriber admission role cannot write projection state");
        assert!(error.to_string().contains("permission denied for table"));
        update.rollback().await.expect("clear expected denial");
    }
    tx.rollback().await.expect("restore subscriber admin role");
}

pub(super) async fn verify_deleted_account(pg: &LogicalPostgres, subject: Uuid) {
    let writer = pg.writer.as_ref().expect("initialized writer");
    let subscriber = pg.subscriber.as_ref().expect("initialized subscriber");
    execute(
        writer,
        &format!("DELETE FROM public.users WHERE id = '{subject}'"),
    )
    .await
    .expect("delete migrated fixture account from authority");
    assert_eq!(
        epoch(writer, subject)
            .await
            .expect("read source after deletion"),
        None
    );
    let marker = pg
        .publish_checkpoint()
        .await
        .expect("commit a post-deletion checkpoint");
    assert!(
        wait_bool(
            subscriber,
            &format!(
                "SELECT EXISTS (SELECT 1 FROM public.iam_session_liveness_checkpoints \
             WHERE challenge = '{marker}') AND NOT EXISTS \
             (SELECT 1 FROM public.iam_session_liveness_epochs WHERE subject = '{subject}') AS ok"
            ),
        )
        .await
        .expect("observe post-deletion checkpoint with absent account"),
        "a deleted account must not appear as epoch zero on the subscriber"
    );
}
