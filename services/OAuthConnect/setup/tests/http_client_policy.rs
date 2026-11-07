//! B-builder: send with the actual setup Client, never an invented inspectable client.
//! TLS is a local transport proof, not OAuth composition (C) or OIDC runtime.

#[path = "support/b_gzip.rs"]
mod b_gzip;
#[path = "support/b_provider.rs"]
mod b_provider;
#[path = "support/b_tls.rs"]
mod b_tls;

use b_gzip::GZIP_KV;
use b_provider::provider;
use oauth_connect_configuration::{ProviderConfig, ProviderProfile, VendorTransportSecurity};
use oauth_connect_setup::build_http_client;
use std::sync::atomic::Ordering;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

fn target(uri: &str, mode: VendorTransportSecurity) -> ProviderConfig {
    let mut p = provider("forge");
    p.transport_security = mode;
    let ProviderProfile::Oauth(profile) = &mut p.profile else {
        panic!("OAuth fixture seed")
    };
    profile.authorization_url = uri.to_owned();
    profile.token_url = uri.to_owned();
    profile.apis.get_mut("user").unwrap().url = uri.to_owned();
    p
}

fn built(value: &ProviderConfig) -> reqwest::Client {
    match build_http_client(value) {
        Ok(client) => client,
        Err(_) => panic!("B-builder: valid explicit transport must construct the actual client"),
    }
}

fn sanitized(text: &str) {
    assert!(
        !text.to_ascii_lowercase().contains("sentinel"),
        "B-builder: error leaked synthetic sensitive input"
    );
}

// Test readers are bounded too. Only tiny, known fixtures are used here; A owns 1MiB tests.
async fn tiny_body(mut response: reqwest::Response) -> Vec<u8> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.expect("read tiny fixture response") {
        assert!(
            chunk.len() <= 256 - bytes.len(),
            "B fixture exceeded its tiny response allowance"
        );
        bytes.extend_from_slice(&chunk);
    }
    bytes
}

async fn reply(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("GET"))
        .and(path("/probe"))
        .respond_with(response)
        .mount(server)
        .await;
}

#[tokio::test]
async fn b_fixture_tls_ca_san_and_join_preflight_without_service_builder() {
    // Fixture-only control, NOT evidence about setup's client policy or composition C.
    b_tls::run(true, |view| async move {
        let pem = std::fs::read(&view.root).expect("fixture public CA");
        let root = reqwest::Certificate::from_pem(&pem).expect("fixture public CA representation");
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(3))
            .add_root_certificate(root)
            .build()
            .expect("fixture-only trusted control client");
        let response = client
            .get(&view.uri)
            .send()
            .await
            .expect("fixture PKI/SAN must work independently of T0");
        assert_eq!(response.status().as_u16(), 200);
        assert!(
            tiny_body(response).await.as_slice() == br#"{"ok":true}"#.as_slice(),
            "fixture-only control must reach the exact TLS handler response"
        );
        assert_eq!(view.count.load(Ordering::SeqCst), 1);
    })
    .await;
}

#[tokio::test]
async fn b_http_explicit_local_modes_reach_the_listener_with_real_builder_client() {
    for mode in [
        VendorTransportSecurity::LocalInsecure,
        VendorTransportSecurity::IsolatedTest,
    ] {
        let server = MockServer::start().await;
        reply(
            &server,
            ResponseTemplate::new(200).set_body_string("small-body"),
        )
        .await;
        let uri = format!("{}/probe", server.uri());
        let client = built(&target(&uri, mode));
        let response = client
            .get(&uri)
            .send()
            .await
            .expect("actual built-client request");
        assert_eq!(response.status().as_u16(), 200);
        assert!(
            tiny_body(response).await.as_slice() == b"small-body".as_slice(),
            "B-builder: exact tiny plaintext response"
        );
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn b_http_redirect_none_does_not_forward_headers_even_to_an_allowed_local_destination() {
    for status in [302, 307] {
        let server = MockServer::start().await;
        let destination = MockServer::start().await;
        let location = format!("{}/probe", destination.uri());
        reply(
            &server,
            ResponseTemplate::new(status).insert_header("Location", location),
        )
        .await;
        reply(
            &destination,
            ResponseTemplate::new(200).set_body_string("must-not-be-reached"),
        )
        .await;
        let uri = format!("{}/probe", server.uri());
        let client = built(&target(&uri, VendorTransportSecurity::IsolatedTest));
        let response = client
            .get(&uri)
            .header("X-Test-Credential", "credential-sentinel")
            .send()
            .await
            .expect("redirect response must be observable without following it");
        assert_eq!(response.status().as_u16(), status);
        drop(response);
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        assert!(
            destination.received_requests().await.unwrap().is_empty(),
            "B-builder: followed credential-bearing redirect"
        );
    }
}

#[tokio::test]
async fn b_http_gzip_is_actually_decoded_by_the_builder_client_not_blanket_rejected() {
    let server = MockServer::start().await;
    reply(
        &server,
        ResponseTemplate::new(200)
            .insert_header("Content-Encoding", "gzip")
            .set_body_bytes(GZIP_KV),
    )
    .await;
    let uri = format!("{}/probe", server.uri());
    let client = built(&target(&uri, VendorTransportSecurity::IsolatedTest));
    let response = client
        .get(&uri)
        .send()
        .await
        .expect("small valid gzip response");
    assert_eq!(response.status().as_u16(), 200);
    assert!(
        response.headers().get("content-encoding").is_none(),
        "B-builder: reqwest must consume gzip encoding"
    );
    let bytes = tiny_body(response).await;
    assert!(
        bytes.as_slice()
            == br#"{"data":{"data":{"client_secret":"vault-client-sentinel"}}}"#.as_slice(),
        "B-builder: exact known decompressed payload, without exposing it on failure"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn b_http_non_gzip_decoders_remain_disabled_under_additive_cargo_features() {
    for encoding in ["br", "deflate", "zstd"] {
        let server = MockServer::start().await;
        // Intentionally not compressed: a decoder accidentally enabled will fail or alter bytes.
        reply(
            &server,
            ResponseTemplate::new(200)
                .insert_header("Content-Encoding", encoding)
                .set_body_string("wire-only"),
        )
        .await;
        let uri = format!("{}/probe", server.uri());
        let client = built(&target(&uri, VendorTransportSecurity::IsolatedTest));
        let response = client
            .get(&uri)
            .send()
            .await
            .expect("residual encoding remains visible to infra");
        assert_eq!(
            response.headers().get("content-encoding").unwrap(),
            encoding
        );
        assert!(
            tiny_body(response).await.as_slice() == b"wire-only".as_slice(),
            "B-builder: unsupported decoder changed wire bytes"
        );
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn b_http_verified_explicit_ca_and_exact_san_accept_the_actual_tls_peer() {
    b_tls::run(true, |view| async move {
        let mut value = target(&view.uri, VendorTransportSecurity::Verified);
        value.ca_cert_file = Some(view.root);
        let client = built(&value);
        let response = client
            .get(&view.uri)
            .send()
            .await
            .expect("trusted exact-SAN TLS peer");
        assert_eq!(response.status().as_u16(), 200);
        assert!(
            tiny_body(response).await.as_slice() == br#"{"ok":true}"#.as_slice(),
            "B-builder: exact tiny TLS response"
        );
        assert_eq!(view.count.load(Ordering::SeqCst), 1);
    })
    .await;
}

#[tokio::test]
async fn b_http_verified_default_trust_or_unrelated_ca_does_not_accept_fixture_root() {
    b_tls::run(true, |view| async move {
        for roots in [None, Some(view.unrelated_root)] {
            let mut value = target(&view.uri, VendorTransportSecurity::Verified);
            value.ca_cert_file = roots;
            let client = built(&value);
            let error = match client.get(&view.uri).send().await {
                Err(error) => error,
                Ok(_) => panic!("B-builder: untrusted peer was accepted"),
            };
            assert!(
                error.is_connect(),
                "B-builder: expected a TLS connection failure"
            );
            sanitized(&format!("{error:?} {error}"));
            assert_eq!(
                view.count.load(Ordering::SeqCst),
                0,
                "B-builder: TLS failure reached handler"
            );
        }
    })
    .await;
}

#[tokio::test]
async fn b_http_verified_wrong_san_is_refused_even_with_the_signing_ca() {
    b_tls::run(false, |view| async move {
        let mut value = target(&view.uri, VendorTransportSecurity::Verified);
        value.ca_cert_file = Some(view.root);
        let client = built(&value);
        let error = match client.get(&view.uri).send().await {
            Err(error) => error,
            Ok(_) => panic!("B-builder: wrong SAN was accepted"),
        };
        assert!(
            error.is_connect(),
            "B-builder: SAN mismatch must be a TLS connection failure"
        );
        sanitized(&format!("{error:?} {error}"));
        assert_eq!(view.count.load(Ordering::SeqCst), 0);
    })
    .await;
}

#[tokio::test]
async fn b_http_invalid_urls_or_ca_files_fail_closed_without_network() {
    let server = MockServer::start().await;
    let uri = format!("{}/probe", server.uri());
    let directory = tempfile::tempdir().unwrap();
    let malformed = directory.path().join("invalid-ca-sentinel.pem");
    std::fs::write(&malformed, "invalid-CA-content-sentinel").unwrap();
    let mut cases = vec![target(&uri, VendorTransportSecurity::Verified)];
    let mut credentials = target(&uri, VendorTransportSecurity::IsolatedTest);
    let ProviderProfile::Oauth(profile) = &mut credentials.profile else {
        panic!("OAuth seed")
    };
    profile.token_url = uri.replacen("http://", "http://user:password-sentinel@", 1);
    cases.push(credentials);
    for ca in [
        malformed,
        directory.path().join("missing-ca-sentinel.pem"),
        directory.path().to_owned(),
    ] {
        let mut value = provider("forge");
        value.ca_cert_file = Some(ca);
        cases.push(value);
    }
    for value in cases {
        let error = match build_http_client(&value) {
            Err(error) => error,
            Ok(_) => panic!("B-builder: invalid transport declaration constructed a client"),
        };
        sanitized(&format!("{error:?} {error}"));
    }
    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "B-builder: invalid config performed I/O"
    );
}
