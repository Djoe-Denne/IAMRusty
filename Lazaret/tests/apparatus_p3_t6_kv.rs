//! Apparatus P3 — T6 platform KV (Postgres + Redis) and secrets by reference.

mod common;

#[path = "fixtures/mod.rs"]
mod fixtures;

use apparatus_contracts::{
    BindingId, KvStore, MAX_KV_ENTRIES_PER_BINDING, MAX_KV_KEY_LEN, MAX_KV_VALUE_BYTES,
};
use common::setup_test_server;
use fixtures::{TestRedis, VaultFixtures};
use lazaret_domain::{parse_secret_reference, AsyncKvStore, SecretResolver};
use lazaret_infra::{PostgresKvStore, RedisKvStore, VaultHttpSecretResolver};
use serial_test::serial;
use uuid::Uuid;

fn binding(id: Uuid) -> BindingId {
    id.to_string().parse().expect("binding")
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t6_postgres_kv_isolates_bindings_and_purge() {
    let (fixture, _base, _client) = setup_test_server().await.expect("serveur");
    let store = PostgresKvStore::from_arc(&fixture.db());
    let a = binding(Uuid::new_v4());
    let b = binding(Uuid::new_v4());
    let ver = store.put(&a, "k", b"secret-a", None).await.expect("put a");
    assert!(ver >= 1);
    store.put(&b, "k", b"secret-b", None).await.expect("put b");
    assert_eq!(
        store.get(&a, "k").await.expect("get a"),
        Some(b"secret-a".to_vec())
    );
    assert_eq!(
        store.get(&b, "k").await.expect("get b"),
        Some(b"secret-b".to_vec())
    );
    store.purge(&a).await;
    assert_eq!(store.get(&a, "k").await.expect("purged a"), None);
    assert_eq!(
        store.get(&b, "k").await.expect("b intact"),
        Some(b"secret-b".to_vec())
    );
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t6_postgres_cas_and_quota_and_key_bounds() {
    let (fixture, _base, _client) = setup_test_server().await.expect("serveur");
    let store = PostgresKvStore::from_arc(&fixture.db());
    let a = binding(Uuid::new_v4());
    let v1 = store.put(&a, "k", b"one", None).await.expect("put");
    let err = store
        .put(&a, "k", b"two", Some(v1 - 1))
        .await
        .expect_err("cas");
    assert!(err.to_string().contains("cas") || format!("{err:?}").contains("cas"));
    let v2 = store.put(&a, "k", b"two", Some(v1)).await.expect("cas ok");
    assert!(v2 > v1);
    let created = store
        .put(&a, "fresh", b"z", Some(0))
        .await
        .expect("cas 0 creates missing key");
    assert_eq!(created, 1);
    let err0 = store
        .put(&a, "fresh", b"again", Some(0))
        .await
        .expect_err("cas 0 after create");
    assert!(
        err0.to_string().contains("cas") || format!("{err0:?}").contains("cas"),
        "{err0:?}"
    );
    let too_long = "x".repeat(MAX_KV_KEY_LEN + 1);
    assert!(store.get(&a, &too_long).await.is_err());
    let huge = vec![0_u8; MAX_KV_VALUE_BYTES + 1];
    assert!(store.put(&a, "big", &huge, None).await.is_err());
}

#[tokio::test]
#[serial]
async fn t6_redis_kv_isolates_and_implements_port() {
    let redis = TestRedis::new().await.expect("redis");
    redis.flush().expect("flush");
    let store = RedisKvStore::connect(&redis.url()).expect("connect");
    let a = binding(Uuid::new_v4());
    let b = binding(Uuid::new_v4());
    KvStore::kv_put(&store, &a, "k", b"ra", None).expect("put a");
    KvStore::kv_put(&store, &b, "k", b"rb", None).expect("put b");
    assert_eq!(
        KvStore::kv_get(&store, &a, "k").expect("get"),
        Some(b"ra".to_vec())
    );
    assert_eq!(
        KvStore::kv_get(&store, &b, "k").expect("get"),
        Some(b"rb".to_vec())
    );
    KvStore::kv_purge(&store, &a);
    assert_eq!(KvStore::kv_get(&store, &a, "k").expect("purged"), None);
    assert_eq!(
        KvStore::kv_get(&store, &b, "k").expect("b"),
        Some(b"rb".to_vec())
    );
    assert!(KvStore::kv_delete(&store, &b, "k").expect("del"));
}

#[tokio::test]
#[serial]
async fn t6_vault_resolver_returns_plaintext_not_stored_in_kv() {
    let vault = VaultFixtures::service().await;
    vault
        .mock_kv_read("secret", "ops/token", "token", "super-secret")
        .await;
    let resolver =
        VaultHttpSecretResolver::new(vault.base_url(), "test-token", "secret").expect("resolver");
    let reference = "secret:ops/token#token";
    parse_secret_reference(reference).expect("shape");
    let bytes = resolver.resolve(reference).await.expect("resolve");
    assert_eq!(bytes, b"super-secret");

    let (fixture, _base, _client) = setup_test_server().await.expect("serveur");
    let store = PostgresKvStore::from_arc(&fixture.db());
    let a = binding(Uuid::new_v4());
    store
        .put(&a, "ref", reference.as_bytes(), None)
        .await
        .expect("store opaque ref");
    let stored = store.get(&a, "ref").await.expect("get").expect("some");
    assert_eq!(stored, reference.as_bytes());
    assert_ne!(stored, b"super-secret");
}

#[test]
fn t6_secret_reference_rejects_urls() {
    assert!(parse_secret_reference("https://evil").is_err());
    assert!(parse_secret_reference("secret:http://x#y").is_err());
    assert!(parse_secret_reference("secret:../#token").is_err());
    assert!(parse_secret_reference("secret:../x#token").is_err());
    assert!(parse_secret_reference("secret:a//b#f").is_err());
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn t6_postgres_quota_256() {
    let (fixture, _base, _client) = setup_test_server().await.expect("serveur");
    let store = PostgresKvStore::from_arc(&fixture.db());
    let a = binding(Uuid::new_v4());
    for i in 0..MAX_KV_ENTRIES_PER_BINDING {
        store
            .put(&a, &format!("k{i}"), b"v", None)
            .await
            .unwrap_or_else(|_| panic!("put {i}"));
    }
    let err = store
        .put(&a, "overflow", b"v", None)
        .await
        .expect_err("quota");
    assert!(
        err.to_string().contains("quota") || format!("{err:?}").contains("quota"),
        "{err:?}"
    );
    let err_cas0 = store
        .put(&a, "overflow_cas0", b"v", Some(0))
        .await
        .expect_err("quota cas0");
    assert!(
        err_cas0.to_string().contains("quota") || format!("{err_cas0:?}").contains("quota"),
        "{err_cas0:?}"
    );
}

#[tokio::test]
#[serial]
async fn t6_redis_cas() {
    let redis = TestRedis::new().await.expect("redis");
    redis.flush().expect("flush");
    let store = RedisKvStore::connect(&redis.url()).expect("connect");
    let a = binding(Uuid::new_v4());
    let v1 = KvStore::kv_put(&store, &a, "k", b"one", None).expect("put");
    let err = KvStore::kv_put(&store, &a, "k", b"two", Some(v1 - 1)).expect_err("cas");
    assert!(err.to_string().contains("cas") || format!("{err:?}").contains("cas"));
    let v2 = KvStore::kv_put(&store, &a, "k", b"two", Some(v1)).expect("cas ok");
    assert!(v2 > v1);
}
