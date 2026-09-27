//! OrganizationSignerProbe adapter (ADR-0306 test-signer challenge).

use async_trait::async_trait;
use iam_domain::entity::signing_key::{SigningKey, SigningProviderType};
use iam_domain::error::DomainError;
use iam_domain::port::{OrganizationSignerProbe, SigningProvider};
use rsa::pkcs1v15::Pkcs1v15Sign;
use rsa::pkcs8::DecodePublicKey;
use rsa::RsaPublicKey;
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

use super::pem::PemSigningProvider;
use super::rotate::TransitClientConfig;
use super::transit::TransitSigningProvider;

/// Fixed bytes hashed (SHA-256) then signed during an org-signer test.
const CHALLENGE_BYTES: &[u8] = b"aiforall-org-signer-challenge";
const MAX_PEM_BYTES: u64 = 16 * 1024;

/// Filesystem PEM + live Transit probe. Cloud BYOKMS types stay unsupported.
///
/// PEM private keys may only be read from [`pem_root`]. Transit requires a
/// configured [`TransitClientConfig`] — missing URL is a hard error.
#[derive(Clone)]
pub struct DefaultOrganizationSignerProbe {
    pem_root: PathBuf,
    transit: Option<TransitClientConfig>,
}

impl DefaultOrganizationSignerProbe {
    #[must_use]
    pub fn new(pem_root: PathBuf) -> Self {
        Self {
            pem_root,
            transit: None,
        }
    }

    #[must_use]
    pub fn with_transit(mut self, transit: TransitClientConfig) -> Self {
        self.transit = Some(transit);
        self
    }
}

#[async_trait]
impl OrganizationSignerProbe for DefaultOrganizationSignerProbe {
    async fn challenge(&self, key: &SigningKey) -> Result<(), DomainError> {
        match key.provider_type {
            SigningProviderType::PemFile => challenge_pem(&self.pem_root, key).await,
            SigningProviderType::OpenBaoTransit => {
                challenge_transit(self.transit.as_ref(), key).await
            }
            SigningProviderType::AwsKms
            | SigningProviderType::GcpKms
            | SigningProviderType::AzureKeyVault => Err(DomainError::ProviderNotSupported(
                String::from(&key.provider_type),
            )),
        }
    }
}

fn path_policy_error(message: &str) -> DomainError {
    DomainError::AuthorizationError(message.to_string())
}

fn resolve_pem_path(
    pem_root: &Path,
    provider_key_ref: &str,
    organization_id: Option<Uuid>,
) -> Result<PathBuf, DomainError> {
    if provider_key_ref.trim().is_empty() {
        return Err(path_policy_error("PEM path is empty"));
    }
    let requested = Path::new(provider_key_ref);
    if requested
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(path_policy_error("PEM path must not contain '..'"));
    }

    let org_id =
        organization_id.ok_or_else(|| path_policy_error("PEM path requires an organization_id"))?;

    let joined = if requested.is_relative() {
        pem_root.join(requested)
    } else {
        requested.to_path_buf()
    };

    let canonical_root = pem_root.canonicalize().map_err(|e| {
        DomainError::external_service_error(
            "organization_signer_probe",
            &format!("pem_root missing or unreadable: {e}"),
        )
    })?;
    let canonical = joined.canonicalize().map_err(|e| {
        DomainError::external_service_error(
            "organization_signer_probe",
            &format!("private key file missing or unreadable: {e}"),
        )
    })?;

    if !canonical.starts_with(&canonical_root) {
        return Err(path_policy_error(
            "PEM path is outside the allowlisted keys directory",
        ));
    }

    let relative = canonical
        .strip_prefix(&canonical_root)
        .map_err(|_| path_policy_error("PEM path is outside the allowlisted keys directory"))?;
    let org = org_id.to_string();
    let mut comps = relative.components();
    let first_ok = matches!(
        comps.next(),
        Some(Component::Normal(name)) if name == org.as_str()
    );
    if !first_ok || comps.next().is_none() {
        return Err(path_policy_error(
            "PEM path is outside the organization keys directory",
        ));
    }
    Ok(canonical)
}

async fn challenge_pem(pem_root: &Path, key: &SigningKey) -> Result<(), DomainError> {
    let path = resolve_pem_path(pem_root, &key.provider_key_ref, key.organization_id)?;
    let metadata = std::fs::metadata(&path).map_err(|e| {
        DomainError::external_service_error(
            "organization_signer_probe",
            &format!("private key file missing or unreadable: {e}"),
        )
    })?;
    if metadata.len() > MAX_PEM_BYTES {
        return Err(path_policy_error("PEM file exceeds 16 KiB"));
    }
    let private_pem = std::fs::read_to_string(&path).map_err(|e| {
        DomainError::external_service_error(
            "organization_signer_probe",
            &format!("private key file missing or unreadable: {e}"),
        )
    })?;
    if private_pem.len() as u64 > MAX_PEM_BYTES {
        return Err(path_policy_error("PEM file exceeds 16 KiB"));
    }
    let provider = PemSigningProvider::new(&private_pem, &key.public_key)?;
    verify_challenge_signature(&provider, &key.public_key).await
}

async fn challenge_transit(
    transit: Option<&TransitClientConfig>,
    key: &SigningKey,
) -> Result<(), DomainError> {
    let transit = transit.ok_or_else(|| {
        DomainError::external_service_error(
            "openbao_transit",
            "Transit URL not configured — refuse closed",
        )
    })?;
    let token_ref = key
        .credential_ref
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(transit.token_ref.as_str());
    let provider = TransitSigningProvider::new(
        transit.base_url.clone(),
        key.provider_key_ref.clone(),
        token_ref,
        transit.workload.clone(),
        Some(key.public_key.clone()),
    )?;
    verify_challenge_signature(&provider, &key.public_key).await
}

async fn verify_challenge_signature(
    provider: &dyn SigningProvider,
    public_key_pem: &str,
) -> Result<(), DomainError> {
    let digest = Sha256::digest(CHALLENGE_BYTES);
    let signature = provider.sign_digest(&digest).await?;
    let public = RsaPublicKey::from_public_key_pem(public_key_pem)
        .map_err(|e| DomainError::AuthorizationError(format!("invalid RSA public key PEM: {e}")))?;
    public
        .verify(Pkcs1v15Sign::new::<Sha256>(), &digest, &signature)
        .map_err(|e| {
            DomainError::TokenValidationFailed(format!("org-signer challenge verify failed: {e}"))
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use iam_domain::entity::signing_key::{SigningKeyStatus, TrustScope};
    use std::io::Write;
    use uuid::Uuid;

    fn sample_key(
        org_id: Uuid,
        provider_type: SigningProviderType,
        key_ref: &str,
        public_key: &str,
    ) -> SigningKey {
        let now = Utc::now();
        SigningKey {
            id: Uuid::new_v4(),
            kid: Uuid::new_v4().simple().to_string(),
            algorithm: "RS256".to_string(),
            trust_scope: TrustScope::Organization,
            issuer: "http://127.0.0.1/iam/orgs/acme".to_string(),
            provider_type,
            provider_key_ref: key_ref.to_string(),
            credential_ref: None,
            public_key: public_key.to_string(),
            status: SigningKeyStatus::Active,
            organization_id: Some(org_id),
            created_at: now,
            updated_at: now,
        }
    }

    fn probe() -> DefaultOrganizationSignerProbe {
        DefaultOrganizationSignerProbe::new(std::env::temp_dir())
    }

    #[tokio::test]
    async fn pem_challenge_signs_and_verifies() {
        let private = include_str!("../../../config/keys/test-platform.pem");
        let public = include_str!("../../../config/keys/test-platform.pub");
        let org_id = Uuid::new_v4();
        let pem_root = std::env::temp_dir().join(format!("aiforall-probe-root-{}", Uuid::new_v4()));
        let org_dir = pem_root.join(org_id.to_string());
        std::fs::create_dir_all(&org_dir).expect("org pem dir");
        let file_name = format!("{}.pem", Uuid::new_v4());
        let path = org_dir.join(&file_name);
        {
            let mut file = std::fs::File::create(&path).expect("temp pem");
            file.write_all(private.as_bytes()).expect("write pem");
        }
        let key_ref = format!("{org_id}/{file_name}");
        let key = sample_key(org_id, SigningProviderType::PemFile, &key_ref, public);
        DefaultOrganizationSignerProbe::new(pem_root.clone())
            .challenge(&key)
            .await
            .expect("pem challenge");
        let _ = std::fs::remove_dir_all(&pem_root);
    }

    #[tokio::test]
    async fn pem_challenge_missing_file_fails() {
        let public = include_str!("../../../config/keys/test-platform.pub");
        let missing =
            std::env::temp_dir().join(format!("missing-org-signer-{}.pem", Uuid::new_v4()));
        let org_id = Uuid::new_v4();
        let key = sample_key(
            org_id,
            SigningProviderType::PemFile,
            missing.to_str().expect("utf8 path"),
            public,
        );
        let err = probe().challenge(&key).await.expect_err("missing file");
        assert!(matches!(err, DomainError::ExternalServiceError { .. }));
    }

    #[tokio::test]
    async fn pem_challenge_rejects_parent_dir_traversal() {
        let public = include_str!("../../../config/keys/test-platform.pub");
        let org_id = Uuid::new_v4();
        let key = sample_key(
            org_id,
            SigningProviderType::PemFile,
            "../../../windows/system.ini",
            public,
        );
        let err = probe().challenge(&key).await.expect_err("traversal");
        assert!(matches!(err, DomainError::AuthorizationError(_)));
    }

    #[tokio::test]
    async fn pem_challenge_rejects_absolute_outside_root() {
        let public = include_str!("../../../config/keys/test-platform.pub");
        let org_id = Uuid::new_v4();
        let key = sample_key(org_id, SigningProviderType::PemFile, "/etc/passwd", public);
        let err = probe()
            .challenge(&key)
            .await
            .expect_err("absolute outside root");
        assert!(matches!(
            err,
            DomainError::AuthorizationError(_) | DomainError::ExternalServiceError { .. }
        ));
    }

    #[tokio::test]
    async fn transit_challenge_without_url_fails_closed() {
        let org_id = Uuid::new_v4();
        let key = sample_key(
            org_id,
            SigningProviderType::OpenBaoTransit,
            "org-acme-key",
            include_str!("../../../config/keys/test-platform.pub"),
        );
        let err = probe().challenge(&key).await.expect_err("no transit url");
        assert!(matches!(err, DomainError::ExternalServiceError { .. }));
    }

    #[tokio::test]
    async fn cloud_byokms_is_unsupported() {
        let org_id = Uuid::new_v4();
        let key = sample_key(
            org_id,
            SigningProviderType::AwsKms,
            "arn:aws:kms:…",
            "unused",
        );
        let err = probe().challenge(&key).await.expect_err("cloud");
        assert!(matches!(err, DomainError::ProviderNotSupported(_)));
    }

    #[tokio::test]
    async fn pem_challenge_rejects_foreign_organization_path() {
        let private = include_str!("../../../config/keys/test-platform.pem");
        let public = include_str!("../../../config/keys/test-platform.pub");
        let org_a = Uuid::new_v4();
        let org_b = Uuid::new_v4();
        let pem_root = std::env::temp_dir().join(format!("aiforall-probe-iso-{}", Uuid::new_v4()));
        let org_b_dir = pem_root.join(org_b.to_string());
        std::fs::create_dir_all(&org_b_dir).expect("foreign org dir");
        let file_name = format!("{}.pem", Uuid::new_v4());
        std::fs::write(org_b_dir.join(&file_name), private.as_bytes()).expect("write foreign pem");
        let key_ref = format!("{org_b}/{file_name}");
        let key = sample_key(org_a, SigningProviderType::PemFile, &key_ref, public);
        let err = DefaultOrganizationSignerProbe::new(pem_root.clone())
            .challenge(&key)
            .await
            .expect_err("foreign org pem");
        assert!(matches!(err, DomainError::AuthorizationError(_)));
        let _ = std::fs::remove_dir_all(&pem_root);
    }

    #[tokio::test]
    async fn pem_challenge_rejects_absolute_foreign_org() {
        let private = include_str!("../../../config/keys/test-platform.pem");
        let public = include_str!("../../../config/keys/test-platform.pub");
        let org_a = Uuid::new_v4();
        let org_b = Uuid::new_v4();
        let pem_root = std::env::temp_dir().join(format!("aiforall-probe-abs-{}", Uuid::new_v4()));
        let org_b_dir = pem_root.join(org_b.to_string());
        std::fs::create_dir_all(&org_b_dir).expect("foreign org dir");
        let file_name = format!("{}.pem", Uuid::new_v4());
        std::fs::write(org_b_dir.join(&file_name), private.as_bytes()).expect("write foreign pem");
        let key_ref = org_b_dir.join(&file_name).to_string_lossy().into_owned();
        let key = sample_key(org_a, SigningProviderType::PemFile, &key_ref, public);
        let err = DefaultOrganizationSignerProbe::new(pem_root.clone())
            .challenge(&key)
            .await
            .expect_err("absolute foreign org pem");
        assert!(matches!(err, DomainError::AuthorizationError(_)));
        let _ = std::fs::remove_dir_all(&pem_root);
    }

    #[tokio::test]
    async fn pem_challenge_rejects_platform_pem_at_root() {
        let private = include_str!("../../../config/keys/test-platform.pem");
        let public = include_str!("../../../config/keys/test-platform.pub");
        let org_id = Uuid::new_v4();
        let pem_root = std::env::temp_dir().join(format!("aiforall-probe-plat-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&pem_root).expect("pem root");
        std::fs::write(pem_root.join("test-platform.pem"), private.as_bytes())
            .expect("write platform pem");
        let key = sample_key(
            org_id,
            SigningProviderType::PemFile,
            "test-platform.pem",
            public,
        );
        let err = DefaultOrganizationSignerProbe::new(pem_root.clone())
            .challenge(&key)
            .await
            .expect_err("platform pem");
        assert!(matches!(err, DomainError::AuthorizationError(_)));
        let _ = std::fs::remove_dir_all(&pem_root);
    }

    #[tokio::test]
    async fn pem_challenge_rejects_missing_organization_id() {
        let public = include_str!("../../../config/keys/test-platform.pub");
        let mut key = sample_key(
            Uuid::new_v4(),
            SigningProviderType::PemFile,
            "kid.pem",
            public,
        );
        key.organization_id = None;
        let err = probe()
            .challenge(&key)
            .await
            .expect_err("organization_id required");
        assert!(matches!(err, DomainError::AuthorizationError(_)));
    }
}
