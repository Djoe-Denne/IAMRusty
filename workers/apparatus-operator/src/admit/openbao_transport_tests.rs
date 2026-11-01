//! Real loopback TLS at the added P4 transport boundary; no containers or root token.
use super::http_post_json_with_ca;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;

fn endpoint(host: &str) -> (String, String, std::thread::JoinHandle<()>) {
    let rcgen::CertifiedKey { cert, key_pair } =
        rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).expect("test leaf");
    let pem = cert.pem();
    let key = rustls::pki_types::PrivatePkcs8KeyDer::from(key_pair.serialize_der());
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("protocols")
    .with_no_client_auth()
    .with_single_cert(vec![cert.der().clone()], key.into())
    .expect("server config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let port = listener.local_addr().expect("address").port();
    let handle = std::thread::spawn(move || {
        let (socket, _) = listener.accept().expect("accept");
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .expect("timeout");
        let session = rustls::ServerConnection::new(Arc::new(config)).expect("session");
        let mut stream = rustls::StreamOwned::new(session, socket);
        let mut buffer = [0; 4096];
        // A trust/name rejection is expected to terminate the handshake here.
        if stream.read(&mut buffer).is_ok() {
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\nConnection: close\r\n\r\n{\"ok\":true}",
            );
            let _ = stream.flush();
        }
    });
    (format!("https://{host}:{port}"), pem, handle)
}

#[test]
fn openbao_https_accepts_explicit_ca_and_rejects_foreign_ca_and_wrong_name() {
    for case in ["trusted", "foreign", "wrong-name"] {
        let (url, pem, server) = endpoint(if case == "wrong-name" {
            "127.0.0.1"
        } else {
            "localhost"
        });
        let mut ca = tempfile::NamedTempFile::new().expect("CA file");
        let pem = if case == "foreign" {
            rcgen::generate_simple_self_signed(vec!["foreign.invalid".to_owned()])
                .expect("foreign leaf")
                .cert
                .pem()
        } else {
            pem
        };
        ca.write_all(pem.as_bytes()).expect("CA bytes");
        let result = http_post_json_with_ca(
            &url,
            "/v1/auth/kubernetes/login",
            "{}",
            Some(ca.path().to_string_lossy().into_owned()),
        );
        if case == "trusted" {
            assert_eq!(
                result.expect("trusted HTTPS"),
                (200, "{\"ok\":true}".to_owned())
            );
        } else {
            assert!(result.is_err(), "{case} must be rejected");
        }
        server.join().expect("test server");
    }
}

#[test]
fn openbao_https_requires_ca_and_rejects_non_origin_addresses() {
    for origin in [
        "https://localhost:1",
        "file:///secret",
        "https://user:password@localhost",
        "https://localhost/path",
    ] {
        assert!(http_post_json_with_ca(origin, "/v1/auth/kubernetes/login", "{}", None).is_err());
    }
}
