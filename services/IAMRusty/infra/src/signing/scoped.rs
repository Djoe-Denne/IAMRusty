//! Resolve Sign/GetPublicKey from the freshly selected writer binding, not boot kid.
use super::{TransitClientConfig, TransitSigningProvider};
use async_trait::async_trait;
use iam_domain::{
    entity::signing_key::{SigningKey, SigningProviderType, SigningScope, TrustScope},
    error::DomainError,
    port::{SigningCapabilities, SigningProvider},
};
use std::sync::Arc;

/// Local composition adapter using existing Sign/GetPublicKey ports. Never caches
/// Active authority. Only the selected platform provider/credential may be used.
pub struct ScopedSigningProvider {
    provider_type: SigningProviderType,
    key_ref: String,
    credential_ref: Option<String>,
    transit: Option<TransitClientConfig>,
    transit_transport: Option<Arc<reqwest::Client>>,
    unversioned: Option<Arc<dyn SigningProvider>>,
}

impl ScopedSigningProvider {
    pub fn transit(
        config: TransitClientConfig,
        key_ref: String,
        credential_ref: String,
    ) -> Result<Self, DomainError> {
        iam_domain::entity::signing_key::require_transit_key_name(&key_ref)?;
        if config.token_ref != credential_ref {
            return Err(DomainError::InvalidSigningKeyMaterial);
        }
        Ok(Self {
            provider_type: SigningProviderType::OpenBaoTransit,
            key_ref,
            credential_ref: Some(credential_ref),
            transit_transport: Some(TransitSigningProvider::transport()?),
            transit: Some(config),
            unversioned: None,
        })
    }

    /// Explicit nonprod/local or remote bootstrap adapter. The public comparison
    /// in the shared codec prevents a different material from using this delegate.
    pub fn unversioned(
        provider_type: SigningProviderType,
        key_ref: String,
        credential_ref: Option<String>,
        delegate: Arc<dyn SigningProvider>,
    ) -> Result<Self, DomainError> {
        if !matches!(
            provider_type,
            SigningProviderType::PemFile | SigningProviderType::RemoteHttp
        ) {
            return Err(DomainError::InvalidSigningKeyMaterial);
        }
        Ok(Self {
            provider_type,
            key_ref,
            credential_ref,
            transit: None,
            transit_transport: None,
            unversioned: Some(delegate),
        })
    }

    fn require_scope_binding(&self, key: &SigningKey) -> Result<(), DomainError> {
        SigningScope::of(key).validate()?;
        if key.trust_scope != TrustScope::Platform
            || key.organization_id.is_some()
            || key.provider_type != self.provider_type
            || key.provider_key_ref != self.key_ref
            || key.credential_ref != self.credential_ref
        {
            return Err(DomainError::InvalidSigningKeyMaterial);
        }
        Ok(())
    }

    fn bound_transit_provider(
        &self,
        key: &SigningKey,
    ) -> Result<TransitSigningProvider, DomainError> {
        self.require_scope_binding(key)?;
        let transit = self
            .transit
            .as_ref()
            .ok_or(DomainError::InvalidSigningKeyMaterial)?;
        let version =
            iam_domain::entity::signing_key::require_transit_key_version(key.provider_key_version)?;
        TransitSigningProvider::with_transport(
            self.transit_transport
                .clone()
                .ok_or(DomainError::InvalidSigningKeyMaterial)?,
            transit.base_url.clone(),
            &key.provider_key_ref,
            version,
            key.credential_ref
                .as_deref()
                .ok_or(DomainError::InvalidSigningKeyMaterial)?,
            transit.workload.clone(),
            Some(key.public_key.clone()),
        )
    }

    fn provider_for(&self, key: &SigningKey) -> Result<Arc<dyn SigningProvider>, DomainError> {
        self.require_scope_binding(key)?;
        if self.transit.is_some() {
            return Ok(Arc::new(self.bound_transit_provider(key)?));
        }
        if key.provider_key_version.is_some() {
            return Err(DomainError::InvalidSigningKeyMaterial);
        }
        self.unversioned
            .clone()
            .ok_or(DomainError::InvalidSigningKeyMaterial)
    }
}

#[async_trait]
impl SigningProvider for ScopedSigningProvider {
    async fn sign_digest(&self, _: &[u8]) -> Result<Vec<u8>, DomainError> {
        Err(DomainError::InvalidSigningKeyMaterial)
    }
    async fn public_key(&self) -> Result<String, DomainError> {
        Err(DomainError::InvalidSigningKeyMaterial)
    }
    async fn sign_digest_for(
        &self,
        key: &SigningKey,
        digest: &[u8],
    ) -> Result<Vec<u8>, DomainError> {
        self.provider_for(key)?.sign_digest(digest).await
    }
    async fn public_key_for(&self, key: &SigningKey) -> Result<String, DomainError> {
        self.provider_for(key)?.public_key().await
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
    use iam_domain::{
        entity::signing_key::SigningKeyStatus,
        port::{WorkloadCredential, WorkloadIdentity},
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Credentials(AtomicUsize);
    #[async_trait]
    impl WorkloadIdentity for Credentials {
        async fn resolve(&self, reference: &str) -> Result<WorkloadCredential, DomainError> {
            assert_eq!(reference, "platform-credential");
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(WorkloadCredential {
                secret: "nonproduction-unit-credential".into(),
            })
        }
    }
    #[tokio::test(flavor = "current_thread")]
    async fn transport_is_shared_across_operations_and_versions_but_credentials_and_binding_stay_fresh(
    ) {
        use base64::{engine::general_purpose::STANDARD, Engine};
        use rsa::{
            pkcs8::DecodePrivateKey,
            signature::{hazmat::PrehashSigner, SignatureEncoding},
        };
        use wiremock::{
            matchers::{body_partial_json, header, method, path},
            Mock, MockServer, ResponseTemplate,
        };
        let server = MockServer::start().await;
        let public = include_str!("../../../config/keys/test-platform.pub");
        let private = rsa::RsaPrivateKey::from_pkcs8_pem(include_str!(
            "../../../config/keys/test-platform.pem"
        ))
        .unwrap();
        let signer = rsa::pkcs1v15::SigningKey::<sha2::Sha256>::new(private);
        let digest = [3u8; 32];
        let signature = STANDARD.encode(signer.sign_prehash(&digest).unwrap().to_vec());
        Mock::given(method("GET")).and(path("/v1/transit/keys/platform"))
            .and(header("X-Vault-Token","nonproduction-unit-credential"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":{"type":"rsa-2048","exportable":false,"supports_signing":true,"latest_version":99,"keys":{"7":{"public_key":public},"8":{"public_key":public}}}}))).expect(4).mount(&server).await;
        for version in [7, 8] {
            Mock::given(method("POST")).and(path("/v1/transit/sign/platform"))
                .and(header("X-Vault-Token","nonproduction-unit-credential"))
                .and(body_partial_json(serde_json::json!({"key_version":version,"prehashed":true,"hash_algorithm":"sha2-256","signature_algorithm":"pkcs1v15","input":STANDARD.encode(digest)})))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":{"signature":format!("vault:v{version}:{signature}")}}))).expect(2).mount(&server).await;
        }
        let credentials = Arc::new(Credentials(AtomicUsize::new(0)));
        let transports_before = super::super::transit::transport_constructions();
        let scoped = ScopedSigningProvider::transit(
            TransitClientConfig {
                base_url: server.uri(),
                token_ref: "platform-credential".into(),
                workload: credentials.clone(),
            },
            "platform".into(),
            "platform-credential".into(),
        )
        .unwrap();
        let now = chrono::Utc::now();
        let mut key = SigningKey {
            id: uuid::Uuid::new_v4(),
            kid: iam_domain::entity::signing_key::opaque_kid(),
            algorithm: "RS256".into(),
            trust_scope: TrustScope::Platform,
            organization_id: None,
            issuer: "https://issuer.example/iam".into(),
            provider_type: SigningProviderType::OpenBaoTransit,
            provider_key_ref: "platform".into(),
            provider_key_version: Some(7),
            credential_ref: Some("platform-credential".into()),
            public_key: public.into(),
            status: SigningKeyStatus::Active,
            created_at: now,
            updated_at: now,
        };
        for version in [7, 8, 7, 8] {
            key.provider_key_version = Some(version);
            let bound = scoped.bound_transit_provider(&key).unwrap();
            assert!(Arc::ptr_eq(&bound.client,scoped.transit_transport.as_ref().unwrap()),"actual request adapter holds the SAME configured client/pool, not two new clients per token");
            assert_eq!(scoped.public_key_for(&key).await.unwrap(), public);
            assert_eq!(
                scoped.sign_digest_for(&key, &digest).await.unwrap(),
                STANDARD.decode(&signature).unwrap()
            );
        }
        assert_eq!(
            credentials.0.load(Ordering::SeqCst),
            8,
            "transport reuse never memoizes request credentials"
        );
        assert_eq!(super::super::transit::transport_constructions(), transports_before + 1,
            "all REAL public_key_for/sign_digest_for calls must use the one composition transport; standalone new per operation is a regression");
        for field in [0, 1, 2, 3] {
            let mut wrong = key.clone();
            match field {
                0 => wrong.credential_ref = Some("other".into()),
                1 => wrong.provider_key_ref = "other".into(),
                2 => wrong.provider_key_version = Some(0),
                _ => {
                    wrong.organization_id = Some(uuid::Uuid::new_v4());
                    wrong.trust_scope = TrustScope::Organization;
                }
            }
            assert!(scoped.public_key_for(&wrong).await.is_err());
            assert!(scoped.sign_digest_for(&wrong, &digest).await.is_err());
        }
        assert_eq!(server.received_requests().await.unwrap().len(), 8);
        assert_eq!(
            credentials.0.load(Ordering::SeqCst),
            8,
            "invalid bindings rejected before credentials/network"
        );
        // Neutral same public pair tests transport + pin protocol, NOT distinct
        // N/N+1 crypto. The codec's live verification/primary fence tests remain.
    }
}
