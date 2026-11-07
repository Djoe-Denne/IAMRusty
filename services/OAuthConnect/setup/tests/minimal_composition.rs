//! C-1/G-3/G-6 and review T-2: real consumption after integrated A+B GREEN.
//!
//! Obligations -> risks -> scenarios -> oracles:
//! - Complete, isolated catalogue -> swapped credentials/partial publication -> two
//!   OAuth instances direct+vault -> exact tokens/profiles, POST receipts and HMAC.
//! - Boot snapshot -> hidden refresh/cache/retry -> mutate vault/vendor after boot ->
//!   changed profile but original credentials, zero new vault GETs, failed POST once.
//! - Trusted builder consumed -> fixture-only policy proof -> C-created adapters on
//!   redirects/gzip/TLS/deadline -> typed errors, complete outputs and phase receipts.
//! - Real source assembly -> invented loader/env syntax -> marked child with real SDK
//!   TOML/`RUN_ENV` -> composed authorize+complete and proof that exactly one oracle ran.
//!
//! KEEP each test: the inverse of its scenario is its targeted mutation. No new
//! product client is constructed in any helper. OIDC/invalid-direct negatives can
//! already pass T0 and are only characterizations until the positive C cases pass.

#[path = "support/c_flow.rs"]
mod c_flow;
#[path = "support/c_gzip.rs"]
mod c_gzip;
#[path = "support/c_tls.rs"]
mod c_tls;

use c_flow::{
    authentication, callback, config, exchange_error, hmac_proof, must, provider, redacted,
    rejected, request, tokens, vault_provider, Vault, Vendor, ACCESS, BAO_CLIENT, BAO_HMAC,
    BAO_LOOKUP, CLIENT, EMAIL_PATH, HMAC, HMAC_RAW, TOKEN_PATH, USER_PATH,
};
use idp_connect_contract::{AuthorizeAuthenticationRequest, FederatedOAuthClient};
use oauth_connect_configuration::{
    load_config, AppConfig, OidcProfile, ProviderProfile, VendorTransportSecurity,
};
use oauth_connect_setup::{build_oauth_clients, ConfiguredOAuthClient};
use serde_json::json;
use std::{collections::BTreeMap, sync::atomic::Ordering, time::Duration};
use wiremock::ResponseTemplate;

async fn composed(value: &AppConfig) -> Vec<ConfiguredOAuthClient> {
    let clients = must(build_oauth_clients(value).await, "real OAuth composition");
    redacted(&format!("{clients:?}"));
    clients
}

fn instance<'a>(clients: &'a [ConfiguredOAuthClient], id: &str) -> &'a ConfiguredOAuthClient {
    assert_eq!(
        clients.iter().filter(|c| c.id == id).count(),
        1,
        "C: exactly one client per logical instance"
    );
    clients
        .iter()
        .find(|c| c.id == id)
        .expect("C expected public instance id")
}

#[tokio::test]
async fn c1_two_oauth_instances_direct_and_vault_route_only_their_own_credentials_and_hmac() {
    let first = Vendor::start().await;
    let second = Vendor::start().await;
    first
        .success(71, "caseuser", "primary@example.test", ACCESS)
        .await;
    second
        .success(
            902,
            "enterpriseuser",
            "enterprise@example.test",
            "enterprise-access-sentinel",
        )
        .await;
    let vault = Vault::start().await;
    vault.success(CLIENT, HMAC_RAW).await;
    let first_secret = " direct-client-one+&=:/sentinel ";
    let first_raw_hmac = " \tdirect-HMAC-one-canonical-sentinel\r\n";
    let first_hmac = "direct-HMAC-one-canonical-sentinel";
    let mut value = config(vec![
        provider("forge", &first.origin(), first_secret, first_raw_hmac),
        vault_provider(provider("enterprise", &second.origin(), CLIENT, HMAC_RAW)),
    ]);
    value.openbao = Some(vault.config());
    let clients = composed(&value).await;
    assert_eq!(
        clients.len(),
        2,
        "C: complete catalogue, not filtered by profile or secrets mode"
    );
    let boot_requests = vault.total().await;
    assert_eq!(vault.count(BAO_CLIENT).await, 1);
    assert_eq!(vault.count(BAO_HMAC).await, 1);
    first.counts(0, 0, 0).await;
    second.counts(0, 0, 0).await;
    let a = instance(&clients, "forge");
    let b = instance(&clients, "enterprise");
    hmac_proof(&a.hmac_key, "forge", first_hmac, first_raw_hmac, HMAC);
    hmac_proof(&b.hmac_key, "enterprise", HMAC, HMAC_RAW, first_hmac);
    authentication(
        &must(
            a.client.complete_authorization(&request("forge")).await,
            "first complete",
        ),
        "71",
        "caseuser",
        "primary@example.test",
        ACCESS,
    );
    authentication(
        &must(
            b.client
                .complete_authorization(&request("enterprise"))
                .await,
            "second complete",
        ),
        "902",
        "enterpriseuser",
        "enterprise@example.test",
        "enterprise-access-sentinel",
    );
    first.counts(1, 1, 1).await;
    second.counts(1, 1, 1).await;
    first.credentials("forge", first_secret, ACCESS).await;
    second
        .credentials("enterprise", CLIENT, "enterprise-access-sentinel")
        .await;
    assert_eq!(
        vault.total().await,
        boot_requests,
        "C: completion must not reacquire secrets"
    );
}

#[tokio::test]
async fn c1_boot_snapshot_keeps_keys_without_vault_refresh_cache_or_retry_after_vendor_failure() {
    let vendor = Vendor::start().await;
    let vault = Vault::start().await;
    vault.success(CLIENT, HMAC_RAW).await;
    let mut value = config(vec![vault_provider(provider(
        "forge",
        &vendor.origin(),
        CLIENT,
        HMAC_RAW,
    ))]);
    value.openbao = Some(vault.config());
    let clients = composed(&value).await;
    assert_eq!(clients.len(), 1);
    assert_eq!(vault.count(BAO_CLIENT).await, 1);
    assert_eq!(vault.count(BAO_HMAC).await, 1);
    vault.server.reset().await;
    vault
        .success("replacement-client-sentinel", "replacement-HMAC-sentinel")
        .await;
    let built = instance(&clients, "forge");
    for (number, id, login, email, access) in [
        (71, "71", "firstuser", "first@example.test", ACCESS),
        (
            72,
            "72",
            "seconduser",
            "second@example.test",
            "second-access-sentinel",
        ),
    ] {
        vendor.server.reset().await;
        vendor.success(number, login, email, access).await;
        authentication(
            &must(
                built.client.complete_authorization(&request("forge")).await,
                "snapshot complete",
            ),
            id,
            login,
            email,
            access,
        );
        vendor.counts(1, 1, 1).await;
        vendor.credentials("forge", CLIENT, access).await;
        hmac_proof(
            &built.hmac_key,
            "forge",
            HMAC,
            HMAC_RAW,
            "replacement-HMAC-sentinel",
        );
        assert_eq!(
            vault.total().await,
            0,
            "C: no vault refresh during business calls"
        );
    }
    vendor.server.reset().await;
    vendor
        .response(
            "POST",
            TOKEN_PATH,
            ResponseTemplate::new(403).set_body_string("failed-token-body-sentinel"),
        )
        .await;
    exchange_error(&built.client.complete_authorization(&request("forge")).await);
    vendor.counts(1, 0, 0).await;
    vendor.credentials("forge", CLIENT, ACCESS).await;
    assert_eq!(
        vault.total().await,
        0,
        "C: failed vendor call must not re-resolve credentials"
    );
    hmac_proof(
        &built.hmac_key,
        "forge",
        HMAC,
        HMAC_RAW,
        "replacement-HMAC-sentinel",
    );
}

#[tokio::test]
async fn c1_mixed_oauth_oidc_is_refused_before_any_acquisition_or_discovery() {
    let vendor = Vendor::start().await;
    let vault = Vault::start().await;
    vault.success(CLIENT, HMAC_RAW).await;
    let oauth = vault_provider(provider("forge", &vendor.origin(), CLIENT, HMAC_RAW));
    let mut oidc = vault_provider(provider("identity", &vendor.origin(), CLIENT, HMAC_RAW));
    oidc.scopes = vec!["openid".into(), "profile".into(), "email".into()];
    oidc.profile = ProviderProfile::Oidc(OidcProfile {
        issuer: format!("{}/authority", vendor.origin()),
        allowed_endpoint_origins: vec![vendor.origin()],
    });
    let mut value = config(vec![oauth, oidc]);
    value.openbao = Some(vault.config());
    rejected(build_oauth_clients(&value).await);
    assert_eq!(
        vault.total().await,
        0,
        "C: unsupported profile refused before resolving any secret"
    );
    vendor.counts(0, 0, 0).await;
}

#[tokio::test]
async fn c1_late_direct_hmac_short_after_trim_returns_no_catalogue_before_io() {
    let vendor = Vendor::start().await;
    let vault = Vault::start().await;
    vault.success(CLIENT, HMAC_RAW).await;
    let mut value = config(vec![
        vault_provider(provider("forge", &vendor.origin(), CLIENT, HMAC_RAW)),
        provider("other", &vendor.origin(), CLIENT, " 0123456789abcde "),
    ]);
    value.openbao = Some(vault.config());
    rejected(build_oauth_clients(&value).await);
    assert_eq!(vault.total().await, 0);
    vendor.counts(0, 0, 0).await;
}

#[tokio::test]
async fn c1_direct_valid_plus_late_vault_403_has_no_partial_collection_and_reaches_sdk() {
    let vendor = Vendor::start().await;
    let vault = Vault::start().await;
    vault
        .response(
            BAO_LOOKUP,
            ResponseTemplate::new(200)
                .set_body_json(json!({"data": {"ttl": 600, "policies": ["read"]}})),
        )
        .await;
    vault
        .response(
            BAO_CLIENT,
            ResponseTemplate::new(403).set_body_string("vault-failure-sentinel"),
        )
        .await;
    vault
        .response(
            BAO_HMAC,
            ResponseTemplate::new(200)
                .set_body_json(json!({"data": {"data": {"hmac_secret": HMAC_RAW}}})),
        )
        .await;
    let mut value = config(vec![
        provider("forge", &vendor.origin(), CLIENT, HMAC_RAW),
        vault_provider(provider("enterprise", &vendor.origin(), CLIENT, HMAC_RAW)),
    ]);
    value.openbao = Some(vault.config());
    rejected(build_oauth_clients(&value).await);
    assert_eq!(
        vault.count(BAO_CLIENT).await,
        1,
        "C: do not count the T0 refusal as a remote-failure proof"
    );
    vendor.counts(0, 0, 0).await;
}

#[tokio::test]
async fn c2_composed_client_does_not_follow_307_even_to_a_configured_profile_destination() {
    let vendor = Vendor::start().await;
    let destination = Vendor::start().await;
    let mut p = provider("forge", &vendor.origin(), CLIENT, HMAC_RAW);
    let ProviderProfile::Oauth(profile) = &mut p.profile else {
        panic!("C OAuth seed")
    };
    profile.apis.get_mut("user").unwrap().url = format!("{}{USER_PATH}", destination.origin());
    vendor
        .response(
            "POST",
            TOKEN_PATH,
            ResponseTemplate::new(307)
                .insert_header("Location", format!("{}{USER_PATH}", destination.origin())),
        )
        .await;
    destination
        .success(71, "caseuser", "primary@example.test", ACCESS)
        .await;
    let clients = composed(&config(vec![p])).await;
    exchange_error(
        &instance(&clients, "forge")
            .client
            .complete_authorization(&request("forge"))
            .await,
    );
    vendor.counts(1, 0, 0).await;
    destination.counts(0, 0, 0).await;
}

#[tokio::test]
async fn c2_composed_client_really_decodes_gzip_token_post_and_both_independent_gets() {
    let vendor = Vendor::start().await;
    for (verb, route, gzip) in [
        ("POST", TOKEN_PATH, c_gzip::TOKEN),
        ("GET", USER_PATH, c_gzip::USER),
        ("GET", EMAIL_PATH, c_gzip::EMAIL),
    ] {
        vendor
            .response(
                verb,
                route,
                ResponseTemplate::new(200)
                    .insert_header("Content-Encoding", "gzip")
                    .insert_header("Content-Type", "application/json")
                    .set_body_bytes(gzip),
            )
            .await;
    }
    let clients = composed(&config(vec![provider(
        "forge",
        &vendor.origin(),
        CLIENT,
        HMAC_RAW,
    )]))
    .await;
    authentication(
        &must(
            instance(&clients, "forge")
                .client
                .complete_authorization(&request("forge"))
                .await,
            "composed gzip complete",
        ),
        "71",
        "caseuser",
        "primary@example.test",
        ACCESS,
    );
    vendor.counts(1, 1, 1).await;
    vendor.credentials("forge", CLIENT, ACCESS).await;
}

#[tokio::test]
async fn c2_composed_client_verified_ca_and_san_complete_token_post_and_json_gets() {
    c_tls::run(true, |view| async move {
        let mut p = provider("forge", &view.origin, CLIENT, HMAC_RAW);
        p.transport_security = VendorTransportSecurity::Verified;
        p.ca_cert_file = Some(view.root);
        let clients = composed(&config(vec![p])).await;
        authentication(
            &must(
                instance(&clients, "forge")
                    .client
                    .complete_authorization(&request("forge"))
                    .await,
                "composed verified TLS complete",
            ),
            "71",
            "caseuser",
            "primary@example.test",
            ACCESS,
        );
        assert_eq!(view.receipts.token.load(Ordering::SeqCst), 1);
        assert_eq!(view.receipts.user.load(Ordering::SeqCst), 1);
        assert_eq!(view.receipts.emails.load(Ordering::SeqCst), 1);
        let forms = view
            .receipts
            .forms
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let fields: BTreeMap<String, String> = url::form_urlencoded::parse(&forms[0])
            .into_owned()
            .collect();
        drop(forms);
        assert!(
            fields
                .get("client_secret")
                .is_some_and(|value| value == CLIENT),
            "C TLS: exact resolved credential"
        );
        assert!(fields
            .get("client_id")
            .is_some_and(|value| value == "public-forge"));
    })
    .await;
}

#[tokio::test]
async fn c2_composed_client_untrusted_ca_or_wrong_san_fails_before_any_http_handler() {
    for matching_san in [true, false] {
        c_tls::run(matching_san, move |view| async move {
            let mut p = provider("forge", &view.origin, CLIENT, HMAC_RAW);
            p.transport_security = VendorTransportSecurity::Verified;
            p.ca_cert_file = Some(if matching_san {
                view.unrelated_root
            } else {
                view.root
            });
            let clients = composed(&config(vec![p])).await;
            exchange_error(
                &instance(&clients, "forge")
                    .client
                    .complete_authorization(&request("forge"))
                    .await,
            );
            assert_eq!(view.receipts.token.load(Ordering::SeqCst), 0);
            assert_eq!(view.receipts.user.load(Ordering::SeqCst), 0);
            assert_eq!(view.receipts.emails.load(Ordering::SeqCst), 0);
        })
        .await;
    }
}

// Fixture source generation only; actual source assembly/deserialization belongs to the SDK.
fn loader_source(origin: &str) -> String {
    let p = provider("forge", origin, CLIENT, HMAC_RAW);
    let mut declaration = must(serde_json::to_value(p), "fixture public declaration");
    // Product Serialize intentionally omits secrets. Add synthetic test-only source values
    // explicitly; no wrapper trait or product serializer is weakened for this fixture.
    let fields = declaration.as_object_mut().expect("C declaration object");
    fields.remove("ca_cert_file");
    fields.insert("client_secret".into(), json!(CLIENT));
    fields.insert("hmac_secret".into(), json!(HMAC_RAW));
    must(
        toml::to_string(&json!({"providers": [declaration]})),
        "fixture TOML source",
    )
}

// Process isolation only. No parent listener/port or configuration environment is inherited.
fn file_source_child(test: &str, marker: &str) {
    let directory = tempfile::tempdir().expect("C isolated loader directory");
    std::fs::create_dir(directory.path().join("config")).unwrap();
    std::fs::write(
        directory.path().join("config/test.toml"),
        loader_source("https://loaded.example.invalid"),
    )
    .unwrap();
    std::fs::write(
        directory.path().join("config/development.toml"),
        "unknown='wrong-source-sentinel'\n",
    )
    .unwrap();
    std::fs::write(
        directory.path().join(".env"),
        "OAUTH_CONNECT_LOGGING__LEVEL=warn\n",
    )
    .unwrap();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", test, "--nocapture"])
        .current_dir(directory.path())
        .env_clear();
    for key in ["SystemRoot", "WINDIR", "PATH", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command.env("RUN_ENV", "test").env(marker, test);
    let output = command
        .output()
        .expect("C isolated test child, never Cargo");
    let stdout = String::from_utf8_lossy(&output.stdout);
    redacted(&stdout);
    redacted(&String::from_utf8_lossy(&output.stderr));
    assert!(
        stdout.contains("C_LOADER_INPUT_VALIDATED"),
        "C child loader must validate its input before any composition RED is counted"
    );
    assert!(
        output.status.success(),
        "C child loader/composition oracle failed"
    );
    assert!(
        stdout.contains("running 1 test"),
        "C child must discover exactly one test"
    );
    assert!(
        stdout.contains("C_LOADER_ORACLE_EXECUTED"),
        "C child must actually execute the oracle"
    );
}

#[tokio::test]
async fn c1_real_sdk_file_loader_in_marked_child_feeds_the_actual_composition() {
    const TEST: &str = "c1_real_sdk_file_loader_in_marked_child_feeds_the_actual_composition";
    const MARKER: &str = "OC_C_LOADER_CHILD";
    if std::env::var(MARKER).is_ok_and(|value| value == TEST) {
        let vendor = Vendor::start().await;
        vendor
            .success(71, "caseuser", "primary@example.test", ACCESS)
            .await;
        // All networking is child-local. The real loader sees the file, not a patched AppConfig.
        std::fs::write("config/test.toml", loader_source(&vendor.origin())).unwrap();
        let loaded = must(load_config(), "real SDK file loader");
        assert_eq!(loaded.providers.len(), 1);
        assert_eq!(loaded.providers[0].id, "forge");
        assert_eq!(loaded.logging.level, "warn");
        println!("C_LOADER_INPUT_VALIDATED");
        let clients = composed(&loaded).await;
        assert_eq!(clients.len(), 1);
        let built = instance(&clients, "forge");
        hmac_proof(
            &built.hmac_key,
            "forge",
            HMAC,
            HMAC_RAW,
            "different-HMAC-sentinel",
        );
        let start = AuthorizeAuthenticationRequest {
            redirect_uri: callback("forge"),
            state: "state-c-sentinel".into(),
            code_challenge: Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".into()),
            code_challenge_method: Some("S256".into()),
            nonce: request("forge").expected_nonce,
        };
        let auth = must(
            built.client.authorize_authentication(&start).await,
            "loaded composed authorize",
        );
        let parsed = must(
            url::Url::parse(&auth.authorization_url),
            "authorization URL",
        );
        let expected_origin = must(url::Url::parse(&vendor.origin()), "fixture public origin");
        assert_eq!(parsed.origin(), expected_origin.origin());
        assert_eq!(parsed.path(), "/authorize");
        let params: BTreeMap<String, String> = parsed.query_pairs().into_owned().collect();
        assert!(params
            .get("client_id")
            .is_some_and(|value| value == "public-forge"));
        assert!(params
            .get("redirect_uri")
            .is_some_and(|value| value == &callback("forge")));
        assert!(!params.contains_key("nonce") && !params.contains_key("code_verifier"));
        assert_eq!(auth.scope, "user:email");
        vendor.counts(0, 0, 0).await;
        authentication(
            &must(
                built.client.complete_authorization(&request("forge")).await,
                "SDK-loaded composed complete",
            ),
            "71",
            "caseuser",
            "primary@example.test",
            ACCESS,
        );
        vendor.counts(1, 1, 1).await;
        vendor.credentials("forge", CLIENT, ACCESS).await;
        println!("C_LOADER_ORACLE_EXECUTED");
        return;
    }
    file_source_child(TEST, MARKER);
}

#[tokio::test]
async fn c2_t2_actual_composed_client_has_ten_second_timeout_not_the_fixture_client() {
    let vendor = Vendor::start().await;
    vendor
        .success(71, "caseuser", "primary@example.test", ACCESS)
        .await;
    let clients = composed(&config(vec![provider(
        "forge",
        &vendor.origin(),
        CLIENT,
        HMAC_RAW,
    )]))
    .await;
    let built = instance(&clients, "forge");
    authentication(
        &must(
            built.client.complete_authorization(&request("forge")).await,
            "deadline nominal control",
        ),
        "71",
        "caseuser",
        "primary@example.test",
        ACCESS,
    );
    vendor.counts(1, 1, 1).await;
    vendor.server.reset().await;
    vendor
        .response(
            "POST",
            TOKEN_PATH,
            ResponseTemplate::new(200)
                .set_body_json(tokens(ACCESS))
                .set_delay(Duration::from_secs(11)),
        )
        .await;
    vendor
        .response(
            "GET",
            USER_PATH,
            ResponseTemplate::new(200).set_body_json(c_flow::user(71, "caseuser")),
        )
        .await;
    vendor
        .response(
            "GET",
            EMAIL_PATH,
            ResponseTemplate::new(200).set_body_json(c_flow::emails("primary@example.test")),
        )
        .await;
    let start = tokio::time::Instant::now();
    let result = tokio::time::timeout(
        Duration::from_secs(15),
        built.client.complete_authorization(&request("forge")),
    )
    .await;
    let result =
        result.unwrap_or_else(|_| panic!("T-2: watchdog fired, product deadline is NOT proven"));
    exchange_error(&result);
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_secs(9) && elapsed < Duration::from_secs(11),
        "T-2: error must occur at product 10s deadline, not immediate refusal or valid delayed response");
    vendor.counts(1, 0, 0).await;
    vendor.credentials("forge", CLIENT, ACCESS).await;
}
