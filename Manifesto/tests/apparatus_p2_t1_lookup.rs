//! Apparatus T1 — lookup SQL de `apparatus_bindings.source` (harness P1).

mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::*;
use fixtures::DbFixtures;
use manifesto_infra::{
    ApparatusBindingSource, ApparatusBindingSourceLookup, SqlApparatusBindingSourceLookup,
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serial_test::serial;
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn t1_sql_lookup_reads_managed_legacy_or_none() {
    let (fixture, _base_url, _client, _openfga, _components) =
        setup_test_server().await.expect("serveur de test");
    let db = fixture.db();

    let owner_managed = Uuid::new_v4();
    let owner_legacy = Uuid::new_v4();
    let (_project, _member, managed_component) =
        DbFixtures::create_project_with_component(&db, owner_managed, "taskboard")
            .await
            .expect("composant managed");
    let (_project, _member, legacy_component) =
        DbFixtures::create_project_with_component(&db, owner_legacy, "wiki")
            .await
            .expect("composant legacy");

    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO apparatus_bindings (component_id, source) VALUES ($1, 'managed')",
        [managed_component.id().into()],
    ))
    .await
    .expect("insert managed binding");
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO apparatus_bindings (component_id, source) VALUES ($1, 'legacy')",
        [legacy_component.id().into()],
    ))
    .await
    .expect("insert legacy binding");

    let lookup = SqlApparatusBindingSourceLookup::new(db.as_ref().clone());

    let managed = lookup
        .source_for_component(managed_component.id())
        .await
        .expect("lookup managed");
    assert_eq!(managed, Some(ApparatusBindingSource::Managed));

    let legacy = lookup
        .source_for_component(legacy_component.id())
        .await
        .expect("lookup legacy");
    assert_eq!(legacy, Some(ApparatusBindingSource::Legacy));

    let missing = lookup
        .source_for_component(Uuid::new_v4())
        .await
        .expect("lookup unknown uuid");
    assert_eq!(missing, None);
}
