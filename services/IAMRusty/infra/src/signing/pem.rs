//! Local PEM-file SigningProvider (dev / simple deployments).

use async_trait::async_trait;
use iam_domain::error::DomainError;
use iam_domain::port::{SigningCapabilities, SigningProvider};
use rsa::pkcs1v15::Pkcs1v15Sign;
use rsa::pkcs8::DecodePrivateKey;
use rsa::{traits::PublicKeyParts, RsaPrivateKey};
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
    /// Returns [`DomainError`] if either PEM is invalid or their normalized n/e
    /// do not match. Setup must do this before registering Active or starting tasks.
    pub fn new(
        private_key_pem: &str,
        public_key_pem: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let error = || DomainError::AuthorizationError("invalid or mismatched RSA key pair".into());
        let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_pem).map_err(|_| error())?;
        private_key.validate().map_err(|_| error())?;
        let public_key_pem = public_key_pem.into();
        let public = iam_domain::entity::signing_key::parse_signing_public_key(&public_key_pem)
            .map_err(|_| error())?;
        let derived = private_key.to_public_key();
        if derived.n() != public.n() || derived.e() != public.e() {
            return Err(error());
        }
        Ok(Self {
            private_key,
            public_key_pem,
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
    use rand::{rngs::StdRng, SeedableRng};
    use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
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

    #[test]
    fn pem_constructor_rejects_two_valid_but_mismatched_pairs_before_any_bootstrap() {
        // Deliberately non-secret deterministic test pair B, distinct from fixture A.
        let other = RsaPrivateKey::new(&mut StdRng::seed_from_u64(1304), 2048).unwrap();
        let private_b = other.to_pkcs8_pem(LineEnding::LF).unwrap();
        let public_b = other
            .to_public_key()
            .to_public_key_pem(LineEnding::LF)
            .unwrap();
        let private_a = include_str!("../../../config/keys/test-platform.pem");
        let public_a = include_str!("../../../config/keys/test-platform.pub");
        assert!(PemSigningProvider::new(private_b.as_str(), public_b.clone()).is_ok());
        assert!(PemSigningProvider::new(private_a, public_a).is_ok());
        assert!(PemSigningProvider::new(private_a, public_a.replace("\n", "\r\n")).is_ok());
        for (private, public) in [
            (private_b.as_str(), public_a),
            (private_a, public_b.as_str()),
        ] {
            let error = PemSigningProvider::new(private, public)
                .err()
                .expect("pair mismatch");
            assert!(!format!("{error:?} {error}").contains("BEGIN"));
        }
        for (private, public) in [
            ("invalid-private-payload", public_a),
            (private_a, "invalid-public-payload"),
        ] {
            let error = PemSigningProvider::new(private, public)
                .err()
                .expect("invalid PEM");
            assert!(!format!("{error:?} {error}").contains("payload"));
        }
    }
}
