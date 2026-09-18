//! Apparatus P3 — T12 OpenBao product (compose/testcontainer KV v2).
//!
//! Protocol proof against real OpenBao `-dev`. T6 remains wiremock.

#[path = "fixtures/mod.rs"]
mod fixtures;

use fixtures::TestOpenBao;
use lazaret_domain::{SecretError, SecretResolver};
use lazaret_infra::VaultHttpSecretResolver;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn t12_openbao_kv_v2_resolve_round_trip() {
    let openbao = TestOpenBao::new().await.expect("openbao");
    openbao
        .put_kv_v2("ops/token", "token", "super-secret")
        .await
        .expect("put");
    let resolver = VaultHttpSecretResolver::new(openbao.base_url(), openbao.token(), "secret")
        .expect("resolver");
    let bytes = resolver
        .resolve("secret:ops/token#token")
        .await
        .expect("resolve");
    assert_eq!(bytes, b"super-secret");
}

#[tokio::test]
#[serial]
async fn t12_openbao_empty_token_fail_closes() {
    let openbao = TestOpenBao::new().await.expect("openbao");
    let resolver =
        VaultHttpSecretResolver::new(openbao.base_url(), "", "secret").expect("resolver");
    let err = resolver
        .resolve("secret:ops/token#token")
        .await
        .expect_err("empty token");
    assert_eq!(err, SecretError::ResolveFailed);
}

#[tokio::test]
#[serial]
async fn t12_openbao_missing_path_fail_closes() {
    let openbao = TestOpenBao::new().await.expect("openbao");
    let resolver = VaultHttpSecretResolver::new(openbao.base_url(), openbao.token(), "secret")
        .expect("resolver");
    let err = resolver
        .resolve("secret:ops/missing#token")
        .await
        .expect_err("missing path");
    assert_eq!(err, SecretError::ResolveFailed);
}
