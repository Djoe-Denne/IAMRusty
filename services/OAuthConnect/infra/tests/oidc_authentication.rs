//! A / G-1–G-3: real standard OIDC against signed local HTTP collaborators.
//!
//! Obligation -> risk -> scenario -> oracle:
//! - G1 pinned discovery/bounded callback -> SSRF or unbounded decode -> genuine
//!   metadata/JWKS/token/`UserInfo`, origin/status/size/encoding mutations -> exact
//!   profile or closed refusal, endpoint receipts and no redirect destination I/O.
//! - G2 validated ID token -> untrusted identity -> RS256 nominal then independently
//!   signed claim/signature mutants -> exact fixed DTO or error with POST once/UI0.
//! - G3 provenance -> false verified email -> conditional `UserInfo` and non-null
//!   conflicts -> source-bound values, exact subject, zero/one `UserInfo` requests.
//!
//! Preflights do not call `OidcClient`. Nominal constructor refusal is business RED,
//! not fixture/compile evidence. Invalid-declaration refusals are characterizations
//! until the adjacent nominal client paths pass.
//! Crypto/fixture helper code is test-only; no decoded unsigned identity oracle.

#[path = "support/oidc_fixture.rs"]
mod fixture;

#[path = "support/oidc_http.rs"]
mod http_fixture;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use fixture::{
    authentication_failed, completion, form, http, json_response, jwk, jwks, key_a, key_b, must,
    profile, redacted, refused, secret, sign, OidcFixture, ACCESS, CALLBACK, CLIENT_ID, DISCOVERY,
    JWKS, KID, SECRET, SUBJECT, TOKEN, USERINFO, VERIFIER,
};
use http_fixture::{
    access_hash, advertised_decoding_key, authorize, body, duplicate_field, gzip, padded,
    CHALLENGE, LIMIT,
};
use idp_connect_contract::{FederatedOAuthClient, FederatedOAuthError};
use jsonwebtoken::{Algorithm, Header, Validation};
use oauth_connect_configuration::{validate_config, AppConfig, ProviderProfile};
use oauth_connect_infra::OidcClient;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use wiremock::{matchers::path, Mock, ResponseTemplate};

async fn initialized(f: &OidcFixture) -> OidcClient {
    must(
        validate_config(&AppConfig {
            providers: vec![f.provider.clone()],
            ..AppConfig::default()
        }),
        "declaration fixture preflight (failure is not OIDC business RED)",
    );
    must(
        OidcClient::new(&f.provider, secret(), http()).await,
        "valid OIDC discovery and initial JWKS",
    )
}

fn default_profile(f: &OidcFixture, auth: &idp_connect_contract::ProviderAuthentication) {
    profile(
        auth,
        SUBJECT,
        "name",
        Some("one@example.test"),
        true,
        Some(&f.url("/avatar")),
    );
}

fn remove(claims: &mut Value, key: &str) {
    let Some(object) = claims.as_object_mut() else {
        panic!("fixture claims object required");
    };
    object.remove(key);
}

#[tokio::test]
async fn preflight_rs256_keys_jwks_metadata_and_hash_fixture_are_independently_valid() {
    let f = OidcFixture::new().await;
    let metadata: openidconnect::core::CoreProviderMetadata = must(
        serde_json::from_value(f.metadata()),
        "standard metadata parser",
    );
    let keys: openidconnect::core::CoreJsonWebKeySet = must(
        serde_json::from_value(jwks(key_a(), KID)),
        "standard JWKS parser",
    );
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[f.issuer()]);
    validation.set_audience(&[CLIENT_ID]);
    let token = sign(key_a(), KID, &f.claims_at(Utc::now()));
    let verified = must(
        jsonwebtoken::decode::<Value>(&token, &advertised_decoding_key(key_a()), &validation),
        "independent signed RSA preflight",
    );
    assert!(
        verified.claims["sub"] == SUBJECT,
        "real signature verified before reading subject"
    );
    assert!(
        jsonwebtoken::decode::<Value>(&token, &advertised_decoding_key(key_b()), &validation)
            .is_err(),
        "second RSA key is distinct"
    );
    let hs = must(
        jsonwebtoken::encode(
            &Header::new(Algorithm::HS256),
            &f.claims_at(Utc::now()),
            &jsonwebtoken::EncodingKey::from_secret(SECRET.as_bytes()),
        ),
        "cryptographically valid HS mutant preflight",
    );
    let mut hs_validation = Validation::new(Algorithm::HS256);
    hs_validation.set_issuer(&[f.issuer()]);
    hs_validation.set_audience(&[CLIENT_ID]);
    let verified_hs = must(
        jsonwebtoken::decode::<Value>(
            &hs,
            &jsonwebtoken::DecodingKey::from_secret(SECRET.as_bytes()),
            &hs_validation,
        ),
        "HS fixture signature must be correct so algorithm policy is the only rejection cause",
    );
    assert!(verified_hs.claims["sub"] == SUBJECT);
    assert_eq!(keys.keys().len(), 1, "one standard eligible RSA JWK");
    assert!(
        metadata.issuer().as_str() == f.issuer(),
        "pinned fixture issuer"
    );
    // SHA256("abc") left half: ba7816bf8f01cfea414140de5dae2223.
    assert_eq!(
        access_hash("abc"),
        "ungWv48Bz-pBQUDeXa4iIw",
        "fixed independent at_hash vector"
    );
    let duplicate_metadata = duplicate_field(&f.metadata(), "issuer", &json!(f.issuer()));
    assert!(
        serde_json::from_slice::<openidconnect::core::CoreProviderMetadata>(&duplicate_metadata)
            .is_err(),
        "independent parser preflight: duplicate known metadata field is rejected"
    );
    let response = OidcFixture::token_body(&token);
    let duplicate_token = duplicate_field(&response, "access_token", &json!(ACCESS));
    assert!(
        serde_json::from_slice::<openidconnect::core::CoreTokenResponse>(&duplicate_token).is_err(),
        "independent parser preflight: duplicate known token field is rejected"
    );
    f.assert_counts(0, 0, 0, 0).await;
}

#[tokio::test]
async fn preflight_isolated_discovery_and_jwks_documents_are_reachable_without_oidc_client() {
    let f = OidcFixture::new().await;
    let http = http();
    for route in [DISCOVERY, JWKS] {
        let response = must(http.get(f.url(route)).send().await, "fixture endpoint HTTP");
        assert_eq!(response.status().as_u16(), 200, "fixture response status");
        let value: Value = must(response.json().await, "small fixture-only HTTP JSON");
        assert!(value.is_object(), "fixture document object");
    }
    f.keys.wait_count(1).await;
    // Verify the actual signed UserInfo wire response independently of OidcClient.
    let info = json!({"iss": f.issuer(), "aud": CLIENT_ID, "sub": SUBJECT,
        "email": "one@example.test", "email_verified": true});
    let signed = sign(key_a(), KID, &info);
    f.userinfo
        .set(fixture::signed_userinfo_response(signed.clone()));
    let mut response = must(
        http.get(f.url(USERINFO)).send().await,
        "signed fixture HTTP",
    );
    assert_eq!(response.status().as_u16(), 200, "signed fixture status");
    assert!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            == Some("application/jwt"),
        "signed fixture must actually deliver application/jwt"
    );
    assert!(signed.len() <= 8192, "signed fixture must remain small");
    let mut received = Vec::new();
    while let Some(chunk) = must(response.chunk().await, "signed fixture body chunk") {
        assert!(
            chunk.len() <= 8192 - received.len(),
            "signed fixture body must remain within the preflight byte bound"
        );
        received.extend_from_slice(&chunk);
    }
    assert!(
        received.as_slice() == signed.as_bytes(),
        "signed fixture must deliver exact original JWT bytes"
    );
    f.assert_counts(1, 1, 0, 1).await;
}

#[tokio::test]
async fn g1_nominal_startup_authorize_and_complete_use_one_initial_jwks_and_fixed_output() {
    let f = OidcFixture::new().await;
    let client = initialized(&f).await;
    f.assert_counts(1, 1, 0, 0).await;
    let request = authorize();
    let response = must(
        client.authorize_authentication(&request).await,
        "OIDC authorize",
    );
    let url = must(
        url::Url::parse(&response.authorization_url),
        "authorize URL",
    );
    let mut endpoint = url.clone();
    endpoint.set_query(None);
    assert!(
        endpoint.as_str() == f.url("/authorize"),
        "exact discovered authorize endpoint, no fragment or credentials"
    );
    let pairs: Vec<_> = url.query_pairs().into_owned().collect();
    let params: std::collections::BTreeMap<_, _> = pairs.iter().cloned().collect();
    assert_eq!(
        pairs.len(),
        params.len(),
        "no duplicate parameters or openid scope"
    );
    for (key, value) in [
        ("response_type", "code"),
        ("client_id", CLIENT_ID),
        ("state", request.state.as_str()),
        ("redirect_uri", CALLBACK),
        ("code_challenge", CHALLENGE),
        ("code_challenge_method", "S256"),
        ("nonce", request.nonce.as_str()),
    ] {
        assert!(
            params.get(key).map(String::as_str) == Some(value),
            "exact OIDC authorize field {key}"
        );
    }
    let scopes = params
        .get("scope")
        .map_or_else(|| panic!("scope required"), std::convert::identity);
    let mut scopes: Vec<_> = scopes.split_whitespace().collect();
    scopes.sort_unstable();
    assert_eq!(
        scopes,
        ["email", "openid", "profile"],
        "openid exactly once; no implicit scopes"
    );
    let mut advertised: Vec<_> = response.scope.split_whitespace().collect();
    advertised.sort_unstable();
    assert_eq!(
        advertised,
        ["email", "openid", "profile"],
        "advertised scope matches request"
    );
    assert!(!params.contains_key("code_verifier"));
    let auth = must(
        client
            .complete_authorization(&completion("oidc-code-nominal"))
            .await,
        "RS256 authentication",
    );
    default_profile(&f, &auth);
    redacted(&format!("{client:?}"));
    f.assert_counts(1, 1, 1, 0).await;
    let receipts = f.receipts().await;
    let post = receipts
        .iter()
        .find(|r| r.url.path() == TOKEN)
        .map_or_else(|| panic!("token POST required"), std::convert::identity);
    assert!(
        post.headers.get("authorization").is_none(),
        "client_secret_post must not also send Authorization credentials"
    );
    assert!(
        post.method.as_str() == "POST" && post.url.query().is_none(),
        "code only in form, never URL"
    );
    let fields = form(post);
    for (key, value) in [
        ("grant_type", "authorization_code"),
        ("code", "oidc-code-nominal"),
        ("redirect_uri", CALLBACK),
        ("code_verifier", VERIFIER),
        ("client_id", CLIENT_ID),
        ("client_secret", SECRET),
    ] {
        assert!(
            fields.get(key).map(String::as_str) == Some(value),
            "exact decoded token field {key}"
        );
    }
}

#[tokio::test]
async fn g1_discovery_authorization_endpoint_rejects_reserved_query_parameters() {
    // OIDC-S1 / G-1, contract §1: reject reserved names, not all endpoint queries.
    let nominal = OidcFixture::new().await;
    let mut metadata = nominal.metadata();
    metadata["authorization_endpoint"] = json!(nominal.url("/authorize?tenant=oidc-team"));
    nominal.discovery.json(&metadata);
    let client = initialized(&nominal).await;
    let request = authorize();
    let response = must(
        client.authorize_authentication(&request).await,
        "nonreserved discovery query must remain allowed",
    );
    let url = must(
        url::Url::parse(&response.authorization_url),
        "authorize URL",
    );
    let pairs: Vec<_> = url.query_pairs().into_owned().collect();
    for (name, expected) in [
        ("tenant", "oidc-team"),
        ("state", request.state.as_str()),
        ("nonce", request.nonce.as_str()),
    ] {
        let values: Vec<_> = pairs
            .iter()
            .filter(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
            .collect();
        assert!(
            values.as_slice() == [expected],
            "authorize must preserve tenant and exactly one caller security context"
        );
    }
    nominal.assert_counts(1, 1, 0, 0).await;

    // Exact existing OAuth policy (configuration/validation.rs), independently
    // enumerated as test data; no dependency on a future production guard.
    for (plain, encoded) in [
        ("state", "%73tate"),
        ("redirect_uri", "%72edirect_uri"),
        ("client_id", "%63lient_id"),
        ("scope", "%73cope"),
        ("response_type", "%72esponse_type"),
        ("code_challenge", "%63ode_challenge"),
        ("code_challenge_method", "%63ode_challenge_method"),
        ("code_verifier", "%63ode_verifier"),
        ("nonce", "%6Eonce"),
    ] {
        for name in <[&str; 2]>::from((plain, encoded)) {
            let f = OidcFixture::new().await;
            let mut metadata = f.metadata();
            metadata["authorization_endpoint"] =
                json!(f.url(&format!("/authorize?tenant=oidc-team&{name}=injected")));
            f.discovery.json(&metadata);
            let result = OidcClient::new(&f.provider, secret(), http()).await;
            refused(&result, "reserved discovery authorization query");
            f.assert_counts(1, 0, 0, 0).await;
        }
    }
}

#[tokio::test]
async fn g1_direct_v2_calls_reject_redirect_nonce_and_pkce_before_token_io() {
    let f = OidcFixture::new().await;
    let client = initialized(&f).await;
    let mut bad_authorize = Vec::new();
    let mut a = authorize();
    a.redirect_uri.push_str("&extra=1");
    bad_authorize.push(a);
    let mut a = authorize();
    a.nonce.clear();
    bad_authorize.push(a);
    let mut a = authorize();
    a.code_challenge = None;
    bad_authorize.push(a);
    let mut a = authorize();
    a.state.clear();
    bad_authorize.push(a);
    let mut a = authorize();
    a.code_challenge_method = Some("plain".to_owned());
    bad_authorize.push(a);
    for request in bad_authorize {
        assert!(matches!(
            client.authorize_authentication(&request).await,
            Err(FederatedOAuthError::Authorize)
        ));
    }
    let mut invalid = Vec::new();
    let mut r = completion("oidc-code-invalid");
    r.redirect_uri.push_str("/suffix");
    invalid.push(r);
    let mut r = completion("oidc-code-invalid");
    r.expected_nonce = "short".to_owned();
    invalid.push(r);
    let mut r = completion("oidc-code-invalid");
    r.code.clear();
    invalid.push(r);
    let mut r = completion("oidc-code-invalid");
    r.code_verifier = None;
    invalid.push(r);
    let mut r = completion("oidc-code-invalid");
    r.code_verifier = Some("bad+chars".repeat(6));
    invalid.push(r);
    for request in invalid {
        authentication_failed(&client.complete_authorization(&request).await);
    }
    // Legacy surfaces cannot silently authenticate without v2's expected nonce.
    assert!(matches!(
        client.authorize(CALLBACK, "legacy-state").await,
        Err(FederatedOAuthError::Authorize)
    ));
    assert!(matches!(
        client
            .authorize_with_pkce(CALLBACK, "legacy-state", Some(CHALLENGE))
            .await,
        Err(FederatedOAuthError::Authorize)
    ));
    assert!(matches!(
        client.exchange_code("oidc-code-legacy", CALLBACK).await,
        Err(FederatedOAuthError::ExchangeCode)
    ));
    assert!(matches!(
        client
            .exchange_code_with_pkce("oidc-code-legacy", CALLBACK, Some(VERIFIER))
            .await,
        Err(FederatedOAuthError::ExchangeCode)
    ));
    assert!(matches!(
        client.user_profile(ACCESS).await,
        Err(FederatedOAuthError::UserProfile)
    ));
    f.assert_counts(1, 1, 0, 0).await;
}

#[tokio::test]
async fn g1_invalid_oidc_declarations_refuse_before_any_discovery_io() {
    for mutation in ["transport", "pkce", "openid"] {
        let mut f = OidcFixture::new().await;
        match mutation {
            "transport" => {
                f.provider.transport_security =
                    oauth_connect_configuration::VendorTransportSecurity::Verified;
            }
            "pkce" => f.provider.pkce_supported = false,
            _ => f.provider.scopes.retain(|scope| scope != "openid"),
        }
        refused(
            &OidcClient::new(&f.provider, secret(), http()).await,
            "invalid OIDC declaration",
        );
        f.assert_counts(0, 0, 0, 0).await;
    }
}

#[tokio::test]
async fn g1_explicitly_allowed_second_origin_can_supply_jwks_without_extra_discovery() {
    let mut f = OidcFixture::new().await;
    let other = OidcFixture::new().await;
    let ProviderProfile::Oidc(config) = &mut f.provider.profile else {
        panic!("OIDC declaration");
    };
    config.allowed_endpoint_origins.push(other.issuer());
    let mut metadata = f.metadata();
    metadata["jwks_uri"] = json!(other.url(JWKS));
    f.discovery.json(&metadata);
    let client = initialized(&f).await;
    let auth = must(
        client
            .complete_authorization(&completion("oidc-code-cross-origin"))
            .await,
        "explicitly approved JWKS origin",
    );
    default_profile(&f, &auth);
    f.assert_counts(1, 0, 1, 0).await;
    other.assert_counts(0, 1, 0, 0).await;
}

#[tokio::test]
async fn g1_forbidden_endpoint_origin_never_receives_even_the_internal_discovery_jwks_get() {
    for field in [
        "jwks_uri",
        "token_endpoint",
        "userinfo_endpoint",
        "authorization_endpoint",
    ] {
        for variant in ["origin", "credentials", "fragment"] {
            let f = OidcFixture::new().await;
            let forbidden = OidcFixture::new().await;
            let mut destination = if variant == "origin" {
                must(
                    url::Url::parse(&forbidden.url("/target")),
                    "foreign-origin fixture",
                )
            } else {
                must(
                    url::Url::parse(&f.url("/target")),
                    "local forbidden URL fixture",
                )
            };
            if variant == "credentials" {
                must(
                    destination.set_username("hidden-user"),
                    "URL username fixture",
                );
                must(
                    destination.set_password(Some(SECRET)),
                    "URL password fixture",
                );
            }
            if variant == "fragment" {
                destination.set_fragment(Some(SECRET));
            }
            let mut metadata = f.metadata();
            metadata[field] = json!(destination.as_str());
            f.discovery.json(&metadata);
            refused(
                &OidcClient::new(&f.provider, secret(), http()).await,
                "unapproved discovered endpoint",
            );
            assert_eq!(
                f.discovery.count(),
                1,
                "discovery attempted on pinned origin"
            );
            forbidden.assert_counts(0, 0, 0, 0).await;
            assert_eq!(f.token.count(), 0, "no token operation at failed boot");
            assert!(
                f.receipts()
                    .await
                    .iter()
                    .all(|r| matches!(r.url.path(), DISCOVERY | JWKS)),
                "no credential-bearing/fragment target request before validation"
            );
        }
    }
}

#[tokio::test]
async fn g1_discovery_issuer_auth_method_and_invalid_metadata_fail_closed() {
    for mutation in [
        "issuer",
        "token_endpoint_auth_methods_supported",
        "id_token_signing_alg_values_supported",
        "missing",
    ] {
        let f = OidcFixture::new().await;
        let mut metadata = f.metadata();
        match mutation {
            "issuer" => metadata["issuer"] = json!(f.url("/different-issuer")),
            "token_endpoint_auth_methods_supported" => {
                metadata[mutation] = json!(["private_key_jwt"]);
            }
            "id_token_signing_alg_values_supported" => metadata[mutation] = json!(["HS256"]),
            _ => remove(&mut metadata, "issuer"),
        }
        f.discovery.json(&metadata);
        refused(
            &OidcClient::new(&f.provider, secret(), http()).await,
            "invalid/incompatible metadata",
        );
        assert_eq!(f.discovery.count(), 1, "real metadata response reached");
        assert_eq!(f.token.count(), 0);
    }
}

#[tokio::test]
async fn g1_basic_auth_and_normative_discovery_default_use_only_the_configured_method() {
    let mut f = OidcFixture::new().await;
    f.provider.token_auth_method = oauth_connect_configuration::TokenAuthMethod::ClientSecretBasic;
    let mut metadata = f.metadata();
    remove(&mut metadata, "token_endpoint_auth_methods_supported");
    f.discovery.json(&metadata);
    let client = initialized(&f).await;
    default_profile(
        &f,
        &must(
            client
                .complete_authorization(&completion("oidc-code-basic-default"))
                .await,
            "normative Basic auth default",
        ),
    );
    let receipts = f.receipts().await;
    let post = receipts
        .iter()
        .find(|r| r.url.path() == TOKEN)
        .map_or_else(|| panic!("token POST required"), std::convert::identity);
    let Some(encoded) = post
        .headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Basic "))
    else {
        panic!("configured Basic auth header required");
    };
    let credentials = must(
        base64::engine::general_purpose::STANDARD.decode(encoded),
        "Basic base64 fixture",
    );
    assert!(
        credentials == format!("{CLIENT_ID}:{SECRET}").as_bytes(),
        "correct configured credentials only"
    );
    assert!(
        !form(post).contains_key("client_secret"),
        "not both Basic and Post credentials"
    );
    f.assert_counts(1, 1, 1, 0).await;
}

#[derive(Clone, Copy)]
enum Endpoint {
    Discovery,
    Jwks,
    Token,
    UserInfo,
}

impl Endpoint {
    const fn startup(self) -> bool {
        matches!(self, Self::Discovery | Self::Jwks)
    }
    const fn document(self, f: &OidcFixture) -> &fixture::Document {
        match self {
            Self::Discovery => &f.discovery,
            Self::Jwks => &f.keys,
            Self::Token => &f.token,
            Self::UserInfo => &f.userinfo,
        }
    }
    fn value(self, f: &OidcFixture, claims: &Value) -> Value {
        match self {
            Self::Discovery => f.metadata(),
            Self::Jwks => jwks(key_a(), KID),
            Self::Token => OidcFixture::token_body(&sign(key_a(), KID, claims)),
            Self::UserInfo => {
                json!({"sub": SUBJECT, "email": "one@example.test", "email_verified": true})
            }
        }
    }
}

async fn callback_case(
    endpoint: Endpoint,
    response: ResponseTemplate,
    f: &OidcFixture,
    accepted: bool,
) {
    endpoint.document(f).set(response);
    let result = OidcClient::new(&f.provider, secret(), http()).await;
    if endpoint.startup() && !accepted {
        refused(&result, "startup callback status/bytes/encoding/redirect");
        f.assert_counts(1, usize::from(matches!(endpoint, Endpoint::Jwks)), 0, 0)
            .await;
        return;
    }
    let client = must(result, "valid startup for callback test");
    let result = client
        .complete_authorization(&completion("oidc-code-callback"))
        .await;
    if accepted {
        default_profile(f, &must(result, "callback valid bounded response"));
    } else {
        authentication_failed(&result);
    }
    f.assert_counts(1, 1, 1, usize::from(matches!(endpoint, Endpoint::UserInfo)))
        .await;
}

async fn callback_fixture(endpoint: Endpoint) -> (OidcFixture, Value) {
    let f = OidcFixture::new().await;
    let mut claims = f.claims_at(Utc::now());
    if matches!(endpoint, Endpoint::UserInfo) {
        remove(&mut claims, "email");
    }
    f.set_claims(&claims);
    let value = endpoint.value(&f, &claims);
    (f, value)
}

#[tokio::test]
async fn g1_every_callback_accepts_small_gzip_and_exact_limit_but_refuses_decompressed_plus_one() {
    for endpoint in [
        Endpoint::Discovery,
        Endpoint::Jwks,
        Endpoint::Token,
        Endpoint::UserInfo,
    ] {
        for (compressed, limit) in [
            (false, LIMIT),
            (false, LIMIT + 1),
            (true, 8192),
            (true, LIMIT),
            (true, LIMIT + 1),
        ] {
            let (f, value) = callback_fixture(endpoint).await;
            let bytes = padded(&value, limit);
            let response = if compressed {
                body(gzip(&bytes), Some("gzip"))
            } else {
                body(bytes, Some("identity"))
            };
            callback_case(endpoint, response, &f, limit <= LIMIT).await;
        }
    }
}

#[tokio::test]
async fn g1_every_callback_refuses_status_html_residual_encoding_and_truncated_gzip() {
    for endpoint in [
        Endpoint::Discovery,
        Endpoint::Jwks,
        Endpoint::Token,
        Endpoint::UserInfo,
    ] {
        for invalid in [
            "403",
            "429",
            "503",
            "html",
            "br",
            "unknown",
            "empty",
            "list",
            "truncated",
        ] {
            let (f, value) = callback_fixture(endpoint).await;
            let bytes = must(serde_json::to_vec(&value), "fixture document");
            let response = match invalid {
                "403" => ResponseTemplate::new(403).set_body_json(&value),
                "429" => ResponseTemplate::new(429).set_body_json(&value),
                "503" => ResponseTemplate::new(503).set_body_json(&value),
                "html" => body(b"<html>error</html>".to_vec(), None),
                "br" => body(bytes, Some("br")),
                "unknown" => body(bytes, Some("unknown")),
                "empty" => body(bytes, Some("")),
                "list" => body(bytes, Some("identity, br")),
                _ => {
                    let mut bytes = gzip(&bytes);
                    bytes.truncate(bytes.len() - 8);
                    body(bytes, Some("gzip"))
                }
            };
            callback_case(endpoint, response, &f, false).await;
        }
    }
}

#[tokio::test]
async fn g1_every_callback_refuses_redirect_even_to_an_approved_origin_without_destination_io() {
    for endpoint in [
        Endpoint::Discovery,
        Endpoint::Jwks,
        Endpoint::Token,
        Endpoint::UserInfo,
    ] {
        for status in [302, 307] {
            let (f, _) = callback_fixture(endpoint).await;
            Mock::given(path("/capture"))
                .respond_with(json_response(&json!({})))
                .mount(&f.server)
                .await;
            callback_case(
                endpoint,
                ResponseTemplate::new(status).insert_header("location", f.url("/capture").as_str()),
                &f,
                false,
            )
            .await;
            assert!(
                f.receipts()
                    .await
                    .iter()
                    .all(|r| r.url.path() != "/capture"),
                "no credential-bearing destination request"
            );
        }
    }
}

#[tokio::test]
async fn g1_callback_preserves_standard_duplicate_field_rejection_instead_of_value_reserialization()
{
    for endpoint in [Endpoint::Discovery, Endpoint::Token] {
        let (f, value) = callback_fixture(endpoint).await;
        let (key, repeated) = if matches!(endpoint, Endpoint::Discovery) {
            ("issuer", json!(f.issuer()))
        } else {
            ("access_token", json!(ACCESS))
        };
        let duplicate = duplicate_field(&value, key, &repeated);
        callback_case(endpoint, body(duplicate, None), &f, false).await;
    }
}

#[tokio::test]
async fn g1_callback_token_deadline_is_internal_not_a_watchdog_or_replayed_code() {
    let f = OidcFixture::new().await;
    let client = initialized(&f).await;
    let control = must(
        client
            .complete_authorization(&completion("oidc-code-deadline-control"))
            .await,
        "deadline nominal control",
    );
    default_profile(&f, &control);
    f.token.set(
        json_response(&OidcFixture::token_body(&sign(
            key_a(),
            KID,
            &f.claims_at(Utc::now()),
        )))
        .set_delay(Duration::from_secs(11)),
    );
    let before = f.token.count();
    let started = Instant::now();
    let completed = must(
        tokio::time::timeout(
            Duration::from_secs(15),
            client.complete_authorization(&completion("oidc-code-deadline")),
        )
        .await,
        "watchdog must not expire instead of the internal timeout",
    );
    assert!(
        matches!(completed, Err(FederatedOAuthError::ExchangeCode)),
        "delayed valid token POST fails at exchange"
    );
    assert!(
        started.elapsed() >= Duration::from_secs(9),
        "immediate validation refusal is not timeout evidence"
    );
    f.assert_counts(1, 1, before + 1, 0).await;
}

#[tokio::test]
async fn g2_signed_claim_mutants_are_refused_without_userinfo_or_replaying_the_code() {
    let f = OidcFixture::new().await;
    let foreign = OidcFixture::new().await;
    let client = initialized(&f).await;
    let control_claims = f.claims_at(Utc::now());
    let mut mutants = Vec::new();
    for key in ["iss", "aud", "exp", "iat", "sub", "nonce"] {
        let mut claims = control_claims.clone();
        remove(&mut claims, key);
        mutants.push(claims);
    }
    for (key, value) in [
        ("iss", json!(foreign.issuer())),
        ("aud", json!("foreign-client")),
        ("aud", json!([CLIENT_ID, "extra-client"])),
        ("azp", json!("foreign-client")),
        ("exp", json!(Utc::now().timestamp() - 1)),
        ("iat", json!(Utc::now().timestamp() + 120)),
        ("nbf", json!(Utc::now().timestamp() + 120)),
        ("nbf", json!("not-a-date")),
        ("sub", Value::Null),
        ("sub", json!(42)),
        ("sub", json!("")),
        ("nonce", json!("different-nonce")),
        ("at_hash", json!(access_hash("different-access-token"))),
    ] {
        let mut claims = control_claims.clone();
        if key == "aud" && value.is_array() {
            claims["azp"] = json!(CLIENT_ID);
        }
        claims[key] = value;
        mutants.push(claims);
    }
    for (index, claims) in mutants.iter().enumerate() {
        // Every negative is adjacent to a genuinely valid RS256 control.
        f.set_claims(&control_claims);
        default_profile(
            &f,
            &must(
                client
                    .complete_authorization(&completion(&format!("oidc-code-control-{index}")))
                    .await,
                "adjacent signed control",
            ),
        );
        f.set_claims(claims);
        authentication_failed(
            &client
                .complete_authorization(&completion(&format!("oidc-code-mutant-{index}")))
                .await,
        );
        f.assert_counts(1, 1, (index + 1) * 2, 0).await;
    }
    foreign.assert_counts(0, 0, 0, 0).await;
}

#[tokio::test]
async fn g2_at_hash_is_bound_to_the_actual_access_token_and_azp_may_equal_the_client() {
    let f = OidcFixture::new().await;
    let client = initialized(&f).await;
    let mut claims = f.claims_at(Utc::now());
    claims["at_hash"] = json!(access_hash(ACCESS));
    claims["azp"] = json!(CLIENT_ID);
    f.set_claims(&claims);
    default_profile(
        &f,
        &must(
            client
                .complete_authorization(&completion("oidc-code-hash-valid"))
                .await,
            "valid at_hash/azp",
        ),
    );
    let mut response = OidcFixture::token_body(&sign(key_a(), KID, &claims));
    response["access_token"] = json!("another-access");
    f.token.json(&response);
    authentication_failed(
        &client
            .complete_authorization(&completion("oidc-code-hash-substitution"))
            .await,
    );
    f.assert_counts(1, 1, 2, 0).await;
}

#[tokio::test]
async fn g2_other_signer_and_unapproved_algorithms_do_not_validate_identity() {
    let f = OidcFixture::new().await;
    // Advertised alternatives are not local permission. An RSA JWK without an
    // alg pin is standard and prevents its own alg field from masking a missing
    // RS256-only verifier policy. HS256 is signed with the configured secret.
    let mut metadata = f.metadata();
    metadata["id_token_signing_alg_values_supported"] = json!(["RS256", "RS384", "HS256"]);
    f.discovery.json(&metadata);
    let mut rsa_key = jwk(key_a(), KID);
    remove(&mut rsa_key, "alg");
    f.keys.json(&json!({"keys": [rsa_key]}));
    let client = initialized(&f).await;
    let claims = f.claims_at(Utc::now());
    let mut hs = Header::new(Algorithm::HS256);
    hs.kid = Some(KID.to_owned());
    let mut rs384 = Header::new(Algorithm::RS384);
    rs384.kid = Some(KID.to_owned());
    let none = format!(
        "{}.{}.",
        URL_SAFE_NO_PAD.encode(b"{\"alg\":\"none\"}"),
        URL_SAFE_NO_PAD.encode(must(serde_json::to_vec(&claims), "unsigned mutant claims"))
    );
    default_profile(
        &f,
        &must(
            client
                .complete_authorization(&completion("oidc-code-signature-control"))
                .await,
            "adjacent signed crypto control",
        ),
    );
    let tokens = [
        sign(key_b(), KID, &claims),
        must(
            jsonwebtoken::encode(
                &hs,
                &claims,
                &jsonwebtoken::EncodingKey::from_secret(SECRET.as_bytes()),
            ),
            "HS mutant",
        ),
        must(
            jsonwebtoken::encode(&rs384, &claims, &key_a().encoding),
            "RSA384 mutant",
        ),
        none,
    ];
    for (index, token) in tokens.iter().enumerate() {
        f.token.json(&OidcFixture::token_body(token));
        authentication_failed(
            &client
                .complete_authorization(&completion(&format!("oidc-code-signature-{index}")))
                .await,
        );
        f.assert_counts(1, 1, index + 2, 0).await;
    }
}

#[tokio::test]
async fn g2_ambiguous_initial_key_id_never_selects_the_first_key() {
    let f = OidcFixture::new().await;
    f.keys
        .json(&json!({"keys": [jwk(key_a(), KID), jwk(key_b(), KID)]}));
    // Boot may reject ambiguous material outright. If retained, authentication
    // must fail rather than choose its first key. No requirement to accept boot.
    match OidcClient::new(&f.provider, secret(), http()).await {
        Ok(client) => {
            authentication_failed(
                &client
                    .complete_authorization(&completion("oidc-code-ambiguous"))
                    .await,
            );
            f.assert_counts(1, 1, 1, 0).await;
        }
        Err(error) => {
            redacted(&error.to_string());
            f.assert_counts(1, 1, 0, 0).await;
        }
    }
}

#[tokio::test]
async fn g2_missing_malformed_or_encrypted_shape_id_tokens_are_not_optional_identity() {
    let f = OidcFixture::new().await;
    let client = initialized(&f).await;
    for (index, token) in [None, Some("not-a-token"), Some("a.b.c.d.e")]
        .iter()
        .enumerate()
    {
        let mut body = OidcFixture::token_body("unused");
        if let Some(token) = token {
            body["id_token"] = json!(token);
        } else {
            remove(&mut body, "id_token");
        }
        f.token.json(&body);
        authentication_failed(
            &client
                .complete_authorization(&completion(&format!("oidc-code-invalid-id-{index}")))
                .await,
        );
        f.assert_counts(1, 1, index + 1, 0).await;
    }
    // The five-part example is a syntax rejection, not a valid encrypted-JWE crypto preflight.
}

#[tokio::test]
async fn g2_token_header_urls_are_not_new_discovery_or_jwks_authorities() {
    let f = OidcFixture::new().await;
    let foreign = OidcFixture::new().await;
    let client = initialized(&f).await;
    default_profile(
        &f,
        &must(
            client
                .complete_authorization(&completion("oidc-code-header-control"))
                .await,
            "adjacent pinned-authority control",
        ),
    );
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(KID.to_owned());
    header.jku = Some(foreign.url("/jwks-from-header"));
    header.x5u = Some(foreign.url("/certificate-from-header"));
    let token = must(
        jsonwebtoken::encode(&header, &f.claims_at(Utc::now()), &key_a().encoding),
        "signed header-url fixture",
    );
    f.token.json(&OidcFixture::token_body(&token));
    let result = client
        .complete_authorization(&completion("oidc-code-header-urls"))
        .await;
    // Ignoring an irrelevant hint or refusing it are both fail-closed policies;
    // selecting an authority/URL from it is not. No extra acceptance rule invented.
    result.as_ref().map_or_else(
        |_| authentication_failed(&result),
        |auth| default_profile(&f, auth),
    );
    foreign.assert_counts(0, 0, 0, 0).await;
    f.assert_counts(1, 1, 2, 0).await;
}

#[tokio::test]
async fn g3_subject_strings_remain_exact_without_numeric_cast_trim_or_issuer_prefix() {
    let f = OidcFixture::new().await;
    let client = initialized(&f).await;
    for (index, sub) in ["000042", " 000042 ", "λ", " "].iter().enumerate() {
        let mut claims = f.claims_at(Utc::now());
        claims["sub"] = json!(sub);
        f.set_claims(&claims);
        let auth = must(
            client
                .complete_authorization(&completion(&format!("oidc-code-sub-{index}")))
                .await,
            "exact nonempty subject",
        );
        profile(
            &auth,
            sub,
            "name",
            Some("one@example.test"),
            true,
            Some(&f.url("/avatar")),
        );
        f.assert_counts(1, 1, index + 1, 0).await;
    }
}

#[tokio::test]
async fn g3_present_email_does_not_fetch_userinfo_for_other_missing_claims() {
    let f = OidcFixture::new().await;
    let client = initialized(&f).await;
    let mut claims = f.claims_at(Utc::now());
    for key in ["preferred_username", "picture", "email_verified"] {
        remove(&mut claims, key);
    }
    f.set_claims(&claims);
    let auth = must(
        client
            .complete_authorization(&completion("oidc-code-no-extra-fetch"))
            .await,
        "incomplete descriptive profile",
    );
    profile(&auth, SUBJECT, "", Some("one@example.test"), false, None);
    f.assert_counts(1, 1, 1, 0).await;
}

#[tokio::test]
async fn g3_absent_email_without_endpoint_is_valid_incomplete_profile_not_synthetic_email() {
    let f = OidcFixture::new().await;
    let mut metadata = f.metadata();
    remove(&mut metadata, "userinfo_endpoint");
    f.discovery.json(&metadata);
    let mut claims = f.claims_at(Utc::now());
    remove(&mut claims, "email");
    f.set_claims(&claims);
    let client = initialized(&f).await;
    let auth = must(
        client
            .complete_authorization(&completion("oidc-code-no-userinfo-endpoint"))
            .await,
        "optional missing email/endpoint",
    );
    profile(&auth, SUBJECT, "name", None, false, Some(&f.url("/avatar")));
    f.assert_counts(1, 1, 1, 0).await;
}

#[tokio::test]
async fn g3_userinfo_fills_email_with_same_source_verification_without_false_defaults() {
    let f = OidcFixture::new().await;
    let client = initialized(&f).await;
    for (index, id_verified) in [None, Some(true)].iter().enumerate() {
        let mut claims = f.claims_at(Utc::now());
        remove(&mut claims, "email");
        if let Some(value) = id_verified {
            claims["email_verified"] = json!(value);
        } else {
            remove(&mut claims, "email_verified");
        }
        f.set_claims(&claims);
        let mut info = json!({"sub": SUBJECT, "email": "one@example.test"});
        if id_verified.is_none() {
            info["email_verified"] = json!(true);
        }
        f.userinfo.json(&info);
        let auth = must(
            client
                .complete_authorization(&completion(&format!("oidc-code-provenance-{index}")))
                .await,
            "source-bound UserInfo verification",
        );
        profile(
            &auth,
            SUBJECT,
            "name",
            Some("one@example.test"),
            id_verified.is_none(),
            Some(&f.url("/avatar")),
        );
        f.assert_counts(1, 1, index + 1, index + 1).await;
    }
    let receipts = f.receipts().await;
    for request in receipts.iter().filter(|r| r.url.path() == USERINFO) {
        assert!(
            request
                .headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                == Some(format!("Bearer {ACCESS}").as_str()),
            "UserInfo uses access token, not ID token"
        );
    }
}

#[tokio::test]
async fn g3_userinfo_subject_and_nonnull_claim_conflicts_are_refused() {
    let f = OidcFixture::new().await;
    let client = initialized(&f).await;
    let mut claims = f.claims_at(Utc::now());
    remove(&mut claims, "email");
    f.set_claims(&claims);
    let mut invalid = vec![json!({"email": "one@example.test"})];
    for (key, value) in [
        ("sub", json!("other-subject")),
        ("sub", Value::Null),
        ("sub", json!(42)),
        ("preferred_username", json!("different-name")),
        ("picture", json!(f.url("/different-picture"))),
        ("email_verified", json!(false)),
        ("email", json!(12)),
        ("email_verified", json!("true")),
    ] {
        let mut info = json!({"sub": SUBJECT, "email": "one@example.test"});
        info[key] = value;
        invalid.push(info);
    }
    for (index, info) in invalid.iter().enumerate() {
        f.userinfo.json(info);
        authentication_failed(
            &client
                .complete_authorization(&completion(&format!(
                    "oidc-code-userinfo-conflict-{index}"
                )))
                .await,
        );
        f.assert_counts(1, 1, index + 1, index + 1).await;
    }
}

#[tokio::test]
async fn g3_optional_null_is_absent_but_bad_id_claim_types_do_not_trigger_userinfo_repair() {
    let f = OidcFixture::new().await;
    let client = initialized(&f).await;
    let mut claims = f.claims_at(Utc::now());
    for key in ["email", "email_verified", "picture", "preferred_username"] {
        claims[key] = Value::Null;
    }
    f.set_claims(&claims);
    let auth = must(
        client
            .complete_authorization(&completion("oidc-code-null-optionals"))
            .await,
        "standard null optionals",
    );
    profile(&auth, SUBJECT, "", Some("one@example.test"), true, None);
    f.assert_counts(1, 1, 1, 1).await;
    for (index, (key, value)) in [
        ("email", json!(42)),
        ("email_verified", json!("true")),
        ("preferred_username", json!({})),
        ("picture", json!(false)),
    ]
    .iter()
    .enumerate()
    {
        let mut claims = f.claims_at(Utc::now());
        claims[*key] = value.clone();
        f.set_claims(&claims);
        authentication_failed(
            &client
                .complete_authorization(&completion(&format!("oidc-code-bad-type-{index}")))
                .await,
        );
        f.assert_counts(1, 1, index + 2, 1).await;
    }
}

#[tokio::test]
async fn g3_signed_userinfo_is_passed_as_original_bytes_to_the_standard_parser() {
    let f = OidcFixture::new().await;
    let client = initialized(&f).await;
    let mut claims = f.claims_at(Utc::now());
    remove(&mut claims, "email");
    f.set_claims(&claims);
    let info = json!({"iss": f.issuer(), "aud": CLIENT_ID, "sub": SUBJECT, "email": "one@example.test", "email_verified": true});
    let signed = sign(key_a(), KID, &info);
    f.userinfo.set(fixture::signed_userinfo_response(signed));
    let auth = must(
        client
            .complete_authorization(&completion("oidc-code-signed-userinfo"))
            .await,
        "signed UserInfo callback bytes",
    );
    default_profile(&f, &auth);
    f.assert_counts(1, 1, 1, 1).await;
}

#[tokio::test]
async fn g4_instances_with_same_kid_and_subject_do_not_share_trust_or_require_prior_authorize() {
    let a = OidcFixture::new().await;
    let b = OidcFixture::new().await;
    b.keys.json(&jwks(key_b(), KID));
    b.set_signed(key_b(), KID, &b.claims_at(Utc::now()));
    let ca = initialized(&a).await;
    let cb = initialized(&b).await;
    default_profile(
        &a,
        &must(
            ca.complete_authorization(&completion("oidc-code-instance-a"))
                .await,
            "instance A key",
        ),
    );
    default_profile(
        &b,
        &must(
            cb.complete_authorization(&completion("oidc-code-instance-b"))
                .await,
            "instance B key",
        ),
    );
    b.set_signed(key_a(), KID, &b.claims_at(Utc::now()));
    authentication_failed(
        &cb.complete_authorization(&completion("oidc-code-cross-key"))
            .await,
    );
    a.assert_counts(1, 1, 1, 0).await;
    b.assert_counts(1, 1, 2, 0).await;
    let replica = initialized(&a).await;
    default_profile(
        &a,
        &must(
            replica
                .complete_authorization(&completion("oidc-code-fresh-replica"))
                .await,
            "another client without authorize cache",
        ),
    );
    a.assert_counts(2, 2, 2, 0).await;
}
