//! Integration-only data helpers; the core signed fixture is also used privately.

use super::fixture::{jwk, must, nonce, TestKey, CALLBACK, KID};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use flate2::{write::GzEncoder, Compression};
use idp_connect_contract::AuthorizeAuthenticationRequest;
use jsonwebtoken::DecodingKey;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::Write;
use wiremock::ResponseTemplate;

pub const CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
pub const LIMIT: usize = 1_048_576;

pub fn authorize() -> AuthorizeAuthenticationRequest {
    AuthorizeAuthenticationRequest {
        redirect_uri: CALLBACK.to_owned(),
        state: "opaque+state&=canary".to_owned(),
        code_challenge: Some(CHALLENGE.to_owned()),
        code_challenge_method: Some("S256".to_owned()),
        nonce: nonce(),
    }
}

pub fn advertised_decoding_key(key: &TestKey) -> DecodingKey {
    let value = jwk(key, KID);
    let Some(n) = value["n"].as_str() else {
        panic!("fixture RSA modulus required");
    };
    let Some(e) = value["e"].as_str() else {
        panic!("fixture RSA exponent required");
    };
    must(
        DecodingKey::from_rsa_components(n, e),
        "decoder from actual advertised JWK components",
    )
}

pub fn access_hash(access: &str) -> String {
    // Independent RFC7518/SHA256 left-half vector, not AccessTokenHash production API.
    let digest = Sha256::digest(access.as_bytes());
    URL_SAFE_NO_PAD.encode(&digest[..16])
}

pub fn padded(value: &Value, size: usize) -> Vec<u8> {
    let mut bytes = must(serde_json::to_vec(value), "JSON fixture");
    assert!(bytes.len() <= size, "fixture fits byte edge");
    bytes.resize(size, b' ');
    bytes
}

pub fn duplicate_field(value: &Value, key: &str, duplicate: &Value) -> Vec<u8> {
    let original = must(serde_json::to_string(value), "fixture object serialization");
    let Some(tail) = original.strip_prefix('{') else {
        panic!("fixture object required");
    };
    let key = must(serde_json::to_string(key), "fixture field serialization");
    let duplicate = must(
        serde_json::to_string(duplicate),
        "fixture value serialization",
    );
    format!("{{{key}:{duplicate},{tail}").into_bytes()
}

pub fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    must(encoder.write_all(bytes), "gzip fixture write");
    must(encoder.finish(), "gzip fixture finish")
}

pub fn body(bytes: Vec<u8>, encoding: Option<&str>) -> ResponseTemplate {
    let response = ResponseTemplate::new(200)
        .set_body_bytes(bytes)
        .insert_header("content-type", "application/json");
    match encoding {
        Some(encoding) => response.insert_header("content-encoding", encoding),
        None => response,
    }
}
