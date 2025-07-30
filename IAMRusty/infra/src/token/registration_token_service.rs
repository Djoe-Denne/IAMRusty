use base64::{engine::general_purpose, Engine as _};
use iam_domain::entity::registration_token::{
    ProviderInfo, RegistrationFlow, RegistrationTokenClaims,
};
use iam_domain::error::DomainError;
use iam_domain::port::service::RegistrationTokenService;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use tracing::{debug, error};
use uuid::Uuid;

use super::JwtAlgorithm;

/// Registration token service implementation using RSA signing
#[derive(Clone)]
pub struct RegistrationTokenServiceImpl {
    algorithm_config: JwtAlgorithm,
}

impl RegistrationTokenServiceImpl {
    /// Create a new registration token service
    /// Only supports RSA algorithms for security reasons
    pub fn new(algorithm_config: JwtAlgorithm) -> Result<Self, DomainError> {
        // Ensure we're using RSA for registration tokens
        if !matches!(algorithm_config, JwtAlgorithm::RS256(_)) {
            return Err(DomainError::AuthorizationError(
                "Registration tokens must use RSA256 algorithm for security".to_string(),
            ));
        }

        Ok(Self { algorithm_config })
    }

    /// Get the encoding key
    fn get_encoding_key(&self) -> Result<EncodingKey, DomainError> {
        match &self.algorithm_config {
            JwtAlgorithm::RS256(key_pair) => {
                EncodingKey::from_rsa_pem(key_pair.private_key.as_bytes()).map_err(|e| {
                    error!("Failed to create RSA encoding key: {}", e);
                    DomainError::AuthorizationError(format!("Invalid private key: {}", e))
                })
            }
            JwtAlgorithm::HS256(_) => Err(DomainError::AuthorizationError(
                "Registration tokens must use RSA256 algorithm".to_string(),
            )),
        }
    }

    /// Get the decoding key
    fn get_decoding_key(&self) -> Result<DecodingKey, DomainError> {
        match &self.algorithm_config {
            JwtAlgorithm::RS256(key_pair) => {
                DecodingKey::from_rsa_pem(key_pair.public_key.as_bytes()).map_err(|e| {
                    error!("Failed to create RSA decoding key: {}", e);
                    DomainError::InvalidToken
                })
            }
            JwtAlgorithm::HS256(_) => Err(DomainError::AuthorizationError(
                "Registration tokens must use RSA256 algorithm".to_string(),
            )),
        }
    }

    /// Get the key ID (for RSA)
    fn get_key_id(&self) -> Option<String> {
        match &self.algorithm_config {
            JwtAlgorithm::RS256(key_pair) => Some(key_pair.kid.clone()),
            JwtAlgorithm::HS256(_) => None,
        }
    }

    /// Decode JWT payload without signature verification (for expiration pre-check)
    fn decode_payload_without_verification(&self, token: &str) -> Result<String, DomainError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(DomainError::InvalidToken);
        }

        // Decode the payload (second part)
        let payload_bytes = general_purpose::URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|_| DomainError::InvalidToken)?;

        String::from_utf8(payload_bytes).map_err(|_| DomainError::InvalidToken)
    }
}

impl RegistrationTokenService for RegistrationTokenServiceImpl {
    fn generate_registration_token(
        &self,
        user_id: Uuid,
        email: String,
    ) -> Result<String, DomainError> {
        debug!(
            "Generating email/password registration token for user: {} with email: {}",
            user_id, email
        );

        let claims = RegistrationTokenClaims::new(user_id, email);

        let mut header = Header {
            alg: Algorithm::RS256,
            ..Default::default()
        };

        // Set key ID for RSA keys
        if let Some(kid) = self.get_key_id() {
            header.kid = Some(kid);
        }

        let encoding_key = self.get_encoding_key()?;

        jsonwebtoken::encode(&header, &claims, &encoding_key).map_err(|e| {
            error!("Failed to encode registration JWT: {}", e);
            DomainError::AuthorizationError(format!("Registration token encoding failed: {}", e))
        })
    }

    fn generate_oauth_registration_token(
        &self,
        user_id: Uuid,
        email: String,
        provider_info: ProviderInfo,
    ) -> Result<String, DomainError> {
        debug!(
            "Generating OAuth registration token for user: {} with email: {} and provider info",
            user_id, email
        );

        let mut claims = RegistrationTokenClaims::new(user_id, email);
        claims.flow = RegistrationFlow::OAuth;
        claims.provider_info = Some(provider_info);

        let mut header = Header {
            alg: Algorithm::RS256,
            ..Default::default()
        };

        // Set key ID for RSA keys
        if let Some(kid) = self.get_key_id() {
            header.kid = Some(kid);
        }

        let encoding_key = self.get_encoding_key()?;

        jsonwebtoken::encode(&header, &claims, &encoding_key).map_err(|e| {
            error!("Failed to encode OAuth registration JWT: {}", e);
            DomainError::AuthorizationError(format!(
                "OAuth registration token encoding failed: {}",
                e
            ))
        })
    }

    fn validate_registration_token(
        &self,
        token: &str,
    ) -> Result<RegistrationTokenClaims, DomainError> {
        debug!("Validating registration token");

        // First, try to decode the payload without signature verification to check expiration
        // This allows us to prioritize expiration errors over signature errors
        if let Ok(payload_str) = self.decode_payload_without_verification(token) {
            if let Ok(claims) = serde_json::from_str::<RegistrationTokenClaims>(&payload_str) {
                if claims.is_expired() {
                    debug!("Registration token expired (pre-check)");
                    return Err(DomainError::TokenExpired);
                }
            }
        }

        let decoding_key = self.get_decoding_key()?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_required_spec_claims(&["sub", "user_id", "email", "exp", "iat", "jti"]);
        validation.set_audience(&["registration"]); // Optional: restrict audience

        let token_data =
            jsonwebtoken::decode::<RegistrationTokenClaims>(token, &decoding_key, &validation)
                .map_err(|e| match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                        debug!("Registration token expired");
                        DomainError::TokenExpired
                    }
                    jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                        debug!("Registration token has invalid signature");
                        DomainError::InvalidToken
                    }
                    _ => {
                        error!("Registration token validation error: {}", e);
                        DomainError::InvalidToken
                    }
                })?;

        let claims = token_data.claims;

        // Validate that this is indeed a registration token
        if !claims.is_registration_token() {
            error!("Token is not a registration token, sub: {}", claims.sub);
            return Err(DomainError::InvalidToken);
        }

        // Additional validation: check if token is expired (redundant with JWT lib but explicit)
        if claims.is_expired() {
            debug!("Registration token expired (explicit check)");
            return Err(DomainError::TokenExpired);
        }

        Ok(claims)
    }

    fn is_registration_token_valid(&self, token: &str) -> bool {
        self.validate_registration_token(token).is_ok()
    }
}
