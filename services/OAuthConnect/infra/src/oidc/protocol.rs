//! Standard protocol generics: the sole additional private claim is `NumericDate` `nbf`.

use idp_connect_contract::FederatedOAuthError;
use oauth_connect_configuration::TokenAuthMethod;
use openidconnect::{
    core::{
        CoreAuthDisplay, CoreAuthPrompt, CoreClientAuthMethod, CoreErrorResponseType,
        CoreGenderClaim, CoreGrantType, CoreJsonWebKey, CoreJsonWebKeySet,
        CoreJweContentEncryptionAlgorithm, CoreJwsSigningAlgorithm, CoreProviderMetadata,
        CoreResponseType, CoreRevocableToken, CoreRevocationErrorResponse,
        CoreTokenIntrospectionResponse, CoreTokenType,
    },
    AdditionalClaims, AuthType, Client, ClientId, ClientSecret, EmptyExtraTokenFields,
    EndpointMaybeSet, EndpointNotSet, EndpointSet, IdToken, IdTokenFields, StandardErrorResponse,
    StandardTokenResponse,
};
use serde::{Deserialize, Serialize};

use super::{callback::BoundHttp, Instance};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct TemporalClaims {
    #[serde(default)]
    pub(super) nbf: Option<i64>,
}
impl AdditionalClaims for TemporalClaims {}

type Fields = IdTokenFields<
    TemporalClaims,
    EmptyExtraTokenFields,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJwsSigningAlgorithm,
>;
pub(super) type Reply = StandardTokenResponse<Fields, CoreTokenType>;
pub(super) type IdentityToken = IdToken<
    TemporalClaims,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJwsSigningAlgorithm,
>;
pub(super) type ProtocolClient = Client<
    TemporalClaims,
    CoreAuthDisplay,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJsonWebKey,
    CoreAuthPrompt,
    StandardErrorResponse<CoreErrorResponseType>,
    Reply,
    CoreTokenIntrospectionResponse,
    CoreRevocableToken,
    CoreRevocationErrorResponse,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

impl Instance {
    pub(super) fn protocol(&self, keys: &CoreJsonWebKeySet) -> ProtocolClient {
        // The standard client copies its JWKS; rebuild from this exact snapshot
        // for BOTH ID-token verification and signed UserInfo after refresh.
        ProtocolClient::from_provider_metadata(
            self.metadata.clone().set_jwks(keys.clone()),
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(
                self.client_secret.expose_secret().to_owned(),
            )),
        )
        .set_auth_type(match self.auth {
            TokenAuthMethod::ClientSecretBasic => AuthType::BasicAuth,
            TokenAuthMethod::ClientSecretPost => AuthType::RequestBody,
        })
    }
}

pub(super) fn validate_metadata(
    metadata: &CoreProviderMetadata,
    http: &BoundHttp,
    auth: TokenAuthMethod,
) -> Result<(), FederatedOAuthError> {
    let error = FederatedOAuthError::ExchangeCode;
    http.guard_endpoints(metadata)?;
    let expected = match auth {
        TokenAuthMethod::ClientSecretBasic => CoreClientAuthMethod::ClientSecretBasic,
        TokenAuthMethod::ClientSecretPost => CoreClientAuthMethod::ClientSecretPost,
    };
    let supported = metadata.token_endpoint_auth_methods_supported().map_or(
        expected == CoreClientAuthMethod::ClientSecretBasic,
        |methods| methods.contains(&expected),
    );
    if !supported
        || metadata.jwks().keys().is_empty()
        || !metadata
            .id_token_signing_alg_values_supported()
            .contains(&CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256)
        || !metadata
            .response_types_supported()
            .iter()
            .any(|types| types.as_slice() == [CoreResponseType::Code])
        || metadata
            .grant_types_supported()
            .is_some_and(|grants| !grants.contains(&CoreGrantType::AuthorizationCode))
    {
        return Err(error);
    }
    Ok(())
}
