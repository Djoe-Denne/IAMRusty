//! C/G-7/G-8, independent contract oracles, written against integrated A+B.
//! Obligation -> risk -> scenario -> oracle:
//! - Atomic boot -> omitted/partial OIDC -> mixed direct/vault and late failures ->
//!   complete profiles/keys, real SDK receipts, no returned catalogue/application.
//! - True composition -> fixture-only transport proof -> gzip/TLS OIDC requests ->
//!   signed identity and exact phase receipts using `build_clients`' returned port.
//! - Stateless service -> local nonce correlation -> rebuild then complete without
//!   authorize -> identical identity with explicit request context, no hidden session.
//! - Owned admission -> early listen/false readiness -> Application port0, bind/TLS
//!   failure -> full-path HMAC HTTP, real ready, shutdown/join before panic propagation.
//!
//! KEEP tests below; negatives passing T0 alone are characterizations, not proof.

#[path = "support/d_owned.rs"]
mod d_owned;
#[path = "support/d_tls.rs"]
mod d_tls;
#[path = "support/d_vault.rs"]
mod d_vault;
#[path = "support/d_vendor.rs"]
mod d_vendor;

use d_vault::Vault;
use d_vendor::{
    body, clean, complete, metadata, mount, must, signature, source, token, Vendors, CHALLENGE,
    DISCOVERY, KEY_OAUTH, KEY_OIDC, NONCE, RAW_KEY, SECRET_OIDC,
};
use flate2::{write::GzEncoder, Compression};
use idp_connect_contract::{
    hmac::{verify, SIGNATURE_HEADER, TIMESTAMP_HEADER},
    FederatedOAuthClient, FederatedOAuthError,
};
use oauth_connect_configuration::{
    load_config, DomainError, OidcProfile, ProviderProfile, VendorTransportSecurity,
};
use oauth_connect_infra::OidcClient;
use oauth_connect_setup::{build_clients, AppBuilder, Application, ConfiguredClient};
use rustycog::config::secrets::SecretValue;
use serde_json::Value;
use std::{
    error::Error,
    io::Write,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
use wiremock::ResponseTemplate;

fn rejected<T>(result: &Result<T, DomainError>) {
    let Err(error) = result else {
        panic!("boot returned a usable partial catalogue/application");
    };
    clean(&format!("{error:?} {error}"));
    let mut source = error.source();
    for _ in 0..8 {
        let Some(cause) = source else {
            return;
        };
        clean(&format!("{cause:?} {cause}"));
        source = cause.source();
    }
    assert!(source.is_none(), "unexpected unbounded error chain");
}
fn selected<'a>(clients: &'a [ConfiguredClient], id: &str) -> &'a ConfiguredClient {
    assert_eq!(clients.iter().filter(|c| c.id == id).count(), 1);
    clients
        .iter()
        .find(|c| c.id == id)
        .expect("complete mixed catalogue id")
}
fn key_proof(client: &ConfiguredClient, expected: &str, other: &str) {
    clean(&format!("{client:?}"));
    let path = format!("/oauth-connect/{}/v2/complete", client.id);
    let bytes = body(true);
    let (at, mac) = signature(&path, &bytes, expected);
    must(
        verify(
            client.hmac_key.as_bytes(),
            "POST",
            &path,
            at,
            &bytes,
            &mac,
            at,
        ),
        "composed canonical HMAC verification",
    );
    for wrong in [other, RAW_KEY] {
        assert!(
            verify(wrong.as_bytes(), "POST", &path, at, &bytes, &mac, at).is_err(),
            "raw/other key accepted"
        );
    }
}
fn incoming_client() -> reqwest::Client {
    must(
        reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(4))
            .build(),
        "incoming service test client",
    )
}
async fn response_bytes(mut response: reqwest::Response) -> (reqwest::StatusCode, Vec<u8>) {
    let status = response.status();
    let mut bytes = Vec::new();
    while let Some(chunk) = must(response.chunk().await, "bounded incoming response") {
        assert!(chunk.len() <= 8192 - bytes.len(), "oversized test response");
        bytes.extend_from_slice(&chunk);
    }
    (status, bytes)
}
async fn signed_call(
    http: &reqwest::Client,
    origin: &str,
    id: &str,
    complete_flow: bool,
    key: &str,
) -> (reqwest::StatusCode, Vec<u8>) {
    let operation = if complete_flow {
        "complete"
    } else {
        "authorize"
    };
    let path = format!("/oauth-connect/{id}/v2/{operation}");
    let bytes = body(complete_flow);
    let (at, mac) = signature(&path, &bytes, key);
    response_bytes(must(
        http.post(format!("{origin}{path}"))
            .header("content-type", "application/json")
            .header(TIMESTAMP_HEADER, at.to_string())
            .header(SIGNATURE_HEADER, mac)
            .body(bytes)
            .send()
            .await,
        "Application real signed v2 request",
    ))
    .await
}

#[tokio::test]
async fn d_fixture_signed_oidc_preflight_uses_integrated_a_without_composition() {
    let vendors = Vendors::new().await;
    let config = vendors.config();
    let http = must(
        reqwest::Client::builder().no_proxy().build(),
        "trusted isolated OIDC preflight client",
    );
    let secret = must(
        SecretValue::new(SECRET_OIDC.into()),
        "preflight secret wrapper",
    );
    let client = must(
        OidcClient::new(&config.providers[1], secret, http).await,
        "integrated A fixture preflight",
    );
    let result = must(
        client.complete_authorization(&complete()).await,
        "preflight signed OIDC identity",
    );
    vendors.assert_profile(&must(serde_json::to_value(result), "preflight DTO"), true);
    assert_eq!(vendors.count(true, DISCOVERY).await, 1);
    assert_eq!(vendors.count(true, "/keys").await, 1);
    vendors.credentials(true).await;
}

async fn mixed_completions(vendors: &Vendors, clients: &[ConfiguredClient]) {
    for _ in 0..2 {
        for (id, oidc) in [("forge", false), ("authority", true)] {
            let authenticated = must(
                selected(clients, id)
                    .client
                    .complete_authorization(&complete())
                    .await,
                "mixed complete",
            );
            vendors.assert_profile(
                &must(serde_json::to_value(authenticated), "mixed DTO"),
                oidc,
            );
            vendors.credentials(oidc).await;
        }
    }
    assert_eq!(
        vendors.count(true, "/keys").await,
        1,
        "fresh per-instance JWKS cache"
    );
}
async fn renewed_identity(vendors: &Vendors, clients: &[ConfiguredClient]) {
    // Same access token, newly signed identity: no access-token-to-profile cache is admissible.
    vendors.oidc.reset().await;
    let mut renewed = d_vendor::claims(&vendors.oidc.uri());
    renewed["sub"] = Value::String("000043".into());
    mount(
        &vendors.oidc,
        "POST",
        "/token",
        ResponseTemplate::new(200).set_body_json(d_vendor::token_with_claims(&renewed)),
    )
    .await;
    let renewed = must(
        selected(clients, "authority")
            .client
            .complete_authorization(&complete())
            .await,
        "same-access-token renewed signed identity",
    );
    assert!(
        renewed.profile.id == "000043",
        "identity must be recomputed from each verified token"
    );
    assert!(renewed.tokens.access_token == "oidc-access-sentinel");
    assert_eq!(vendors.count(true, "/token").await, 1);
    assert_eq!(vendors.count(true, "/keys").await, 0);
}
async fn failed_identity(vendors: &Vendors, clients: &[ConfiguredClient]) {
    vendors.oidc.reset().await;
    mount(
        &vendors.oidc,
        "POST",
        "/token",
        ResponseTemplate::new(403).set_body_string("failure-sentinel"),
    )
    .await;
    let result = selected(clients, "authority")
        .client
        .complete_authorization(&complete())
        .await;
    assert!(
        matches!(result, Err(FederatedOAuthError::ExchangeCode)),
        "token failure must not become partial authentication"
    );
    assert_eq!(
        vendors.count(true, "/token").await,
        1,
        "no retry of authorization code"
    );
}

#[tokio::test]
async fn d7_mixed_direct_vault_catalogue_is_complete_snapshot_and_has_no_partial_business_success()
{
    let vendors = Vendors::new().await;
    let vault = Vault::new().await;
    let mut config = vendors.config();
    Vault::provider(&mut config.providers[1]);
    config.openbao = Some(vault.config());
    let clients = must(
        build_clients(&config).await,
        "mixed direct/vault composition",
    );
    assert_eq!(clients.len(), 2);
    assert!(
        vault.reads().await >= 2,
        "SDK acquisition must actually occur"
    );
    vault.initial_reads().await;
    assert_eq!(vendors.count(true, DISCOVERY).await, 1);
    assert_eq!(vendors.count(true, "/keys").await, 1);
    key_proof(selected(&clients, "forge"), KEY_OAUTH, KEY_OIDC);
    key_proof(selected(&clients, "authority"), KEY_OIDC, KEY_OAUTH);
    vault.server.reset().await;
    vault
        .values("replaced-client-sentinel", "replaced-key-sentinel")
        .await;
    mixed_completions(&vendors, &clients).await;
    assert_eq!(
        vault.reads().await,
        0,
        "no business-time SDK secret refresh"
    );
    renewed_identity(&vendors, &clients).await;
    assert_eq!(vault.reads().await, 0);
    failed_identity(&vendors, &clients).await;
    assert_eq!(vault.reads().await, 0);
    key_proof(selected(&clients, "authority"), KEY_OIDC, KEY_OAUTH);
}

#[tokio::test]
async fn d7_late_issuer_jwks_and_secret_failures_refuse_whole_application() {
    for failure in ["issuer", "jwks", "secret"] {
        let vendors = Vendors::new().await;
        let mut config = vendors.config();
        if failure == "secret" {
            config.providers[1].hmac_secret = Some("too-short".into());
        } else {
            vendors.oidc.reset().await;
            let mut discovery = metadata(&vendors.oidc.uri());
            if failure == "issuer" {
                discovery["issuer"] = Value::String("http://127.0.0.1:1/wrong".into());
            }
            mount(
                &vendors.oidc,
                "GET",
                DISCOVERY,
                ResponseTemplate::new(200).set_body_json(discovery),
            )
            .await;
            mount(
                &vendors.oidc,
                "GET",
                "/keys",
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"keys": []})),
            )
            .await;
        }
        rejected(&build_clients(&config).await);
        rejected(&Application::new(config).await);
        if failure != "secret" {
            assert!(
                vendors.count(true, DISCOVERY).await >= 1,
                "T0 refusal alone is not a late-failure proof"
            );
        }
        // Both constructions are intentional; do not assume one JWKS fetch.
        if failure == "issuer" {
            assert_eq!(
                vendors.count(true, "/keys").await,
                0,
                "issuer mismatch must stop before JWKS acquisition"
            );
        } else if failure == "jwks" {
            assert!(
                vendors.count(true, "/keys").await > 0,
                "invalid JWKS refusal must actually reach the keys endpoint"
            );
        }
        assert_eq!(vendors.count(false, "/token").await, 0);
        assert_eq!(vendors.count(true, "/token").await, 0);
    }
}

#[tokio::test]
async fn d7_late_vault_failure_is_global_and_actually_reaches_sdk_acquisition() {
    let vendors = Vendors::new().await;
    let vault = Vault::new().await;
    vault.server.reset().await;
    mount(
        &vault.server,
        "GET",
        "/v1/auth/token/lookup-self",
        ResponseTemplate::new(200).set_body_json(
            serde_json::json!({"data": {"ttl": 600, "policies": ["connector-read"]}}),
        ),
    )
    .await;
    mount(
        &vault.server,
        "GET",
        "/v1/secret/data/service/oauth",
        ResponseTemplate::new(403).set_body_string("vault-refusal-sentinel"),
    )
    .await;
    mount(
        &vault.server,
        "GET",
        "/v1/secret/data/service/hmac",
        ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"data": {"data": {"key": RAW_KEY}}})),
    )
    .await;
    let mut config = vendors.config();
    Vault::provider(&mut config.providers[1]);
    config.openbao = Some(vault.config());
    rejected(&build_clients(&config).await);
    vault.denied_read().await;
    assert_eq!(vendors.count(false, "/token").await, 0);
    assert_eq!(vendors.count(true, "/token").await, 0);
}

#[tokio::test]
async fn d7_duplicate_catalogue_is_rejected_before_oidc_acquisition() {
    let vendors = Vendors::new().await;
    let mut config = vendors.config();
    config.providers[1].id = "forge".into();
    rejected(&Application::new(config).await);
    assert!(vendors.requests(true).await.is_empty());
    assert!(vendors.requests(false).await.is_empty());
}

#[tokio::test]
async fn d7_composed_oidc_decodes_real_gzip_discovery_jwks_and_signed_token() {
    let vendors = Vendors::new().await;
    vendors.oidc.reset().await;
    for (verb, path, value) in [
        ("GET", DISCOVERY, metadata(&vendors.oidc.uri())),
        ("GET", "/keys", d_vendor::jwks()),
        ("POST", "/token", token(&vendors.oidc.uri())),
    ] {
        let bytes = must(serde_json::to_vec(&value), "gzip JSON fixture");
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        must(encoder.write_all(&bytes), "gzip fixture input");
        let compressed = must(encoder.finish(), "gzip fixture finalization");
        mount(
            &vendors.oidc,
            verb,
            path,
            ResponseTemplate::new(200)
                .set_body_bytes(compressed)
                .insert_header("content-type", "application/json")
                .insert_header("content-encoding", "gzip"),
        )
        .await;
    }
    let clients = must(
        build_clients(&vendors.config()).await,
        "actual composed gzip OIDC transport",
    );
    let result = must(
        selected(&clients, "authority")
            .client
            .complete_authorization(&complete())
            .await,
        "composed gzip OIDC complete",
    );
    vendors.assert_profile(
        &must(serde_json::to_value(result), "gzip profile DTO"),
        true,
    );
    for route in [DISCOVERY, "/keys", "/token"] {
        assert_eq!(vendors.count(true, route).await, 1);
    }
}

async fn probe_document(
    http: &reqwest::Client,
    endpoint: &str,
    expected: &Value,
    phase: &str,
    hits: &std::sync::atomic::AtomicUsize,
    expected_hits: usize,
) {
    let response = match http.get(endpoint).send().await {
        Ok(response) => response,
        Err(error) => {
            let category = if error.is_timeout() {
                "timeout"
            } else if error.is_connect() {
                "connect"
            } else {
                "other"
            };
            panic!(
                "TLS preflight phase={phase} category={category} hits={}",
                hits.load(Ordering::SeqCst)
            );
        }
    };
    assert!(
        response.status() == reqwest::StatusCode::OK,
        "TLS preflight phase={phase} non-200 hits={}",
        hits.load(Ordering::SeqCst)
    );
    assert!(
        response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            == Some("application/json"),
        "TLS preflight phase={phase} non-JSON-content-type hits={}",
        hits.load(Ordering::SeqCst)
    );
    let (_, bytes) = response_bytes(response).await;
    let Ok(document) = serde_json::from_slice::<Value>(&bytes) else {
        panic!(
            "TLS preflight phase={phase} invalid-JSON hits={}",
            hits.load(Ordering::SeqCst)
        );
    };
    assert!(
        document == *expected,
        "TLS preflight phase={phase} unexpected-document hits={}",
        hits.load(Ordering::SeqCst)
    );
    assert_eq!(
        hits.load(Ordering::SeqCst),
        expected_hits,
        "TLS preflight phase={phase} handler count"
    );
}

#[tokio::test]
async fn d_fixture_tls_oidc_preflight_is_independent_of_the_mixed_builder() {
    d_tls::run(true, |view| async move {
        let vendors = Vendors::new().await;
        let mut provider = vendors.config().providers.remove(1);
        provider.profile = ProviderProfile::Oidc(OidcProfile {
            issuer: view.origin.clone(),
            allowed_endpoint_origins: vec![view.origin.clone()],
        });
        provider.transport_security = VendorTransportSecurity::Verified;
        assert!(
            oauth_connect_configuration::validate_provider(&provider).is_ok(),
            "TLS preflight phase=configuration hits={}",
            view.hits.load(Ordering::SeqCst)
        );
        let root = must(std::fs::read(view.ca), "fixture CA bytes");
        let root = must(
            reqwest::Certificate::from_pem(&root),
            "fixture root parsing",
        );
        let http = must(
            reqwest::Client::builder()
                .no_proxy()
                .add_root_certificate(root)
                .build(),
            "explicitly trusted fixture client",
        );
        probe_document(
            &http,
            &format!("{}{DISCOVERY}", view.origin),
            &metadata(&view.origin),
            "discovery",
            &view.hits,
            1,
        )
        .await;
        probe_document(
            &http,
            &format!("{}/keys", view.origin),
            &d_vendor::jwks(),
            "jwks",
            &view.hits,
            2,
        )
        .await;
        let baseline = view.hits.load(Ordering::SeqCst);
        let secret = must(SecretValue::new(SECRET_OIDC.into()), "fixture secret");
        let Ok(client) = OidcClient::new(&provider, secret, http).await else {
            panic!(
                "TLS preflight phase=oidc-constructor handler_delta={}",
                view.hits.load(Ordering::SeqCst) - baseline
            );
        };
        let result = must(
            client.complete_authorization(&complete()).await,
            "TLS fixture signed complete",
        );
        assert!(result.profile.id == "000042");
        assert_eq!(view.hits.load(Ordering::SeqCst), 5);
    })
    .await;

    // Qualify the repaired PKI independently of the still-refusing C builder.
    for (matching_san, trusted_root) in [(false, true), (true, false)] {
        d_tls::run(matching_san, move |view| async move {
            let mut builder = reqwest::Client::builder().no_proxy();
            if trusted_root {
                let pem = must(std::fs::read(view.ca), "negative fixture CA bytes");
                let root = must(
                    reqwest::Certificate::from_pem(&pem),
                    "negative fixture root",
                );
                builder = builder.add_root_certificate(root);
            }
            let http = must(builder.build(), "negative fixture client");
            let Err(error) = http.get(format!("{}{DISCOVERY}", view.origin)).send().await else {
                panic!("TLS preflight accepted wrong SAN or untrusted CA");
            };
            assert!(
                error.is_connect() && !error.is_timeout(),
                "TLS preflight negative failed outside connection verification"
            );
            assert_eq!(view.hits.load(Ordering::SeqCst), 0);
        })
        .await;
    }
}

#[tokio::test]
async fn d7_composed_oidc_verifies_ca_and_san_on_initialization_and_token() {
    for (matching, trusted) in [(true, true), (false, true), (true, false)] {
        d_tls::run(matching, move |view| async move {
            let vendors = Vendors::new().await;
            let mut config = vendors.config();
            let provider = &mut config.providers[1];
            provider.profile = ProviderProfile::Oidc(OidcProfile {
                issuer: view.origin.clone(),
                allowed_endpoint_origins: vec![view.origin],
            });
            provider.transport_security = VendorTransportSecurity::Verified;
            provider.ca_cert_file = if trusted { Some(view.ca) } else { None };
            let result = build_clients(&config).await;
            if matching && trusted {
                let clients = must(result, "actual composed CA/SAN OIDC transport");
                let result = must(
                    selected(&clients, "authority")
                        .client
                        .complete_authorization(&complete())
                        .await,
                    "verified TLS OIDC complete",
                );
                let value = must(serde_json::to_value(result), "TLS identity DTO");
                assert!(value["profile"]["id"].as_str() == Some("000042"));
                assert!(value["tokens"]["access_token"].as_str() == Some("oidc-access-sentinel"));
                assert_eq!(view.hits.load(Ordering::SeqCst), 3);
            } else {
                rejected(&result);
                assert_eq!(
                    view.hits.load(Ordering::SeqCst),
                    0,
                    "untrusted CA/wrong SAN refused before HTTP handlers"
                );
            }
        })
        .await;
    }
}

async fn platform_ready(origin: &str, http: &reqwest::Client) {
    for route in ["health", "ready"] {
        let (status, bytes) = response_bytes(must(
            http.get(format!("{origin}/oauth-connect/{route}"))
                .send()
                .await,
            "platform endpoint",
        ))
        .await;
        assert_eq!(status, reqwest::StatusCode::OK);
        clean(&String::from_utf8_lossy(&bytes));
        if route == "ready" {
            let report: Value = must(serde_json::from_slice(&bytes), "readiness JSON");
            assert!(report["status"].as_str() == Some("ready"));
        }
    }
}

async fn application_flows(origin: &str, http: &reqwest::Client, vendors: &Vendors) {
    for (id, key, oidc) in [("forge", KEY_OAUTH, false), ("authority", KEY_OIDC, true)] {
        let (status, bytes) = signed_call(http, origin, id, false, key).await;
        assert_eq!(status, reqwest::StatusCode::OK);
        let dto: Value = must(serde_json::from_slice(&bytes), "authorize DTO");
        let url = must(
            url::Url::parse(dto["authorization_url"].as_str().expect("authorize URL")),
            "authorize URL parse",
        );
        let query: std::collections::BTreeMap<_, _> = url.query_pairs().into_owned().collect();
        assert!(query.get("code_challenge").is_some_and(|v| v == CHALLENGE));
        assert_eq!(
            query.get("nonce").map(String::as_str),
            if oidc { Some(NONCE) } else { None }
        );
        let (status, bytes) = signed_call(http, origin, id, true, key).await;
        assert_eq!(status, reqwest::StatusCode::OK);
        vendors.assert_profile(
            &must(serde_json::from_slice(&bytes), "listener complete DTO"),
            oidc,
        );
        vendors.credentials(oidc).await;
    }
}

fn verified_platform_bearer_fixture() -> String {
    // Genuine RS256 access token with UUID subject; no platform trust is configured.
    let now = chrono::Utc::now().timestamp();
    let claims = serde_json::json!({"sub": "11111111-1111-4111-8111-111111111111",
        "iss": "https://platform.example.test", "aud": "oauth-connect",
        "iat": now - 5, "nbf": now - 5, "exp": now + 3600});
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.typ = Some("aiforall-access+jwt".into());
    header.kid = Some("service-key".into());
    let encoding = jsonwebtoken::EncodingKey::from_rsa_pem(include_bytes!(
        "../../../IAMRusty/config/keys/test-platform.pem"
    ))
    .unwrap_or_else(|_| panic!("access-token private fixture could not be parsed"));
    let bearer = jsonwebtoken::encode(&header, &claims, &encoding)
        .unwrap_or_else(|_| panic!("access-token fixture could not be signed"));
    let decoding = jsonwebtoken::DecodingKey::from_rsa_pem(include_bytes!(
        "../../../IAMRusty/config/keys/test-platform.pub"
    ))
    .unwrap_or_else(|_| panic!("access-token public fixture could not be parsed"));
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_issuer(&["https://platform.example.test"]);
    validation.set_audience(&["oauth-connect"]);
    validation.validate_nbf = true;
    let verified =
        jsonwebtoken::decode::<Value>(&bearer, &decoding, &validation).unwrap_or_else(|_| {
            panic!("access-token fixture failed independent cryptographic verification")
        });
    assert!(
        verified.claims == claims,
        "independent fixture verification must preserve the UUID-bearing claims"
    );
    bearer
}

// §10/G-7: a signed fixture Bearer cannot replace S2S auth or activate provider I/O.
async fn hmac_only_bearer_boundary(origin: &str, http: &reqwest::Client, vendors: &Vendors) {
    let authorize_path = "/oauth-connect/authority/v2/authorize";
    let authorize_body = body(false);
    let (at, mac) = signature(authorize_path, &authorize_body, KEY_OIDC);
    let response = must(
        http.post(format!("{origin}{authorize_path}"))
            .header("content-type", "application/json")
            .header("authorization", "arbitrary-not-a-bearer-credential")
            .header(TIMESTAMP_HEADER, at.to_string())
            .header(SIGNATURE_HEADER, mac)
            .body(authorize_body.clone())
            .send()
            .await,
        "valid HMAC authorize with irrelevant Authorization",
    );
    let (status, bytes) = response_bytes(response).await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let authorized: Value = must(
        serde_json::from_slice(&bytes),
        "HMAC-only authorize response",
    );
    assert!(
        authorized["authorization_url"].as_str().is_some(),
        "valid HMAC must still authorize"
    );
    let bearer = verified_platform_bearer_fixture();
    let mut counts = Vec::new();
    for oidc in [false, true] {
        for route in [DISCOVERY, "/keys", "/token"] {
            counts.push(vendors.count(oidc, route).await);
        }
    }
    for wrong_hmac in [false, true] {
        let mut request = http
            .post(format!("{origin}{authorize_path}"))
            .header("content-type", "application/json")
            .bearer_auth(&bearer)
            .body(authorize_body.clone());
        if wrong_hmac {
            let (at, mac) = signature(authorize_path, &authorize_body, KEY_OAUTH);
            request = request
                .header(TIMESTAMP_HEADER, at.to_string())
                .header(SIGNATURE_HEADER, mac);
        }
        let response = must(request.send().await, "Bearer cannot replace valid S2S HMAC");
        let (status, bytes) = response_bytes(response).await;
        assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED);
        assert!(
            bytes.is_empty(),
            "Bearer refusal must expose no contextual content"
        );
        let mut after = Vec::new();
        for oidc in [false, true] {
            for route in [DISCOVERY, "/keys", "/token"] {
                after.push(vendors.count(oidc, route).await);
            }
        }
        assert_eq!(
            after, counts,
            "Bearer-only/wrong-HMAC must not trigger provider acquisition or token exchange"
        );
    }
}

async fn application_refusals(origin: &str, http: &reqwest::Client, vendors: &Vendors) {
    hmac_only_bearer_boundary(origin, http, vendors).await;
    let before = vendors.count(true, "/token").await;
    let (status, bytes) = signed_call(http, origin, "authority", true, KEY_OAUTH).await;
    assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED);
    assert!(bytes.is_empty());
    assert_eq!(vendors.count(true, "/token").await, before);
    // Valid S2S MAC, canonical nonce shape, but wrong OIDC nonce value.
    let path = "/oauth-connect/authority/v2/complete";
    let mut wrong_context = complete();
    wrong_context.expected_nonce = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into();
    let bytes = must(
        serde_json::to_string(&wrong_context),
        "wrong nonce context fixture",
    );
    let (at, mac) = signature(path, &bytes, KEY_OIDC);
    let response = must(
        http.post(format!("{origin}{path}"))
            .header("content-type", "application/json")
            .header(TIMESTAMP_HEADER, at.to_string())
            .header(SIGNATURE_HEADER, mac)
            .body(bytes)
            .send()
            .await,
        "signed wrong nonce request",
    );
    let (status, bytes) = response_bytes(response).await;
    assert_eq!(status, reqwest::StatusCode::BAD_GATEWAY);
    assert!(
        bytes.is_empty(),
        "nonce mismatch must not expose tokens or partial identity"
    );
    assert_eq!(vendors.count(true, "/token").await, before + 1);
}

#[tokio::test]
async fn d7_application_port_zero_signed_flows_readiness_and_rebuild_without_authorize() {
    let vendors = Arc::new(Vendors::new().await);
    let config = vendors.config();
    let app = must(
        AppBuilder::new(config.clone()).build().await,
        "complete Application builder",
    );
    let server = must(app.start().await, "Application owned port0 start");
    let first_vendors = vendors.clone();
    d_owned::run(server, move |origin| async move {
        let http = incoming_client();
        assert_eq!(first_vendors.count(false, "/token").await, 0);
        assert_eq!(first_vendors.count(true, "/token").await, 0);
        platform_ready(&origin, &http).await;
        application_flows(&origin, &http, &first_vendors).await;
        application_refusals(&origin, &http, &first_vendors).await;
    })
    .await;
    // Same issuer, keys, nonce/verifier and callback, entirely new Application.
    drop(app);
    let rebuilt = must(
        Application::new(config).await,
        "rebuilt stateless application",
    );
    let before = vendors.count(true, "/token").await;
    d_owned::run(
        must(rebuilt.start().await, "rebuilt start"),
        move |origin| async move {
            let (status, bytes) =
                signed_call(&incoming_client(), &origin, "authority", true, KEY_OIDC).await;
            assert_eq!(status, reqwest::StatusCode::OK);
            vendors.assert_profile(
                &must(serde_json::from_slice(&bytes), "rebuilt complete DTO"),
                true,
            );
            assert_eq!(vendors.count(true, "/authorize").await, 0);
            assert_eq!(vendors.count(true, "/token").await, before + 1);
        },
    )
    .await;
}

#[tokio::test]
async fn d7_valid_application_bind_conflict_and_invalid_local_tls_fail_without_owned_server() {
    for invalid_tls in [false, true] {
        let vendors = Vendors::new().await;
        let blocker = std::net::TcpListener::bind("127.0.0.1:0")
            .expect("held conflict listener, not bind-and-close reservation");
        let mut config = vendors.config();
        if invalid_tls {
            config.server.tls_enabled = true;
            config.server.tls_port = 0;
            config.server.tls_cert_path = "missing-service-cert-sentinel.pem".into();
            config.server.tls_key_path = "missing-service-key-sentinel.pem".into();
        } else {
            config.server.port = blocker.local_addr().expect("held address").port();
        }
        let app = must(
            Application::new(config).await,
            "valid catalogue before listen failure",
        );
        match app.start().await {
            Err(error) => {
                clean(&format!("{error:?} {error}"));
            }
            Ok(server) => {
                d_owned::run(server, |_| async {
                    panic!("failed bind/TLS unexpectedly admitted a server");
                })
                .await;
            }
        }
        assert_eq!(vendors.count(true, "/token").await, 0);
    }
}

#[tokio::test]
async fn d7_real_loader_child_validates_mixed_input_before_composition() {
    const NAME: &str = "d7_real_loader_child_validates_mixed_input_before_composition";
    if std::env::var("OC_D_LOADER_CHILD").is_ok_and(|v| v == NAME) {
        let vendors = Vendors::new().await;
        std::fs::write("config/test.toml", source(&vendors.config()))
            .expect("isolated loader source");
        let config = must(load_config(), "actual SDK mixed loader");
        assert_eq!(config.providers.len(), 2);
        assert_eq!(config.providers[1].id, "authority");
        assert_eq!(config.server.port, 0);
        println!("D_INPUT_VALIDATED");
        let clients = must(build_clients(&config).await, "loaded mixed composition");
        assert_eq!(clients.len(), 2);
        let result = must(
            selected(&clients, "authority")
                .client
                .complete_authorization(&complete())
                .await,
            "loaded mixed complete",
        );
        vendors.assert_profile(&must(serde_json::to_value(result), "loaded identity"), true);
        println!("D_LOADED_ORACLE_COMPLETE");
        return;
    }
    // Process ownership/drains are shared with the real-binary tests, not another loader.
    let directory = tempfile::tempdir().expect("isolated loader child directory");
    std::fs::create_dir(directory.path().join("config")).expect("isolated config directory");
    let mut child = d_process::Process::spawn(
        &std::env::current_exe().expect("test executable"),
        directory.path(),
        &["--exact", NAME, "--nocapture"],
        Some(("OC_D_LOADER_CHILD", NAME)),
    );
    let outcome = child.finish(Duration::from_secs(15));
    assert!(
        outcome.contains("D_INPUT_VALIDATED"),
        "fixture loader input not validated; not a composition RED"
    );
    assert!(outcome.success, "validated loader child composition failed");
    assert!(
        outcome.contains("D_LOADED_ORACLE_COMPLETE"),
        "child must execute full composition oracle"
    );
}

#[path = "../../tests/support/d_process.rs"]
mod d_process;
