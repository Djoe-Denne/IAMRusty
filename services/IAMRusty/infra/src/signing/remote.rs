//! Digest-only remote signer. Transport policy is shared with boot validation.

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use iam_configuration::{validate_remote_signer_endpoint, SecurityMode};
use iam_domain::error::DomainError;
use iam_domain::port::{SigningCapabilities, SigningProvider, WorkloadIdentity};
use reqwest::{
    header::{HeaderValue, ACCEPT_ENCODING, AUTHORIZATION, CONTENT_ENCODING},
    Client, Response, Url,
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

const OPERATION_TIMEOUT: Duration = Duration::from_secs(10);
const SIGN_LIMIT: usize = 4096;
const PUBLIC_LIMIT: usize = 16384;

/// HTTP adapter; no private material, alternate provider, retry or fallback.
pub struct RemoteSigningProvider {
    client: Client,
    sign_url: Url,
    public_url: Url,
    key_id: String,
    token_ref: String,
    workload: Arc<dyn WorkloadIdentity>,
}

#[derive(Serialize)]
struct SignRequest<'a> {
    key_id: &'a str,
    algorithm: &'static str,
    digest: String,
}
#[derive(Deserialize)]
struct SignResponse {
    signature: String,
}
#[derive(Deserialize)]
struct PublicKeyJson {
    public_key: String,
}

fn failure(message: &str) -> DomainError {
    DomainError::external_service_error("remote_signer", message)
}

fn remote_client_builder() -> reqwest::ClientBuilder {
    Client::builder()
        .timeout(OPERATION_TIMEOUT)
        .min_tls_version(reqwest::tls::Version::TLS_1_2)
        .redirect(reqwest::redirect::Policy::none())
        .referer(false)
        .no_proxy()
        .no_gzip()
        .no_brotli()
        .no_deflate()
}

impl RemoteSigningProvider {
    /// Build a signer with the production-safe Verified policy.
    ///
    /// # Errors
    /// Returns an opaque error for invalid configuration or client initialization.
    pub fn new(
        base_url: impl Into<String>,
        key_id: impl Into<String>,
        algorithm: impl Into<String>,
        token_ref: impl Into<String>,
        workload: Arc<dyn WorkloadIdentity>,
    ) -> Result<Self, DomainError> {
        Self::new_with_security_mode(
            base_url,
            key_id,
            algorithm,
            token_ref,
            workload,
            SecurityMode::Verified,
        )
    }

    /// Build a signer with an explicitly selected security mode.
    /// HTTPS always authenticates both the chain and the server name.
    ///
    /// # Errors
    /// Returns an opaque error for forbidden destinations, algorithms or client initialization.
    pub fn new_with_security_mode(
        base_url: impl Into<String>,
        key_id: impl Into<String>,
        algorithm: impl Into<String>,
        token_ref: impl Into<String>,
        workload: Arc<dyn WorkloadIdentity>,
        mode: SecurityMode,
    ) -> Result<Self, DomainError> {
        let key_id = key_id.into();
        let mut base = validate_remote_signer_endpoint(&base_url.into(), &key_id, mode)
            .map_err(|_| failure("invalid remote signer endpoint configuration"))?;
        if algorithm.into() != "RS256" {
            return Err(failure("unsupported signing algorithm"));
        }
        // A configured prefix is a directory, never replaced by URL joining.
        base.path_segments_mut()
            .map_err(|_| failure("invalid remote signer endpoint configuration"))?
            .pop_if_empty();
        let mut sign_url = base.clone();
        sign_url
            .path_segments_mut()
            .map_err(|_| failure("invalid remote signer endpoint configuration"))?
            .push("sign");
        let mut public_url = base;
        public_url
            .path_segments_mut()
            .map_err(|_| failure("invalid remote signer endpoint configuration"))?
            .push("keys")
            .push(&key_id);
        let client = remote_client_builder()
            .build()
            .map_err(|_| failure("HTTP client initialization failed"))?;
        Ok(Self {
            client,
            sign_url,
            public_url,
            key_id,
            token_ref: token_ref.into(),
            workload,
        })
    }

    async fn authorization(&self) -> Result<HeaderValue, DomainError> {
        let credential = self
            .workload
            .resolve(&self.token_ref)
            .await
            .map_err(|_| failure("credential resolution failed"))?;
        let secret = credential.secret;
        let bearer = secret
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("Bearer "));
        let token = if bearer { &secret[7..] } else { &secret };
        if token.trim().is_empty() {
            return Err(failure("invalid credential"));
        }
        let value = if bearer {
            secret
        } else {
            format!("Bearer {secret}")
        };
        let mut header =
            HeaderValue::from_str(&value).map_err(|_| failure("invalid credential"))?;
        header.set_sensitive(true);
        Ok(header)
    }

    async fn bounded_response(
        mut response: Response,
        limit: usize,
        operation: &str,
    ) -> Result<Vec<u8>, DomainError> {
        if !response.status().is_success() {
            // Do not read, drain or retain any part of an error or redirect body.
            return Err(failure(&format!(
                "{operation} HTTP {}",
                response.status().as_u16()
            )));
        }
        if response
            .headers()
            .get_all(CONTENT_ENCODING)
            .iter()
            .any(|encoding| !encoding.as_bytes().eq_ignore_ascii_case(b"identity"))
        {
            return Err(failure("unsupported response encoding"));
        }
        if response
            .content_length()
            .is_some_and(|length| length > limit as u64)
        {
            return Err(failure("response too large"));
        }
        let mut body = Vec::with_capacity(limit);
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| failure("response transport failed"))?
        {
            if chunk.len() > limit - body.len() {
                return Err(failure("response too large"));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }
}

#[async_trait]
impl SigningProvider for RemoteSigningProvider {
    async fn sign_digest(&self, digest: &[u8]) -> Result<Vec<u8>, DomainError> {
        if digest.len() != 32 {
            return Err(failure("invalid signing digest length"));
        }
        tokio::time::timeout(OPERATION_TIMEOUT, async {
            let auth = self.authorization().await?;
            let body = SignRequest {
                key_id: &self.key_id,
                algorithm: "RS256",
                digest: STANDARD.encode(digest),
            };
            let response = self
                .client
                .post(self.sign_url.clone())
                .header(AUTHORIZATION, auth)
                .header(ACCEPT_ENCODING, "identity")
                .json(&body)
                .send()
                .await
                .map_err(|_| failure("sign request failed"))?;
            let bytes = Self::bounded_response(response, SIGN_LIMIT, "sign").await?;
            let parsed: SignResponse =
                serde_json::from_slice(&bytes).map_err(|_| failure("invalid sign response"))?;
            STANDARD
                .decode(parsed.signature.trim())
                .map_err(|_| failure("invalid signature encoding"))
        })
        .await
        .map_err(|_| failure("sign operation timed out"))?
    }

    async fn public_key(&self) -> Result<String, DomainError> {
        tokio::time::timeout(OPERATION_TIMEOUT, async {
            let auth = self.authorization().await?;
            let response = self
                .client
                .get(self.public_url.clone())
                .header(AUTHORIZATION, auth)
                .header(ACCEPT_ENCODING, "identity")
                .send()
                .await
                .map_err(|_| failure("public key request failed"))?;
            let bytes = Self::bounded_response(response, PUBLIC_LIMIT, "keys").await?;
            let text =
                std::str::from_utf8(&bytes).map_err(|_| failure("invalid public key encoding"))?;
            let pem = serde_json::from_slice::<PublicKeyJson>(&bytes)
                .map_or_else(|_| text.to_owned(), |json| json.public_key);
            let pem = pem.trim().to_owned();
            if pem.contains("PRIVATE KEY") {
                return Err(failure("remote signer must not return a private key"));
            }
            if !pem.contains("PUBLIC KEY") {
                return Err(failure(
                    "remote signer public key response is not a public PEM",
                ));
            }
            Ok(pem)
        })
        .await
        .map_err(|_| failure("public key operation timed out"))?
    }

    fn capabilities(&self) -> SigningCapabilities {
        SigningCapabilities {
            sign_digest: true,
            public_key_available: true,
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/fixtures/remote_signer/mod.rs"]
mod remote_signer_fixtures;

#[cfg(test)]
mod tests {
    use super::remote_signer_fixtures::service::{serve_raw, Framing, RawResponse};
    use super::*;
    use crate::signing::StaticCredential;
    use rcgen::{
        BasicConstraints, Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa,
        KeyPair, KeyUsagePurpose,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn ca(name: &str) -> (Certificate, KeyPair) {
        let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
        params.distinguished_name.push(DnType::CommonName, name);
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
        ];
        let key = KeyPair::generate().unwrap();
        (params.self_signed(&key).unwrap(), key)
    }

    struct TlsSigner {
        url: String,
        requests: Arc<AtomicUsize>,
        task: Option<tokio::task::JoinHandle<()>>,
    }

    impl TlsSigner {
        async fn start(
            issuer: &Certificate,
            issuer_key: &KeyPair,
            correct_name: bool,
            response: RawResponse,
        ) -> Self {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
            let names = if correct_name {
                vec!["127.0.0.1".into()]
            } else {
                vec!["wrong-signer.invalid".into()]
            };
            let mut params = CertificateParams::new(names).unwrap();
            params
                .distinguished_name
                .push(DnType::CommonName, "remote fixture server");
            params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
            params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
            let key = KeyPair::generate().unwrap();
            let leaf = params.signed_by(&key, issuer, issuer_key).unwrap();
            let config = rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(
                    vec![leaf.der().clone(), issuer.der().clone()],
                    rustls::pki_types::PrivateKeyDer::Pkcs8(
                        rustls::pki_types::PrivatePkcs8KeyDer::from(key.serialize_der()),
                    ),
                )
                .unwrap();
            let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!("https://{}", listener.local_addr().unwrap());
            let requests = Arc::new(AtomicUsize::new(0));
            let received = requests.clone();
            let task = tokio::spawn(async move {
                let (stream, _) = listener.accept().await.unwrap();
                if let Ok(stream) = acceptor.accept(stream).await {
                    let _ = serve_raw(stream, response, received).await;
                }
            });
            Self {
                url,
                requests,
                task: Some(task),
            }
        }

        async fn shutdown(mut self) {
            if let Some(task) = self.task.take() {
                task.abort();
                let _ = task.await;
            }
        }
    }

    impl Drop for TlsSigner {
        fn drop(&mut self) {
            if let Some(task) = &self.task {
                task.abort();
            }
        }
    }

    fn trusted_provider(url: &str, root: &Certificate) -> RemoteSigningProvider {
        let wi = Arc::new(StaticCredential::from_pair("ref", "synthetic-token").unwrap());
        let mut provider = RemoteSigningProvider::new(url, "kid-1", "RS256", "ref", wi).unwrap();
        // Test-private trust root only. All other policy comes from the exact
        // production builder; no arbitrary public client injection exists.
        provider.client = remote_client_builder()
            .add_root_certificate(reqwest::Certificate::from_pem(root.pem().as_bytes()).unwrap())
            .build()
            .unwrap();
        provider
    }

    #[tokio::test]
    async fn remote_tls_authenticates_server_name() {
        let (issuer, key) = ca("remote fixture CA");
        let (foreign, _) = ca("foreign fixture CA");
        for public in [false, true] {
            for (correct_name, trusted) in [(true, true), (false, true), (true, false)] {
                let response = RawResponse::ok(serde_json::json!({"signature":"AQ==", "public_key":"-----BEGIN PUBLIC KEY-----\nfixture\n-----END PUBLIC KEY-----"}).to_string().into_bytes(), Framing::Length);
                let server = TlsSigner::start(&issuer, &key, correct_name, response).await;
                let provider =
                    trusted_provider(&server.url, if trusted { &issuer } else { &foreign });
                assert!(provider.authorization().await.unwrap().is_sensitive());
                let accepted = if public {
                    provider.public_key().await.is_ok()
                } else {
                    provider.sign_digest(&[0; 32]).await.is_ok()
                };
                let requests = server.requests.load(Ordering::SeqCst);
                server.shutdown().await;
                assert_eq!(accepted, correct_name && trusted);
                assert_eq!(requests, usize::from(correct_name && trusted));
            }
        }
    }

    #[tokio::test]
    async fn remote_tls_redirect_downgrade_never_reaches_target() {
        use super::remote_signer_fixtures::service::RawSigner;
        let (issuer, key) = ca("remote redirect fixture CA");
        let target = RawSigner::start(RawResponse::ok(Vec::new(), Framing::Length)).await;
        for status in [301, 302, 303, 307, 308] {
            for public in [false, true] {
                let mut response = RawResponse::ok(Vec::new(), Framing::Length);
                response.status = status;
                response
                    .headers
                    .push(("Location".into(), target.url.clone()));
                let server = TlsSigner::start(&issuer, &key, true, response).await;
                let provider = trusted_provider(&server.url, &issuer);
                let refused = if public {
                    provider.public_key().await.is_err()
                } else {
                    provider.sign_digest(&[0; 32]).await.is_err()
                };
                let initial = server.requests.load(Ordering::SeqCst);
                server.shutdown().await;
                assert!(refused);
                assert_eq!(initial, 1);
                assert_eq!(target.requests.load(Ordering::SeqCst), 0);
            }
        }
        target.shutdown().await;
    }
}
