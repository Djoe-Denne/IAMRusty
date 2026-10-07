//! OpenBao Transit SigningProvider — Sign only, never export private key material.

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use iam_domain::error::DomainError;
use iam_domain::port::{SigningCapabilities, SigningProvider, WorkloadIdentity};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tracing::debug;

pub use iam_domain::entity::signing_key::FORBIDDEN_TRANSIT_KEY_NAME;

// Structural construction evidence only; no production observer/cache. Each
// current-thread unit executor owns its count (parallel tests cannot interfere).
#[cfg(test)]
thread_local! {
    static TRANSPORT_CONSTRUCTIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
pub(super) fn transport_constructions() -> usize {
    TRANSPORT_CONSTRUCTIONS.with(std::cell::Cell::get)
}

/// OpenBao / Vault Transit `POST /v1/transit/sign/{name}` adapter.
///
/// Never uses Cosign/`apparatus-p4-cosign` paths. Credentials come from
/// [`WorkloadIdentity`] (typically [`crate::signing::StaticCredential`]).
pub struct TransitSigningProvider {
    pub(super) client: Arc<Client>,
    base_url: String,
    key_name: String,
    key_version: u32,
    token_ref: String,
    workload: Arc<dyn WorkloadIdentity>,
    /// Expected public key bound to this exact version, never a latest fallback.
    public_key_pem: Option<String>,
}

impl TransitSigningProvider {
    /// Create a Transit Sign client.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the HTTP client cannot be built or the key name is forbidden.
    pub fn new(
        base_url: impl Into<String>,
        key_name: impl Into<String>,
        key_version: u32,
        token_ref: impl Into<String>,
        workload: Arc<dyn WorkloadIdentity>,
        public_key_pem: Option<String>,
    ) -> Result<Self, DomainError> {
        let key_name = key_name.into();
        iam_domain::entity::signing_key::require_transit_key_name(&key_name)?;
        iam_domain::entity::signing_key::require_transit_key_version(Some(key_version))?;
        Self::with_transport(
            Self::transport()?,
            base_url,
            key_name,
            key_version,
            token_ref,
            workload,
            public_key_pem,
        )
    }

    /// One constrained transport per configured scoped adapter (standalone
    /// `new` still creates its own for cold fixtures/probes).
    pub(super) fn transport() -> Result<Arc<Client>, DomainError> {
        #[cfg(test)]
        TRANSPORT_CONSTRUCTIONS.with(|count| count.set(count.get() + 1));
        Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| {
                DomainError::external_service_error(
                    "openbao_transit",
                    "HTTP client initialization failed",
                )
            })
            .map(Arc::new)
    }

    // Internal only: injected transport can only originate from the same
    // no-redirect/10s builder, never an arbitrary external client policy.
    pub(super) fn with_transport(
        client: Arc<Client>,
        base_url: impl Into<String>,
        key_name: impl Into<String>,
        key_version: u32,
        token_ref: impl Into<String>,
        workload: Arc<dyn WorkloadIdentity>,
        public_key_pem: Option<String>,
    ) -> Result<Self, DomainError> {
        let key_name = key_name.into();
        iam_domain::entity::signing_key::require_transit_key_name(&key_name)?;
        iam_domain::entity::signing_key::require_transit_key_version(Some(key_version))?;
        let base_url = base_url.into();
        let parsed = reqwest::Url::parse(&base_url).map_err(|_| DomainError::InvalidToken)?;
        if !matches!(parsed.scheme(), "http" | "https")
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(DomainError::InvalidToken);
        }
        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            key_name,
            key_version,
            token_ref: token_ref.into(),
            workload,
            public_key_pem,
        })
    }

    fn endpoint(&self, operation: &str) -> Result<reqwest::Url, DomainError> {
        let mut url = reqwest::Url::parse(&self.base_url).map_err(|_| DomainError::InvalidToken)?;
        url.path_segments_mut()
            .map_err(|_| DomainError::InvalidToken)?
            .pop_if_empty()
            .push("v1")
            .push("transit")
            .push(operation)
            .push(&self.key_name);
        Ok(url)
    }

    fn require_binding(
        &self,
        key: &iam_domain::entity::signing_key::SigningKey,
    ) -> Result<(), DomainError> {
        use iam_domain::entity::signing_key::{SigningProviderType, SigningScope};
        SigningScope::of(key).validate()?;
        if key.provider_type != SigningProviderType::OpenBaoTransit
            || key.provider_key_ref != self.key_name
            || key.provider_key_version != Some(self.key_version)
            || key.credential_ref.as_deref() != Some(self.token_ref.as_str())
        {
            return Err(DomainError::InvalidSigningKeyMaterial);
        }
        if let Some(org) = key.organization_id {
            iam_domain::entity::signing_key::require_org_transit_binding(
                org,
                &key.provider_key_ref,
                key.credential_ref.as_deref(),
            )?;
        }
        Ok(())
    }

    /// `POST /v1/transit/keys/{name}` with `type=rsa-2048`, `exportable=false`.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] on HTTP or auth failure.
    pub async fn create_rsa2048_key(&self) -> Result<(), DomainError> {
        // Creation must not silently enroll an existing name. A race resulting in
        // an existing compatible key still supplies no exclusive-creation proof.
        let cred = self.workload.resolve(&self.token_ref).await?;
        let existing = self
            .client
            .get(self.endpoint("keys")?)
            .header("X-Vault-Token", &cred.secret)
            .send()
            .await
            .map_err(|_| transit_error("create preflight failed"))?;
        if existing.status() != reqwest::StatusCode::NOT_FOUND || self.key_version != 1 {
            return Err(transit_error(
                "key creation is not an unambiguous initial enrollment",
            ));
        }
        let cred = self.workload.resolve(&self.token_ref).await?;
        let url = self.endpoint("keys")?;
        let body = serde_json::json!({
            "type": "rsa-2048",
            "exportable": false,
        });
        debug!("OpenBao Transit create key");
        let response = self
            .client
            .post(url)
            .header("X-Vault-Token", &cred.secret)
            .json(&body)
            .send()
            .await
            .map_err(|_| {
                DomainError::external_service_error("openbao_transit", "request failed")
            })?;
        if !response.status().is_success() {
            let status = response.status();
            return Err(DomainError::external_service_error(
                "openbao_transit",
                &format!("create key HTTP {status}"),
            ));
        }
        let metadata = self.read_key().await?;
        require_versionable_rsa2048(&metadata)?;
        if metadata.latest_version != Some(self.key_version) {
            return Err(transit_error("ambiguous created key version"));
        }
        self.fetch_public_key_pem().await?;
        Ok(())
    }

    /// Rotate the same provider name exactly once. No retry after ambiguous I/O.
    /// Returns only a verified successor number, not an exclusive-creation claim.
    pub async fn rotate_rsa2048_key(&self) -> Result<u32, DomainError> {
        let before = self.read_key().await?;
        require_versionable_rsa2048(&before)?;
        if before.latest_version != Some(self.key_version) {
            return Err(transit_error(
                "binding is not the current authorized provider version",
            ));
        }
        exact_public(&before, self.key_version)?;
        let next = self
            .key_version
            .checked_add(1)
            .ok_or_else(|| transit_error("provider version exhausted"))?;
        let cred = self.workload.resolve(&self.token_ref).await?;
        let mut url = self.endpoint("keys")?;
        url.path_segments_mut()
            .map_err(|_| DomainError::InvalidToken)?
            .push("rotate");
        let response = self
            .client
            .post(url)
            .header("X-Vault-Token", &cred.secret)
            .send()
            .await
            .map_err(|_| transit_error("rotation outcome indeterminate; do not retry"))?;
        if !response.status().is_success() {
            return Err(transit_error("rotation failed; do not retry automatically"));
        }
        let after = self.read_key().await?;
        require_versionable_rsa2048(&after)?;
        if after.latest_version != Some(next) {
            return Err(transit_error("ambiguous successor; enrollment stopped"));
        }
        exact_public(&after, next)?;
        Ok(next)
    }

    /// Enrollment verifies documented provider properties and this exact public,
    /// without using latest as the pin or claiming exclusive creation.
    pub async fn enrollment_public_key(&self) -> Result<String, DomainError> {
        let data = self.read_key().await?;
        // Sign-only enrollment admits the full domain RSA range. Create/Rotate
        // retain their narrower rsa-2048 capability and are never inferred here.
        let public = exact_public(&data, self.key_version)?;
        let parsed = iam_domain::entity::signing_key::parse_signing_public_key(&public)?;
        require_rsa_enrollment(&data, self.key_version, &parsed)?;
        self.confirm_supplied_public(&parsed)?;
        Ok(public)
    }

    fn confirm_supplied_public(&self, public: &rsa::RsaPublicKey) -> Result<(), DomainError> {
        if let Some(pem) = &self.public_key_pem {
            if iam_domain::entity::signing_key::parse_signing_public_key(pem)? != *public {
                return Err(DomainError::InvalidSigningKeyMaterial);
            }
        }
        Ok(())
    }

    async fn read_key(&self) -> Result<TransitReadKeyData, DomainError> {
        let cred = self.workload.resolve(&self.token_ref).await?;
        let url = self.endpoint("keys")?;
        debug!("OpenBao Transit read key");
        let response = self
            .client
            .get(url)
            .header("X-Vault-Token", &cred.secret)
            .send()
            .await
            .map_err(|_| {
                DomainError::external_service_error("openbao_transit", "request failed")
            })?;
        if !response.status().is_success() {
            let status = response.status();
            return Err(DomainError::external_service_error(
                "openbao_transit",
                &format!("read key HTTP {status}"),
            ));
        }
        let parsed: TransitReadKeyResponse = response.json().await.map_err(|_| {
            DomainError::external_service_error("openbao_transit", "invalid key response")
        })?;
        Ok(parsed.data)
    }

    async fn fetch_public_key_pem(&self) -> Result<String, DomainError> {
        exact_public(&self.read_key().await?, self.key_version)
    }
}

fn transit_error(message: &str) -> DomainError {
    DomainError::external_service_error("openbao_transit", message)
}

fn exact_public(data: &TransitReadKeyData, version: u32) -> Result<String, DomainError> {
    data.keys
        .get(&version.to_string())
        .and_then(|v| v.public_key.as_ref())
        .filter(|pem| !pem.trim().is_empty())
        .cloned()
        .ok_or_else(|| transit_error("pinned version public key unavailable"))
}

fn require_versionable_rsa2048(data: &TransitReadKeyData) -> Result<(), DomainError> {
    if data.key_type.as_deref() != Some("rsa-2048")
        || data.exportable != Some(false)
        || data.supports_signing != Some(true)
        || data.latest_version.filter(|v| *v > 0).is_none()
    {
        return Err(transit_error(
            "incompatible or non-versionable provider key",
        ));
    }
    Ok(())
}

fn require_rsa_enrollment(
    data: &TransitReadKeyData,
    version: u32,
    public: &rsa::RsaPublicKey,
) -> Result<(), DomainError> {
    use rsa::traits::PublicKeyParts;
    let bits = public.n().bits();
    if !(2048..=8192).contains(&bits)
        || data.key_type.as_deref() != Some(format!("rsa-{bits}").as_str())
        || data.exportable != Some(false)
        || data.supports_signing != Some(true)
        || data.latest_version.is_none_or(|latest| latest < version)
    {
        return Err(transit_error(
            "incompatible or non-versionable enrollment key",
        ));
    }
    Ok(())
}

fn decode_signature(signature: &str, version: u32) -> Result<Vec<u8>, DomainError> {
    let mut parts = signature.split(':');
    if parts.next() != Some("vault") || parts.next() != Some(format!("v{version}").as_str()) {
        return Err(transit_error("signature provider version mismatch"));
    }
    let raw = parts
        .next()
        .filter(|raw| !raw.is_empty())
        .ok_or_else(|| transit_error("invalid signature format"))?;
    if parts.next().is_some() {
        return Err(transit_error("invalid signature format"));
    }
    STANDARD
        .decode(raw)
        .map_err(|_| transit_error("invalid signature base64"))
}

#[derive(Debug, Deserialize)]
struct TransitSignResponse {
    data: TransitSignData,
}

#[derive(Debug, Deserialize)]
struct TransitSignData {
    signature: String,
}

#[derive(Debug, Deserialize)]
struct TransitReadKeyResponse {
    data: TransitReadKeyData,
}

#[derive(Debug, Deserialize)]
struct TransitReadKeyData {
    #[serde(default)]
    keys: std::collections::HashMap<String, TransitKeyVersion>,
    #[serde(default)]
    latest_version: Option<u32>,
    #[serde(default, rename = "type")]
    key_type: Option<String>,
    #[serde(default)]
    exportable: Option<bool>,
    #[serde(default)]
    supports_signing: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct TransitKeyVersion {
    #[serde(default)]
    public_key: Option<String>,
}

#[async_trait]
impl SigningProvider for TransitSigningProvider {
    async fn sign_digest_for(
        &self,
        key: &iam_domain::entity::signing_key::SigningKey,
        digest: &[u8],
    ) -> Result<Vec<u8>, DomainError> {
        self.require_binding(key)?;
        self.sign_digest(digest).await
    }

    async fn public_key_for(
        &self,
        key: &iam_domain::entity::signing_key::SigningKey,
    ) -> Result<String, DomainError> {
        self.require_binding(key)?;
        self.public_key().await
    }
    async fn sign_digest(&self, digest: &[u8]) -> Result<Vec<u8>, DomainError> {
        if digest.len() != 32 {
            return Err(DomainError::InvalidSigningKeyMaterial);
        }
        let cred = self.workload.resolve(&self.token_ref).await?;
        let url = self.endpoint("sign")?;
        let body = serde_json::json!({
            "input": STANDARD.encode(digest),
            "key_version": self.key_version,
            "prehashed": true,
            "hash_algorithm": "sha2-256",
            "signature_algorithm": "pkcs1v15",
        });

        debug!("OpenBao Transit sign");
        let response = self
            .client
            .post(url)
            .header("X-Vault-Token", &cred.secret)
            .json(&body)
            .send()
            .await
            .map_err(|_| {
                DomainError::external_service_error("openbao_transit", "request failed")
            })?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(DomainError::external_service_error(
                "openbao_transit",
                &format!("HTTP {status}"),
            ));
        }

        let parsed: TransitSignResponse = response.json().await.map_err(|_| {
            DomainError::external_service_error("openbao_transit", "invalid sign response")
        })?;

        decode_signature(&parsed.data.signature, self.key_version)
    }

    async fn public_key(&self) -> Result<String, DomainError> {
        let actual = self.fetch_public_key_pem().await?;
        self.confirm_supplied_public(&iam_domain::entity::signing_key::parse_signing_public_key(
            &actual,
        )?)?;
        Ok(actual)
    }

    fn capabilities(&self) -> SigningCapabilities {
        SigningCapabilities {
            sign_digest: true,
            public_key_available: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signing::StaticCredential;
    use iam_domain::error::DomainError;
    use std::sync::Arc;

    #[test]
    fn new_rejects_forbidden_cosign_transit_key_name() {
        let workload = Arc::new(StaticCredential::default());
        let result = TransitSigningProvider::new(
            "http://127.0.0.1:8200",
            "apparatus-p4-cosign",
            1,
            "openbao-token",
            workload,
            None,
        );
        assert!(
            matches!(
                result,
                Err(DomainError::AuthorizationError(ref msg)) if msg.contains("apparatus-p4-cosign")
            ),
            "TransitSigningProvider::new must refuse apparatus-p4-cosign"
        );
    }

    #[tokio::test]
    async fn enrollment_validates_metadata_and_cached_public_before_any_sign() {
        use wiremock::{
            matchers::{method, path},
            Mock, MockServer, ResponseTemplate,
        };
        let public = include_str!("../../../config/keys/test-platform.pub");
        for case in 0..9 {
            let server = MockServer::start().await;
            let mut data = serde_json::json!({"type":"rsa-2048", "exportable":false, "supports_signing":true,
                "latest_version":8, "keys":{"7":{"public_key":public}}});
            match case {
                0 => {
                    data.as_object_mut().unwrap().remove("supports_signing");
                }
                1 => data["supports_signing"] = false.into(),
                2 => {
                    data.as_object_mut().unwrap().remove("exportable");
                }
                3 => data["exportable"] = true.into(),
                4 => data["type"] = "ecdsa-p256".into(),
                5 => data["type"] = "rsa-4096".into(),
                6 => {
                    data.as_object_mut().unwrap().remove("latest_version");
                }
                7 => data["latest_version"] = 6.into(),
                _ => {}
            }
            Mock::given(method("GET"))
                .and(path("/v1/transit/keys/org-key"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":data})),
                )
                .mount(&server)
                .await;
            let provider = TransitSigningProvider::new(
                server.uri(),
                "org-key",
                7,
                "unit-credential",
                Arc::new(
                    crate::signing::StaticCredential::from_pair("unit-credential", "fixture-token")
                        .unwrap(),
                ),
                Some(public.into()),
            )
            .unwrap();
            assert_eq!(provider.enrollment_public_key().await.is_ok(), case == 8);
            let requests = server.received_requests().await.unwrap();
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].method.as_str(), "GET");
        }
    }

    #[test]
    fn enrollment_keeps_rsa_domain_range_without_claiming_rotate_capability() {
        // Public shape arithmetic only, not a generated private key or PoP.
        // The real 2048-bit positive enrollment and 8192-bit PoP tests remain.
        for bits in [2048, 3072, 4096, 8192] {
            let mut modulus = vec![0u8; bits / 8];
            modulus[0] = 0x80;
            *modulus.last_mut().unwrap() = 1;
            let public = rsa::RsaPublicKey::new_with_max_size(
                rsa::BigUint::from_bytes_be(&modulus),
                rsa::BigUint::from(65537u32),
                bits,
            )
            .unwrap();
            let data: TransitReadKeyData =
                serde_json::from_value(serde_json::json!({"type":format!("rsa-{bits}"),
                "exportable":false,"supports_signing":true,"latest_version":9}))
                .unwrap();
            require_rsa_enrollment(&data, 7, &public).unwrap();
            assert_eq!(require_versionable_rsa2048(&data).is_ok(), bits == 2048);
        }
    }

    #[tokio::test]
    async fn challenge_requests_only_exact_sign_endpoint_and_does_not_follow_redirects() {
        use iam_domain::port::{WorkloadCredential, WorkloadIdentity};
        use wiremock::{
            matchers::{method, path},
            Mock, MockServer, ResponseTemplate,
        };
        struct Credential;
        #[async_trait]
        impl WorkloadIdentity for Credential {
            async fn resolve(&self, _: &str) -> Result<WorkloadCredential, DomainError> {
                Ok(WorkloadCredential {
                    secret: "unit-credential".into(),
                })
            }
        }
        let server = MockServer::start().await;
        let destination = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/transit/sign/org-key"))
            .respond_with(ResponseTemplate::new(302).insert_header("Location", destination.uri()))
            .expect(1)
            .mount(&server)
            .await;
        let provider = TransitSigningProvider::new(
            server.uri(),
            "org-key",
            1,
            "org-credential",
            Arc::new(Credential),
            Some("public-key".into()),
        )
        .unwrap();
        assert!(provider.sign_digest(&[1; 32]).await.is_err());
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url.path(), "/v1/transit/sign/org-key");
        assert!(destination.received_requests().await.unwrap().is_empty());
    }
}
