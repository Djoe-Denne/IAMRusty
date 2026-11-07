//! Signature/standard claims first; only then inspect typed local policy claims.

use chrono::{DateTime, Duration, Utc};
use openidconnect::{
    core::CoreJwsSigningAlgorithm, AccessToken, AccessTokenHash, ClaimsVerificationError, Nonce,
};

use super::{
    profile::Profile,
    protocol::{IdentityToken, ProtocolClient},
};

pub(super) fn validated(
    client: &ProtocolClient,
    token: &IdentityToken,
    access: &AccessToken,
    nonce: &Nonce,
    now: DateTime<Utc>,
    client_id: &str,
) -> Result<Profile, ClaimsVerificationError> {
    let future = now
        .checked_add_signed(Duration::seconds(60))
        .ok_or_else(invalid_claims)?;
    let verifier = client
        .id_token_verifier()
        .set_allowed_algs([CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256])
        .set_other_audience_verifier_fn(|_| false)
        .set_time_fn(move || now)
        .set_issue_time_verifier_fn(move |issued| {
            if issued <= future {
                Ok(())
            } else {
                Err("Invalid OIDC issue time".to_owned())
            }
        });
    let claims = token.claims(&verifier, nonce)?;
    if claims.expiration() <= now
        || claims.issue_time() > claims.expiration()
        || claims
            .authorized_party()
            .is_some_and(|party| party.as_str() != client_id)
        || claims
            .additional_claims()
            .nbf
            .is_some_and(|nbf| nbf > now.timestamp())
    {
        return Err(invalid_claims());
    }
    if let Some(expected) = claims.access_token_hash() {
        let alg = token
            .signing_alg()
            .map_err(ClaimsVerificationError::SignatureVerification)?;
        let key = token
            .signing_key(&verifier)
            .map_err(ClaimsVerificationError::SignatureVerification)?;
        let actual = AccessTokenHash::from_token(access, alg, key).map_err(|_| invalid_claims())?;
        if expected != &actual {
            return Err(invalid_claims());
        }
    }
    Profile::new(
        claims.subject().as_str(),
        claims.preferred_username().map(|value| value.as_str()),
        claims.email().map(|value| value.as_str()),
        claims.email_verified(),
        claims
            .picture()
            .and_then(|localized| localized.get(None))
            .map(|value| value.as_str()),
    )
    .map_err(|_| invalid_claims())
}

fn invalid_claims() -> ClaimsVerificationError {
    ClaimsVerificationError::Other("Invalid OIDC claims".to_owned())
}
