//! Apparatus P3 — T8 KV purge on Manifesto `component_removed`.

mod common;

#[path = "fixtures/mod.rs"]
mod fixtures;

use std::sync::Arc;

use apparatus_contracts::BindingId;
use chrono::Utc;
use common::setup_test_server;
use fixtures::TestRedis;
use lazaret_domain::AsyncKvStore;
use lazaret_infra::{KvPurgeEventHandler, PostgresKvStore, RedisKvStore};
use manifesto_events::{ComponentAddedEvent, ComponentRemovedEvent, ManifestoDomainEvent};
use rustycog::events::{DomainEvent, EventHandler};
use serial_test::serial;
use uuid::Uuid;

fn binding(id: Uuid) -> BindingId {
    id.to_string().parse().expect("binding")
}

fn removed(component_id: Uuid) -> Box<dyn DomainEvent> {
    ManifestoDomainEvent::ComponentRemoved(ComponentRemovedEvent::new(
        Uuid::new_v4(),
        component_id,
        "taskboard".to_owned(),
        Uuid::new_v4(),
        Utc::now(),
    ))
    .into()
}

fn added(component_id: Uuid) -> Box<dyn DomainEvent> {
    ManifestoDomainEvent::ComponentAdded(ComponentAddedEvent::new(
        Uuid::new_v4(),
        component_id,
        "taskboard".to_owned(),
        Uuid::new_v4(),
        Utc::now(),
    ))
    .into()
}

async fn assert_purge_isolates_neighbor(store: Arc<dyn AsyncKvStore>) {
    let handler = KvPurgeEventHandler::new(store.clone());
    let a_id = Uuid::new_v4();
    let b_id = Uuid::new_v4();
    let a = binding(a_id);
    let b = binding(b_id);

    store.put(&a, "k", b"secret-a", None).await.expect("put a");
    store.put(&b, "k", b"secret-b", None).await.expect("put b");

    assert!(
        !handler.supports_event_type("component_added"),
        "handler must ignore component_added at the contract"
    );
    handler
        .handle_event(added(a_id))
        .await
        .expect("added is a no-op");
    assert_eq!(
        store.get(&a, "k").await.expect("a after added"),
        Some(b"secret-a".to_vec())
    );
    assert_eq!(
        store.get(&b, "k").await.expect("b after added"),
        Some(b"secret-b".to_vec())
    );

    assert!(handler.supports_event_type("component_removed"));
    assert!(handler.supports_event_type("ComponentRemoved"));
    handler.handle_event(removed(a_id)).await.expect("purge a");
    assert_eq!(store.get(&a, "k").await.expect("purged a"), None);
    assert_eq!(
        store.get(&b, "k").await.expect("b intact"),
        Some(b"secret-b".to_vec())
    );

    handler
        .handle_event(removed(a_id))
        .await
        .expect("second purge is a no-op");
    assert_eq!(store.get(&a, "k").await.expect("a still gone"), None);
    assert_eq!(
        store.get(&b, "k").await.expect("b still intact"),
        Some(b"secret-b".to_vec())
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t8_postgres_purges_binding_on_component_removed() {
    let (fixture, _base, _client) = setup_test_server().await.expect("serveur");
    let store: Arc<dyn AsyncKvStore> = Arc::new(PostgresKvStore::from_arc(&fixture.db()));
    assert_purge_isolates_neighbor(store).await;
}

#[tokio::test]
#[serial]
async fn t8_redis_purges_binding_on_component_removed() {
    let redis = TestRedis::new().await.expect("redis");
    redis.flush().expect("flush");
    let store: Arc<dyn AsyncKvStore> =
        Arc::new(RedisKvStore::connect(&redis.url()).expect("connect"));
    assert_purge_isolates_neighbor(store).await;
}
