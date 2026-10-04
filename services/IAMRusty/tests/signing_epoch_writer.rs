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

#[tokio::test]
#[serial]
async fn emission_fence_reads_fresh_committed_primary_state_not_the_initial_active_object() {
    let key = registry::registry_key(SigningKeyStatus::Active, None);
    let (fixture, _, _) = common::setup_test_server_with_signing_keys(&[key.clone()])
        .await
        .expect("writer registry fixture");
    fixture_cleanup::run(&fixture, async {
    let db = fixture.db();
    let primary = SeaOrmSigningKeyRegistry::new(db.clone());
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
    separate
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "DELETE FROM signing_keys WHERE id=$1",
            [initial.id.into()],
        ))
        .await
        .expect("remove primary epoch");
    assert!(!primary
        .confirm_active_for_emission(&initial)
        .await
        .expect("absent row fence"));
    }).await;
}
