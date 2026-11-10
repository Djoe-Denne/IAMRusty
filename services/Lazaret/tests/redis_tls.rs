//! Native Redis TLS transport and in-place plaintext-to-TLS data compatibility.

#[path = "fixtures/redis.rs"]
mod redis_fixture;

use apparatus_contracts::{BindingId, KvStore};
use lazaret_domain::AsyncKvStore;
use lazaret_infra::RedisKvStore;
use redis_fixture::TestRedisTls;
use serial_test::serial;

fn binding() -> BindingId {
    uuid::Uuid::new_v4().to_string().parse().expect("binding")
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn native_tls_checks_ca_and_hostname_without_fallback_and_preserves_kv() {
    let redis = TestRedisTls::new().await.expect("owned Redis TLS fixture");
    let a = binding();
    let neighbor = binding();
    let plaintext = RedisKvStore::connect(&redis.plaintext_url()).expect("legacy plaintext");
    let version = KvStore::kv_put(&plaintext, &a, "sentinel", b"before-tls", None)
        .expect("seed through plaintext");
    KvStore::kv_put(&plaintext, &neighbor, "sentinel", b"neighbor", None).expect("seed neighbor");

    let tls = RedisKvStore::connect_tls(&redis.url(), &redis.ca_path()).expect("TLS parameters");
    assert_eq!(
        tls.get(&a, "sentinel")
            .await
            .expect("verified TLS handshake"),
        Some(b"before-tls".to_vec())
    );

    let wrong_ca = tempfile::NamedTempFile::new().expect("wrong CA file");
    let other_ca = rcgen::generate_simple_self_signed(vec!["localhost".into()]).expect("other CA");
    std::fs::write(wrong_ca.path(), other_ca.cert.pem()).expect("wrong CA PEM");
    let untrusted =
        RedisKvStore::connect_tls(&redis.url(), wrong_ca.path()).expect("typed wrong CA");
    assert!(
        untrusted.get(&a, "sentinel").await.is_err(),
        "wrong CA must fail at handshake"
    );

    let wrong_name = format!("rediss://127.0.0.1:{}", redis.tls_port);
    let mismatch =
        RedisKvStore::connect_tls(&wrong_name, &redis.ca_path()).expect("TLS parameters");
    assert!(
        mismatch.get(&a, "sentinel").await.is_err(),
        "wrong SAN must fail at handshake"
    );

    let plain_endpoint = format!("rediss://127.0.0.1:{}", redis.plaintext_port);
    let no_fallback =
        RedisKvStore::connect_tls(&plain_endpoint, &redis.ca_path()).expect("TLS parameters");
    assert!(
        no_fallback.get(&a, "sentinel").await.is_err(),
        "TLS must not retry in plaintext"
    );
    // This exact plaintext endpoint does have the key: a downgrade would pass.
    assert_eq!(
        KvStore::kv_get(&plaintext, &a, "sentinel").expect("plaintext control"),
        Some(b"before-tls".to_vec())
    );

    redis
        .disable_plaintext()
        .expect("transition this fixture to TLS-only without flush or restart");
    assert_eq!(
        tls.get(&a, "sentinel")
            .await
            .expect("same data after transport switch"),
        Some(b"before-tls".to_vec())
    );
    assert!(tls
        .put(&a, "sentinel", b"bad-cas", Some(version + 1))
        .await
        .is_err());
    let next = tls
        .put(&a, "sentinel", b"after-tls", Some(version))
        .await
        .expect("CAS via TLS");
    assert!(next > version);
    tls.purge(&a).await.expect("purge via TLS");
    assert_eq!(tls.get(&a, "sentinel").await.expect("purged binding"), None);
    assert_eq!(
        tls.get(&neighbor, "sentinel")
            .await
            .expect("neighbor preserved"),
        Some(b"neighbor".to_vec())
    );
    assert!(tls
        .delete(&neighbor, "sentinel")
        .await
        .expect("delete via TLS"));
    redis
        .cleanup()
        .await
        .expect("stop/remove exact owned fixture");
}
