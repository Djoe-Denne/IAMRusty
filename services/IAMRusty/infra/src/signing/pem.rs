//! Local PEM-file SigningProvider (dev / simple deployments).

use async_trait::async_trait;
use iam_domain::error::DomainError;
use iam_domain::port::{SigningCapabilities, SigningProvider};
use rsa::pkcs1v15::Pkcs1v15Sign;
use rsa::pkcs8::DecodePrivateKey;
use rsa::RsaPrivateKey;
use sha2::Sha256;

/// Signs digests with an in-memory RSA private key (PKCS#8 PEM).
#[derive(Clone)]
pub struct PemSigningProvider {
    private_key: RsaPrivateKey,
    public_key_pem: String,
}

impl PemSigningProvider {
    /// Build from PKCS#8 private PEM and SPKI public PEM.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the private key PEM cannot be parsed.
    pub fn new(
        private_key_pem: &str,
        public_key_pem: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_pem).map_err(|e| {
            DomainError::AuthorizationError(format!("invalid RSA private key PEM: {e}"))
        })?;
        Ok(Self {
            private_key,
            public_key_pem: public_key_pem.into(),
        })
    }
}

#[async_trait]
impl SigningProvider for PemSigningProvider {
    async fn sign_digest(&self, digest: &[u8]) -> Result<Vec<u8>, DomainError> {
        let padding = Pkcs1v15Sign::new::<Sha256>();
        self.private_key
            .sign(padding, digest)
            .map_err(|e| DomainError::AuthorizationError(format!("RSA PKCS1v15 sign failed: {e}")))
    }

    async fn public_key(&self) -> Result<String, DomainError> {
        Ok(self.public_key_pem.clone())
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
    use sha2::{Digest, Sha256};

    // Minimal self-check that PEM provider signs a digest (key from rustycog test fixtures).
    #[tokio::test]
    async fn pem_signs_sha256_digest() {
        let private = include_str!("../../../config/keys/test-platform.pem");
        let public = include_str!("../../../config/keys/test-platform.pub");
        let provider = PemSigningProvider::new(private, public).expect("pem");
        let digest = Sha256::digest(b"signing-input");
        let sig = provider.sign_digest(&digest).await.expect("sign");
        assert!(!sig.is_empty());
        assert!(provider.capabilities().sign_digest);
    }
}
