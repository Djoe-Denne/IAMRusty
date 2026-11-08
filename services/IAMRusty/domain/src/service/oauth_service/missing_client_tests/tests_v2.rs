//! G-5/G-8: LOGIN orchestration against the real v2 domain port.
//! Contract §§3/5/8 is the oracle; local writer state is not SQL evidence.

use super::{StubEmails, StubEncoder, StubTokens, StubUser};
use crate::entity::oauth_transaction::{
    BegunOAuthTransaction, ConsumeOAuthTransaction, ConsumedOAuthTransaction, OAuthOperation,
    OAuthTransaction, OAuthTransactionError,
};
use crate::entity::provider::Provider;
use crate::entity::provider_link::ProviderLink;
use crate::entity::token::{JwkSet, TokenClaims, DEFAULT_JWT_AUDIENCE, DEFAULT_JWT_ISSUER};
use crate::entity::user::User;
use crate::entity::user_email::UserEmail;
use crate::error::DomainError;
use crate::port::repository::{
    OAuthTransactionWriteRepository, TokenReadRepository, TokenWriteRepository,
    UserEmailReadRepository, UserEmailWriteRepository, UserReadRepository, UserWriteRepository,
};
use crate::port::service::JwtTokenEncoder;
use crate::service::{OAuthService, TokenService};
use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Duration, Utc};
use idp_connect_contract::{
    AuthorizeAuthenticationRequest, AuthorizeResponse, CompleteAuthorizationRequest,
    FederatedOAuthClient, FederatedOAuthError, ProviderAuthentication, ProviderTokens,
    ProviderUserProfile,
};
use mockall::mock;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use uuid::Uuid;

const REDIRECT: &str = "https://iam.example/callback";
const STATE: &str = "SignedLoginStateSentinel";
const CODE: &str = "LoginAuthorizationCodeSentinel";
const AUTHORIZATION_URL: &str = "https://github.example/authorize?scope=read%3Auser";

// No expectations: ANY repository or JWT invocation fails immediately. These
// are port guards for the pure refusal path, not mocked business use cases.
mock! {
    Users {}
    #[async_trait]
    impl UserReadRepository for Users {
        type Error = std::io::Error;
        async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, std::io::Error>;
        async fn find_by_email(&self, email: &str) -> Result<Option<User>, std::io::Error>;
        async fn find_by_username(&self, username: &str) -> Result<Option<User>, std::io::Error>;
        async fn find_by_provider_user_id(&self, provider: &Provider, id: &str) -> Result<Option<User>, std::io::Error>;
    }
    #[async_trait]
    impl UserWriteRepository for Users {
        type Error = std::io::Error;
        async fn create(&self, user: User) -> Result<User, std::io::Error>;
        async fn update(&self, user: User) -> Result<User, std::io::Error>;
    }
}

mock! {
    Tokens {}
    #[async_trait]
    impl TokenReadRepository for Tokens {
        type Error = std::io::Error;
        async fn get_provider_tokens(&self, id: Uuid, provider: &Provider) -> Result<Option<ProviderTokens>, std::io::Error>;
        async fn get_provider_link(&self, id: Uuid, provider: &Provider) -> Result<Option<ProviderLink>, std::io::Error>;
        async fn get_user_provider_links(&self, id: Uuid) -> Result<Vec<ProviderLink>, std::io::Error>;
    }
    #[async_trait]
    impl TokenWriteRepository for Tokens {
        type Error = std::io::Error;
        async fn save_provider_tokens(&self, id: Uuid, provider: &Provider, provider_user_id: String, tokens: ProviderTokens) -> Result<(), std::io::Error>;
        async fn delete_provider_tokens(&self, id: Uuid, provider: &Provider) -> Result<(), std::io::Error>;
    }
}

mock! {
    Emails {}
    #[async_trait]
    impl UserEmailReadRepository for Emails {
        type Error = std::io::Error;
        async fn find_by_user_id(&self, id: Uuid) -> Result<Vec<UserEmail>, std::io::Error>;
        async fn find_by_id(&self, id: Uuid) -> Result<Option<UserEmail>, std::io::Error>;
        async fn find_by_email(&self, email: &str) -> Result<Option<UserEmail>, std::io::Error>;
        async fn find_primary_by_user_id(&self, id: Uuid) -> Result<Option<UserEmail>, std::io::Error>;
    }
    #[async_trait]
    impl UserEmailWriteRepository for Emails {
        type Error = std::io::Error;
        async fn create(&self, email: UserEmail) -> Result<UserEmail, std::io::Error>;
        async fn update(&self, email: UserEmail) -> Result<UserEmail, std::io::Error>;
        async fn delete(&self, id: Uuid) -> Result<(), std::io::Error>;
        async fn set_as_primary(&self, user_id: Uuid, email_id: Uuid) -> Result<(), std::io::Error>;
    }
}

mock! {
    Encoder {}
    #[async_trait]
    impl JwtTokenEncoder for Encoder {
        async fn encode(&self, claims: &TokenClaims) -> Result<String, DomainError>;
        fn decode(&self, token: &str) -> Result<TokenClaims, DomainError>;
        fn jwks(&self) -> JwkSet;
    }
}

#[derive(Default)]
struct Calls {
    legacy: [usize; 5],
    authorize: Vec<AuthorizeAuthenticationRequest>,
    complete: Vec<CompleteAuthorizationRequest>,
}

struct Spy {
    calls: Mutex<Calls>,
    writer: Option<Arc<Writer>>,
    refuse_complete: bool,
}

impl Spy {
    fn calls(&self) -> MutexGuard<'_, Calls> {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[async_trait]
impl FederatedOAuthClient for Spy {
    async fn authorize(&self, _: &str, _: &str) -> Result<AuthorizeResponse, FederatedOAuthError> {
        self.calls().legacy[0] += 1;
        Err(FederatedOAuthError::Authorize)
    }
    async fn exchange_code(&self, _: &str, _: &str) -> Result<ProviderTokens, FederatedOAuthError> {
        self.calls().legacy[1] += 1;
        Err(FederatedOAuthError::ExchangeCode)
    }
    async fn user_profile(&self, _: &str) -> Result<ProviderUserProfile, FederatedOAuthError> {
        self.calls().legacy[2] += 1;
        Err(FederatedOAuthError::UserProfile)
    }
    async fn authorize_with_pkce(
        &self,
        _: &str,
        _: &str,
        _: Option<&str>,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        self.calls().legacy[3] += 1;
        Err(FederatedOAuthError::Authorize)
    }
    async fn exchange_code_with_pkce(
        &self,
        _: &str,
        _: &str,
        _: Option<&str>,
    ) -> Result<ProviderTokens, FederatedOAuthError> {
        self.calls().legacy[4] += 1;
        Err(FederatedOAuthError::ExchangeCode)
    }
    async fn authorize_authentication(
        &self,
        request: &AuthorizeAuthenticationRequest,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        self.calls().authorize.push(request.clone());
        Ok(AuthorizeResponse {
            authorization_url: AUTHORIZATION_URL.into(),
            scope: "read:user".into(),
        })
    }
    async fn complete_authorization(
        &self,
        request: &CompleteAuthorizationRequest,
    ) -> Result<ProviderAuthentication, FederatedOAuthError> {
        self.writer
            .as_ref()
            .expect("callback writer fixture")
            .assert_purged();
        self.calls().complete.push(request.clone());
        if self.refuse_complete {
            return Err(FederatedOAuthError::ExchangeCode);
        }
        Ok(ProviderAuthentication {
            tokens: ProviderTokens {
                access_token: "LoginAccessTokenSentinel".into(),
                refresh_token: None,
                expires_in: Some(300),
            },
            profile: ProviderUserProfile {
                id: "github-user-42".into(),
                username: "iam-login-user".into(),
                email: None,
                avatar_url: None,
                email_verified: false,
            },
        })
    }
}

fn clients(spy: &Arc<Spy>) -> Arc<HashMap<Provider, Arc<dyn FederatedOAuthClient>>> {
    let client: Arc<dyn FederatedOAuthClient> = spy.clone();
    Arc::new(HashMap::from([(
        "github".parse().expect("known provider"),
        client,
    )]))
}

fn token_service(encoder: Arc<dyn JwtTokenEncoder>) -> TokenService {
    TokenService::new(
        encoder,
        Duration::hours(1),
        DEFAULT_JWT_ISSUER.into(),
        DEFAULT_JWT_AUDIENCE.into(),
    )
}

fn begun(pkce: bool) -> BegunOAuthTransaction {
    let verifier = URL_SAFE_NO_PAD.encode([17; 32]);
    BegunOAuthTransaction {
        authentication_nonce: URL_SAFE_NO_PAD.encode([107; 32]),
        code_challenge: pkce.then(|| URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))),
        code_challenge_method: pkce.then(|| "S256".into()),
    }
}

// Only fixture storage can expose secrets. A real capability is minted via the
// production consume guard, with correct marker/bindings and a canonical nonce.
struct Writer(Mutex<OAuthTransaction>);

impl Writer {
    fn row(&self) -> MutexGuard<'_, OAuthTransaction> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
    fn assert_purged(&self) {
        let row = self.row();
        assert!(row.consumed_at.is_some(), "G-5: writer already consumed");
        assert!(
            row.authentication_nonce.is_none(),
            "G-5: nonce already purged"
        );
        assert!(row.pkce_verifier.is_none(), "G-5: verifier already purged");
    }
}

#[async_trait]
impl OAuthTransactionWriteRepository for Writer {
    async fn create(&self, _: &OAuthTransaction) -> Result<(), OAuthTransactionError> {
        panic!("callback-only writer fixture")
    }
    async fn purge_expired(&self, _: u32) -> Result<u64, OAuthTransactionError> {
        panic!("cleanup outside orchestration scenario")
    }
    async fn consume(
        &self,
        input: &ConsumeOAuthTransaction,
    ) -> Result<Option<OAuthTransaction>, OAuthTransactionError> {
        let mut row = self.row();
        if row.consumed_at.is_some() || !row.matches(input) {
            return Ok(None);
        }
        let now = Utc::now().timestamp();
        let original = OAuthTransaction {
            id: row.id,
            nonce_hash: row.nonce_hash,
            state_hash: row.state_hash,
            browser_nonce_hash: row.browser_nonce_hash,
            provider: row.provider.clone(),
            operation: row.operation.clone(),
            redirect_uri: row.redirect_uri.clone(),
            expires_at: row.expires_at,
            consumed_at: Some(now),
            pkce_verifier: row.pkce_verifier.take(),
            authentication_nonce: row.authentication_nonce.take(),
        };
        row.consumed_at = Some(now);
        Ok(Some(original))
    }
}

async fn consumed_login() -> (Arc<Writer>, ConsumedOAuthTransaction) {
    let input = ConsumeOAuthTransaction {
        nonce_hash: [1; 32],
        state_hash: [2; 32],
        browser_nonce_hash: [3; 32],
        provider: "github".parse().expect("known provider"),
        operation: OAuthOperation::Login,
        redirect_uri: REDIRECT.into(),
        expires_at: Utc::now().timestamp() + 300,
    };
    let writer = Arc::new(Writer(Mutex::new(OAuthTransaction {
        id: Uuid::from_u128(42),
        nonce_hash: input.nonce_hash,
        state_hash: input.state_hash,
        browser_nonce_hash: input.browser_nonce_hash,
        provider: input.provider.clone(),
        operation: input.operation.clone(),
        redirect_uri: input.redirect_uri.clone(),
        expires_at: input.expires_at,
        consumed_at: None,
        pkce_verifier: Some(URL_SAFE_NO_PAD.encode([17; 32])),
        authentication_nonce: Some(URL_SAFE_NO_PAD.encode([107; 32])),
    })));
    let capability = ConsumedOAuthTransaction::consume(writer.as_ref(), input)
        .await
        .expect("valid canonical committed fixture must yield capability");
    writer.assert_purged();
    (writer, capability)
}

// Obligation G-8 / §3: forward BEGIN's nonce and PKCE pair through v2 unchanged.
// Scenario: LOGIN START with a configured client and valid context, ±PKCE.
// Oracle: one exact authorize DTO, zero v1/complete, exact returned vendor URL.
// Mutation caught: existing authorize_with_pkce v1 or reconstructed nonce/challenge.
#[rstest::rstest]
#[case::pkce(true)]
#[case::no_pkce(false)]
#[tokio::test]
async fn login_start_forwards_begin_context_to_v2_authorize(#[case] pkce: bool) {
    let spy = Arc::new(Spy {
        calls: Mutex::new(Calls::default()),
        writer: None,
        refuse_complete: false,
    });
    let service = OAuthService::new(
        StubUser,
        StubTokens,
        StubEmails,
        token_service(Arc::new(StubEncoder)),
        clients(&spy),
    );
    let begun = begun(pkce);
    let provider = "github".parse().expect("known provider");
    let result = service
        .generate_authorize_url(&provider, REDIRECT, STATE, &begun)
        .await;
    let calls = spy.calls();
    assert_eq!(calls.authorize.len(), 1, "G-8: LOGIN START uses v2 once");
    assert_eq!(calls.legacy, [0; 5]);
    assert!(calls.complete.is_empty());
    let request = &calls.authorize[0];
    assert!(request.redirect_uri == REDIRECT);
    assert!(request.state == STATE);
    assert!(
        request.nonce == begun.authentication_nonce,
        "G-8: exact BEGIN nonce"
    );
    assert!(
        request.code_challenge == begun.code_challenge,
        "G-8: exact BEGIN challenge"
    );
    assert!(request.code_challenge_method == begun.code_challenge_method);
    assert!(matches!(result, Ok(ref url) if url == AUTHORIZATION_URL));
}

// Obligations G-5/G-8 and existing LOGIN email guard (§5), not new email policy.
// Scenario: real consumed LOGIN; complete refuses, or succeeds with email absent.
// Oracle: one exact complete, zero legacy, generic respective error, and strict
// repository/JWT mocks have ZERO calls. Stored secrets remain consumed/purged.
// Mutation caught: existing split v1 flow, lookup before email, effects after refusal.
#[rstest::rstest]
#[case::complete_refused(true)]
#[case::email_absent(false)]
#[tokio::test]
async fn login_complete_refusal_or_missing_email_precedes_all_business_io(
    #[case] refuse_complete: bool,
) {
    let (writer, capability) = consumed_login().await;
    let spy = Arc::new(Spy {
        calls: Mutex::new(Calls::default()),
        writer: Some(writer.clone()),
        refuse_complete,
    });
    let service = OAuthService::new(
        MockUsers::new(),
        MockTokens::new(),
        MockEmails::new(),
        token_service(Arc::new(MockEncoder::new())),
        clients(&spy),
    );
    let provider = "github".parse().expect("known provider");
    let result = service
        .process_callback(&provider, CODE, REDIRECT, &capability)
        .await;
    let calls = spy.calls();
    assert_eq!(
        calls.complete.len(),
        1,
        "G-5: LOGIN calls complete v2 exactly once"
    );
    assert_eq!(calls.legacy, [0; 5], "G-5: no fallback or separate profile");
    assert!(calls.authorize.is_empty());
    let request = &calls.complete[0];
    assert!(request.code == CODE);
    assert!(request.redirect_uri == REDIRECT);
    assert!(request.expected_nonce == URL_SAFE_NO_PAD.encode([107; 32]));
    assert!(request.code_verifier.as_deref() == Some(URL_SAFE_NO_PAD.encode([17; 32]).as_str()));
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("G-5: refused completion/missing email must not authenticate"),
    };
    if refuse_complete {
        assert!(matches!(error, DomainError::OAuth2Error(_)));
    } else {
        assert!(matches!(error, DomainError::UserProfileError(_)));
    }
    for secret in [
        CODE.to_string(),
        URL_SAFE_NO_PAD.encode([107; 32]),
        URL_SAFE_NO_PAD.encode([17; 32]),
        "LoginAccessTokenSentinel".into(),
    ] {
        assert!(
            !error.to_string().contains(&secret),
            "G-5: sanitized error Display"
        );
        assert!(
            !format!("{error:?}").contains(&secret),
            "G-5: sanitized error Debug"
        );
    }
    writer.assert_purged();
}
