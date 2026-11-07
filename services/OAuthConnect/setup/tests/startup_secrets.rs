//! B-1/B-2/B-3: real setup resolution and SDK KV-v2, no composition/vendor/OIDC runtime.

#[path = "support/b_gzip.rs"]
mod b_gzip;
#[path = "support/b_provider.rs"]
mod b_provider;
#[path = "support/b_vault.rs"]
mod b_vault;

use b_gzip::GZIP_KV;
use b_provider::provider;
use b_vault::{padded_kv, Vault, CLIENT_PATH, HMAC_PATH, LOOKUP, TOKEN};
use oauth_connect_configuration::{
    AppConfig, OidcProfile, OpenBaoAuthConfig, ProviderConfig, ProviderProfile, SecretReference,
    SecretsMode,
};
use oauth_connect_setup::{resolve_secrets, ResolvedProviderSecrets};
use rustycog::config::secrets::{OpenBaoKv2Resolver, SecretResolver};
use rustycog::core::error::DomainError;
use serde_json::json;
use std::{
    collections::BTreeMap,
    error::Error,
    io::Write,
    sync::{Arc, Mutex},
    time::Duration,
};
use tracing::instrument::WithSubscriber;
use wiremock::ResponseTemplate;

const CLIENT: &str = "vault-client-sentinel";
const HMAC: &str = "vault-HMAC-sentinel-at-least-sixteen";

fn config(providers: Vec<ProviderConfig>) -> AppConfig {
    AppConfig {
        providers,
        ..AppConfig::default()
    }
}

fn vault_provider(id: &str, namespace: &str) -> ProviderConfig {
    let mut value = provider(id);
    value.secrets_mode = SecretsMode::Openbao;
    value.client_secret = None;
    value.hmac_secret = None;
    value.client_secret_ref = Some(
        SecretReference::new(
            "secret".into(),
            format!("{namespace}/oauth"),
            "client_secret".into(),
        )
        .expect("real client reference"),
    );
    value.hmac_secret_ref = Some(
        SecretReference::new(
            "secret".into(),
            format!("{namespace}/iam"),
            "hmac_secret".into(),
        )
        .expect("real HMAC reference"),
    );
    value
}

fn vault_config(vault: &Vault) -> AppConfig {
    let mut value = config(vec![vault_provider("forge", "github-connect")]);
    value.openbao = Some(vault.config());
    value
}

fn oidc(mut value: ProviderConfig) -> ProviderConfig {
    value.scopes = vec!["openid".into(), "profile".into(), "email".into()];
    value.profile = ProviderProfile::Oidc(OidcProfile {
        issuer: "https://identity.example.invalid/tenant".into(),
        allowed_endpoint_origins: vec!["https://identity.example.invalid".into()],
    });
    value
}

fn sanitized(text: &str) {
    for marker in ["sentinel", "github-connect/oauth", "github-connect/iam"] {
        assert!(
            !text.to_ascii_lowercase().contains(marker),
            "B-3: diagnostic exposed credential/reference material"
        );
    }
}

fn failure<T>(result: Result<T, DomainError>) -> DomainError {
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("B: invalid acquisition returned a usable result"),
    };
    sanitized(&format!("{error:?} {error}"));
    let mut source = error.source();
    for _ in 0..8 {
        let Some(cause) = source else { return error };
        sanitized(&format!("{cause:?} {cause}"));
        source = cause.source();
    }
    assert!(source.is_none(), "B-3: unbounded diagnostic source chain");
    error
}

async fn resolved(value: &AppConfig) -> Vec<ResolvedProviderSecrets> {
    match resolve_secrets(value).await {
        Ok(bundles) => bundles,
        Err(error) => {
            sanitized(&format!("{error:?} {error}"));
            panic!("B-1: valid declarations must resolve as a complete batch");
        }
    }
}

fn exact(bundles: Vec<ResolvedProviderSecrets>, expected: &[(&str, &str, &str)]) {
    sanitized(&format!("{bundles:?}"));
    let mut actual = BTreeMap::new();
    for bundle in bundles {
        let previous = actual.insert(
            bundle.id,
            (
                bundle.client_secret.into_inner(),
                bundle.hmac_secret.into_inner(),
            ),
        );
        assert!(previous.is_none(), "B-1: duplicate resolved id");
    }
    assert_eq!(
        actual.len(),
        expected.len(),
        "B-1: incomplete or extra bundle"
    );
    for (id, client, hmac) in expected {
        let pair = actual
            .get(*id)
            .expect("B-1: expected id missing from resolved batch");
        assert!(
            pair.0.as_bytes() == client.as_bytes(),
            "B-1: wrong client bytes for instance"
        );
        assert!(
            pair.1.as_bytes() == hmac.as_bytes(),
            "B-1: wrong canonical HMAC bytes for instance"
        );
    }
}

async fn nominal_vault(vault: &Vault) {
    vault.lookup(600, &["connector-read"]).await;
    vault.secret(CLIENT_PATH, "client_secret", CLIENT).await;
    vault.secret(HMAC_PATH, "hmac_secret", HMAC).await;
}

#[tokio::test]
async fn b_fixture_sdk_kvv2_bootstrap_and_fields_preflight() {
    let vault = Vault::start().await;
    nominal_vault(&vault).await;
    let value = vault_config(&vault);
    // Establish fixture/API correctness independently of the service's T0 refusal.
    let result = OpenBaoKv2Resolver::connect(value.openbao.as_ref().unwrap()).await;
    let resolver = match result {
        Ok(resolver) => resolver,
        Err(error) => {
            sanitized(&format!("{error:?} {error}"));
            panic!("SDK fixture preflight failed: not a service RED");
        }
    };
    for (reference, expected) in [
        (
            value.providers[0].client_secret_ref.as_ref().unwrap(),
            CLIENT,
        ),
        (value.providers[0].hmac_secret_ref.as_ref().unwrap(), HMAC),
    ] {
        let secret = match resolver.resolve(reference).await {
            Ok(secret) => secret,
            Err(error) => {
                sanitized(&format!("{error:?} {error}"));
                panic!("SDK KV fixture preflight failed: not a service RED");
            }
        };
        sanitized(&format!("{secret:?}"));
        assert!(
            secret.into_inner().as_bytes() == expected.as_bytes(),
            "SDK fixture must return the exact configured field bytes"
        );
    }
    assert_eq!(vault.count(LOOKUP).await, 1);
    assert_eq!(vault.count(CLIENT_PATH).await, 1);
    assert_eq!(vault.count(HMAC_PATH).await, 1);
}

#[tokio::test]
async fn b1_direct_oauth_and_oidc_keep_instance_bytes_without_contacting_configured_vault() {
    let vault = Vault::start().await;
    let mut first = provider("forge");
    first.client_secret = Some("  direct-client-one-sentinel \t".into());
    first.hmac_secret = Some("  direct-HMAC-one-sentinel\r\n".into());
    let mut second = oidc(provider("identity"));
    second.client_secret = Some("direct-client-two-sentinel".into());
    second.hmac_secret = Some("direct-HMAC-two-sentinel".into());
    let mut value = config(vec![first, second]);
    value.openbao = Some(vault.config());
    exact(
        resolved(&value).await,
        &[
            (
                "forge",
                "  direct-client-one-sentinel \t",
                "direct-HMAC-one-sentinel",
            ),
            (
                "identity",
                "direct-client-two-sentinel",
                "direct-HMAC-two-sentinel",
            ),
        ],
    );
    assert_eq!(
        vault.total().await,
        0,
        "B-1: direct mode contacted the vault"
    );
}

#[tokio::test]
async fn b1_direct_hmac_trim_keeps_interior_unicode_and_client_secret_exact() {
    for (raw, canonical) in [
        (" \t0123456789abcdef\r\n", "0123456789abcdef"),
        ("\u{2003}éééééééé\u{00a0}", "éééééééé"),
        ("  AbCD efGH\tijKL mnOP  ", "AbCD efGH\tijKL mnOP"),
    ] {
        let mut p = provider("forge");
        p.client_secret = Some(" \tclient-exact-sentinel \r\n".into());
        p.hmac_secret = Some(raw.into());
        let value = config(vec![p]);
        exact(
            resolved(&value).await,
            &[("forge", " \tclient-exact-sentinel \r\n", canonical)],
        );
        assert!(
            value.providers[0].hmac_secret.as_deref() == Some(raw),
            "B-1: resolver rewrote config"
        );
    }
}

#[tokio::test]
async fn b2_direct_rejects_blank_short_after_trim_and_raw_oversize_before_trim() {
    for raw in [
        " ".to_owned(),
        " 0123456789abcde ".to_owned(),
        "\u{2003}ééééééé\u{2003}".to_owned(),
        format!("{}0123456789abcdef", " ".repeat(16_369)),
    ] {
        let mut p = provider("forge");
        p.hmac_secret = Some(raw);
        failure(resolve_secrets(&config(vec![p])).await);
    }
    for raw in ["".to_owned(), "\t\n".to_owned(), "x".repeat(16_385)] {
        let mut p = provider("forge");
        p.client_secret = Some(raw);
        failure(resolve_secrets(&config(vec![p])).await);
    }
}

#[tokio::test]
async fn b1_direct_accepts_inclusive_raw_sixteen_kib_before_hmac_trim() {
    let client = "c".repeat(16_384);
    let mut p = provider("forge");
    p.client_secret = Some(client.clone());
    p.hmac_secret = Some(format!("{}0123456789abcdef", " ".repeat(16_368)));
    exact(
        resolved(&config(vec![p])).await,
        &[("forge", &client, "0123456789abcdef")],
    );
}

#[tokio::test]
async fn b1_vault_two_instances_resolve_exact_kvv2_routes_fields_and_ids() {
    let vault = Vault::start().await;
    nominal_vault(&vault).await;
    vault
        .secret(
            "/v1/secret/data/enterprise/oauth",
            "client_secret",
            "enterprise-client-sentinel",
        )
        .await;
    vault
        .secret(
            "/v1/secret/data/enterprise/iam",
            "hmac_secret",
            "enterprise-HMAC-sentinel",
        )
        .await;
    let mut value = vault_config(&vault);
    value
        .providers
        .push(oidc(vault_provider("identity", "enterprise")));
    exact(
        resolved(&value).await,
        &[
            ("forge", CLIENT, HMAC),
            (
                "identity",
                "enterprise-client-sentinel",
                "enterprise-HMAC-sentinel",
            ),
        ],
    );
    for route in [
        CLIENT_PATH,
        HMAC_PATH,
        "/v1/secret/data/enterprise/oauth",
        "/v1/secret/data/enterprise/iam",
    ] {
        assert_eq!(
            vault.count(route).await,
            1,
            "B-1: each distinct reference must be acquired once at boot"
        );
    }
    // Resolver/session sharing is not contractual (batch, instance or reference).
    assert!(
        vault.count(LOOKUP).await > 0,
        "B-1: successful bootstrap phase missing"
    );
}

#[tokio::test]
async fn b1_mixed_direct_and_vault_allow_explicitly_equal_secrets_per_instance() {
    let vault = Vault::start().await;
    nominal_vault(&vault).await;
    let mut direct = provider("other");
    direct.client_secret = Some(CLIENT.into());
    direct.hmac_secret = Some(HMAC.into());
    let mut value = vault_config(&vault);
    value.providers.push(direct);
    exact(
        resolved(&value).await,
        &[("forge", CLIENT, HMAC), ("other", CLIENT, HMAC)],
    );
    assert_eq!(vault.count(CLIENT_PATH).await, 1);
    assert_eq!(vault.count(HMAC_PATH).await, 1);
}

#[tokio::test]
async fn b1_vault_hmac_obeys_the_same_trim_and_byte_rules_as_direct() {
    for (raw, canonical) in [
        (" \t0123456789abcdef\r\n", "0123456789abcdef"),
        ("\u{2003}éééééééé\u{00a0}", "éééééééé"),
        ("  AbCD efGH\tijKL mnOP  ", "AbCD efGH\tijKL mnOP"),
    ] {
        let vault = Vault::start().await;
        vault.lookup(600, &["connector-read"]).await;
        vault
            .secret(
                CLIENT_PATH,
                "client_secret",
                " \tclient-exact-sentinel \r\n",
            )
            .await;
        vault.secret(HMAC_PATH, "hmac_secret", raw).await;
        exact(
            resolved(&vault_config(&vault)).await,
            &[("forge", " \tclient-exact-sentinel \r\n", canonical)],
        );
    }
}

#[tokio::test]
async fn b2_vault_short_trimmed_or_raw_oversize_hmac_never_returns_a_batch() {
    for raw in [
        "  ".to_owned(),
        " 0123456789abcde ".to_owned(),
        "\u{2003}ééééééé\u{2003}".to_owned(),
        format!("{}0123456789abcdef", " ".repeat(16_369)),
    ] {
        let vault = Vault::start().await;
        vault.lookup(600, &["connector-read"]).await;
        vault.secret(CLIENT_PATH, "client_secret", CLIENT).await;
        vault.secret(HMAC_PATH, "hmac_secret", &raw).await;
        failure(resolve_secrets(&vault_config(&vault)).await);
        assert_eq!(
            vault.count(HMAC_PATH).await,
            1,
            "B-2: reach the actual SDK acquisition phase"
        );
    }
}

#[tokio::test]
async fn b2_invalid_later_declarations_are_rejected_before_any_vault_io() {
    for case in [
        "missing-direct",
        "mixed-direct",
        "missing-reference",
        "mixed-vault",
    ] {
        let vault = Vault::start().await;
        nominal_vault(&vault).await;
        let mut bad = provider("other");
        match case {
            "missing-direct" => bad.hmac_secret = None,
            "mixed-direct" => {
                bad.client_secret_ref = Some(
                    SecretReference::new(
                        "secret".into(),
                        "reference-sentinel/path".into(),
                        "field_sentinel".into(),
                    )
                    .unwrap(),
                )
            }
            "missing-reference" => {
                bad = vault_provider("other", "other");
                bad.client_secret_ref = None;
            }
            _ => {
                bad = vault_provider("other", "other");
                bad.client_secret = Some("fallback-direct-sentinel".into());
            }
        }
        let mut value = vault_config(&vault);
        value.providers.push(bad);
        failure(resolve_secrets(&value).await);
        assert_eq!(
            vault.total().await,
            0,
            "B-2: prevalidate the complete declaration before acquisition"
        );
    }
}

#[tokio::test]
async fn b2_failure_of_second_remote_instance_exposes_no_partial_result() {
    let vault = Vault::start().await;
    nominal_vault(&vault).await;
    let failed = "/v1/secret/data/second-sentinel/oauth";
    vault
        .response(
            failed,
            ResponseTemplate::new(403).set_body_string("remote-body-sentinel"),
        )
        .await;
    vault
        .secret("/v1/secret/data/second-sentinel/iam", "hmac_secret", HMAC)
        .await;
    let mut value = vault_config(&vault);
    value
        .providers
        .push(vault_provider("other", "second-sentinel"));
    failure(resolve_secrets(&value).await);
    assert_eq!(
        vault.count(failed).await,
        1,
        "B-2: second instance must actually fail remotely"
    );
    // No rollback or ordering of earlier GETs is required; Err exposes no vector.
}

#[tokio::test]
async fn b1_token_file_lf_crlf_and_lookup_ttl_endpoints_are_accepted() {
    for (ending, ttl) in [("\n", 1), ("\r\n", 3_600)] {
        let vault = Vault::start().await;
        std::fs::write(vault.token.path(), format!("{TOKEN}{ending}")).unwrap();
        vault.lookup(ttl, &["connector-read"]).await;
        vault.secret(CLIENT_PATH, "client_secret", CLIENT).await;
        vault.secret(HMAC_PATH, "hmac_secret", HMAC).await;
        exact(
            resolved(&vault_config(&vault)).await,
            &[("forge", CLIENT, HMAC)],
        );
    }
}

#[tokio::test]
async fn b2_token_file_missing_nonregular_blank_oversize_or_invalid_bytes_refuse_before_io() {
    for case in [
        "missing",
        "directory",
        "blank",
        "oversize",
        "space",
        "control",
        "utf8",
    ] {
        let vault = Vault::start().await;
        let directory = tempfile::tempdir().unwrap();
        let mut value = vault_config(&vault);
        let path = match case {
            "missing" => directory.path().join("missing-path-sentinel"),
            "directory" => directory.path().to_owned(),
            _ => {
                let bytes = match case {
                    "blank" => b"\n".to_vec(),
                    "oversize" => vec![b'x'; 16_385],
                    "space" => b"token with-space-sentinel".to_vec(),
                    "control" => b"token\0control-sentinel".to_vec(),
                    _ => vec![0xff],
                };
                std::fs::write(vault.token.path(), bytes).unwrap();
                vault.token.path().to_owned()
            }
        };
        value.openbao.as_mut().unwrap().auth = OpenBaoAuthConfig::TokenFile { path };
        failure(resolve_secrets(&value).await);
        assert_eq!(
            vault.total().await,
            0,
            "B-2: invalid file must never send a bootstrap token"
        );
    }
}

#[tokio::test]
async fn b2_unauthorized_bootstrap_credential_never_reaches_kvv2() {
    let vault = Vault::start().await;
    vault
        .response(
            LOOKUP,
            ResponseTemplate::new(403).set_body_string("credential-error-sentinel"),
        )
        .await;
    failure(resolve_secrets(&vault_config(&vault)).await);
    assert_eq!(vault.count(LOOKUP).await, 1);
    assert_eq!(
        vault.count(CLIENT_PATH).await + vault.count(HMAC_PATH).await,
        0
    );
}

#[tokio::test]
async fn b2_root_unbounded_or_missing_bootstrap_identity_never_reaches_kvv2() {
    for data in [
        json!({"ttl": 0, "policies": ["read"]}),
        json!({"ttl": 3601, "policies": ["read"]}),
        json!({"ttl": 600, "policies": ["root"]}),
        json!({"ttl": 600}),
        json!({"policies": ["read"]}),
        json!({"ttl": 600, "policies": ["read"], "identity_policies": ["root"]}),
    ] {
        let vault = Vault::start().await;
        vault
            .response(
                LOOKUP,
                ResponseTemplate::new(200).set_body_json(json!({"data": data})),
            )
            .await;
        failure(resolve_secrets(&vault_config(&vault)).await);
        assert_eq!(vault.count(LOOKUP).await, 1);
        assert_eq!(
            vault.count(CLIENT_PATH).await + vault.count(HMAC_PATH).await,
            0
        );
    }
}

#[tokio::test]
async fn b2_kvv2_403_404_and_503_are_closed_and_sanitized_without_retry() {
    for status in [403, 404, 503] {
        let vault = Vault::start().await;
        vault.lookup(600, &["connector-read"]).await;
        vault
            .response(
                CLIENT_PATH,
                ResponseTemplate::new(status)
                    .set_body_string("client-token-nonce-verifier-body-sentinel"),
            )
            .await;
        vault.secret(HMAC_PATH, "hmac_secret", HMAC).await;
        failure(resolve_secrets(&vault_config(&vault)).await);
        assert_eq!(
            vault.count(CLIENT_PATH).await,
            1,
            "B-2: no retry or fallback credential"
        );
    }
}

#[tokio::test]
async fn b2_kvv2_missing_null_wrong_type_blank_or_oversize_value_is_not_a_default() {
    for body in [
        json!({}),
        json!({"data": null}),
        json!({"data": {"data": null}}),
        json!({"data": {"data": {}}}),
        json!({"data": {"data": {"client_secret": null}}}),
        json!({"data": {"data": {"client_secret": 42}}}),
        json!({"data": {"data": {"client_secret": "  \t"}}}),
        json!({"data": {"data": {"client_secret": "x".repeat(16_385)}}}),
    ] {
        let vault = Vault::start().await;
        vault.lookup(600, &["connector-read"]).await;
        vault
            .response(CLIENT_PATH, ResponseTemplate::new(200).set_body_json(body))
            .await;
        vault.secret(HMAC_PATH, "hmac_secret", HMAC).await;
        failure(resolve_secrets(&vault_config(&vault)).await);
        assert_eq!(vault.count(CLIENT_PATH).await, 1);
    }
}

#[tokio::test]
async fn b1_vault_accepts_sixteen_kib_raw_values_before_normalization() {
    let vault = Vault::start().await;
    vault.lookup(600, &["connector-read"]).await;
    let client = "c".repeat(16_384);
    vault.secret(CLIENT_PATH, "client_secret", &client).await;
    vault
        .secret(
            HMAC_PATH,
            "hmac_secret",
            &format!("{}0123456789abcdef", " ".repeat(16_368)),
        )
        .await;
    exact(
        resolved(&vault_config(&vault)).await,
        &[("forge", &client, "0123456789abcdef")],
    );
}

#[tokio::test]
async fn b1_sdk_response_limit_is_inclusive_at_sixty_four_kib_not_vendor_one_mib() {
    let vault = Vault::start().await;
    vault.lookup(600, &["connector-read"]).await;
    vault
        .response(
            CLIENT_PATH,
            ResponseTemplate::new(200).set_body_bytes(padded_kv(65_536)),
        )
        .await;
    vault.secret(HMAC_PATH, "hmac_secret", HMAC).await;
    exact(
        resolved(&vault_config(&vault)).await,
        &[("forge", CLIENT, HMAC)],
    );
}

#[tokio::test]
async fn b2_sdk_refuses_oversize_json_errors_and_nonidentity_encoding_with_gzip_feature_active() {
    for response in [
        ResponseTemplate::new(200).set_body_bytes(padded_kv(65_537)),
        ResponseTemplate::new(200).set_body_string("{invalid-json-sentinel"),
        ResponseTemplate::new(200)
            .insert_header("Content-Encoding", "br")
            .set_body_bytes(padded_kv(80)),
        ResponseTemplate::new(200)
            .insert_header("Content-Encoding", "gzip")
            .set_body_bytes(GZIP_KV),
    ] {
        let vault = Vault::start().await;
        vault.lookup(600, &["connector-read"]).await;
        vault.response(CLIENT_PATH, response).await;
        vault.secret(HMAC_PATH, "hmac_secret", HMAC).await;
        failure(resolve_secrets(&vault_config(&vault)).await);
        assert_eq!(vault.count(CLIENT_PATH).await, 1);
    }
}

#[tokio::test]
async fn b2_vault_redirect_never_forwards_bootstrap_token_to_destination() {
    let destination = wiremock::MockServer::start().await;
    let vault = Vault::start().await;
    vault.lookup(600, &["connector-read"]).await;
    vault
        .response(
            CLIENT_PATH,
            ResponseTemplate::new(307)
                .insert_header("Location", format!("{}/destination", destination.uri())),
        )
        .await;
    vault.secret(HMAC_PATH, "hmac_secret", HMAC).await;
    failure(resolve_secrets(&vault_config(&vault)).await);
    assert_eq!(vault.count(CLIENT_PATH).await, 1);
    assert!(
        destination.received_requests().await.unwrap().is_empty(),
        "B-2: vault redirect leaked token"
    );
}

#[tokio::test]
async fn b2_vault_http_without_explicit_loopback_exception_refuses_before_io() {
    let vault = Vault::start().await;
    let mut value = vault_config(&vault);
    value.openbao.as_mut().unwrap().allow_loopback_http = false;
    failure(resolve_secrets(&value).await);
    assert_eq!(vault.total().await, 0);
}

#[tokio::test]
async fn b2_sdk_five_second_timeout_has_nominal_control_reached_phase_and_watchdog() {
    let control = Vault::start().await;
    nominal_vault(&control).await;
    exact(
        resolved(&vault_config(&control)).await,
        &[("forge", CLIENT, HMAC)],
    );
    let vault = Vault::start().await;
    vault.lookup(600, &["connector-read"]).await;
    vault
        .response(
            CLIENT_PATH,
            ResponseTemplate::new(200)
                .set_body_json(json!({"data": {"data": {"client_secret": CLIENT}}}))
                .set_delay(Duration::from_secs(8)),
        )
        .await;
    vault.secret(HMAC_PATH, "hmac_secret", HMAC).await;
    let value = vault_config(&vault);
    let start = tokio::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(9), resolve_secrets(&value)).await;
    let result = result.expect("B-2: watchdog fired, internal SDK timeout is NOT proven");
    failure(result);
    let elapsed = start.elapsed();
    assert!(
        elapsed >= Duration::from_secs(4) && elapsed < Duration::from_secs(8),
        "B-2: failure must be the SDK deadline, not immediate refusal or delayed valid response"
    );
    assert_eq!(
        vault.count(CLIENT_PATH).await,
        1,
        "B-2: delayed GET must be reached once without retry"
    );
}

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);
impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn b3_real_acquisition_logs_debug_and_error_sources_hide_values_and_reference_fields() {
    let vault = Vault::start().await;
    vault.lookup(600, &["connector-read"]).await;
    vault.response(CLIENT_PATH, ResponseTemplate::new(403).set_body_string(
        "body-sentinel bootstrap-token-sentinel code-sentinel verifier-sentinel nonce-sentinel")).await;
    let mut value = vault_config(&vault);
    value.providers[0].hmac_secret_ref = Some(
        SecretReference::new(
            "secret".into(),
            "reference-sentinel/path".into(),
            "field_sentinel".into(),
        )
        .unwrap(),
    );
    vault
        .secret(
            "/v1/secret/data/reference-sentinel/path",
            "field_sentinel",
            HMAC,
        )
        .await;
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .with_writer(move || writer.clone())
        .finish();
    async {
        tracing::debug!("B3_CAPTURE_WITNESS");
        failure(resolve_secrets(&value).await);
    }
    .with_subscriber(subscriber)
    .await;
    let bytes = capture
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let logs = match String::from_utf8(bytes) {
        Ok(logs) => logs,
        Err(_) => panic!("B-3: tracing fixture produced invalid UTF-8"),
    };
    assert!(
        logs.contains("B3_CAPTURE_WITNESS"),
        "B-3: empty capture cannot pass"
    );
    sanitized(&logs);
    sanitized(&format!("{value:?}"));
    sanitized(&serde_json::to_string(&value).unwrap());
    assert_eq!(
        vault.count(CLIENT_PATH).await,
        1,
        "B-3: exercise real outbound failure"
    );
}
