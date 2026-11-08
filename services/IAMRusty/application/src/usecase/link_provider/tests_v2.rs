//! G-5/G-8: native LINK/RELINK orchestration, contract §§2.1/3/5.
//! No repository expectation is installed: any business lookup/write fails.
//! Local writer consumption/purge is a fixture invariant, NOT SQL evidence.

use super::{
    LinkProviderError, LinkProviderResponse, LinkProviderUseCase, LinkProviderUseCaseImpl,
};
use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use iam_domain::entity::oauth_transaction::{
    BegunOAuthTransaction, ConsumeOAuthTransaction, ConsumedOAuthTransaction, OAuthOperation,
    OAuthTransaction, OAuthTransactionError,
};
use iam_domain::entity::provider::{Provider, ProviderTokens, ProviderUserProfile};
use iam_domain::entity::provider_link::ProviderLink;
use iam_domain::entity::user::User;
use iam_domain::entity::user_email::UserEmail;
use iam_domain::error::DomainError;
use iam_domain::port::repository::{
    OAuthTransactionWriteRepository, TokenReadRepository, TokenWriteRepository,
    UserEmailReadRepository, UserEmailWriteRepository, UserReadRepository, UserWriteRepository,
};
use iam_domain::port::service::{
    AuthorizeAuthenticationRequest, AuthorizeResponse, CompleteAuthorizationRequest,
    FederatedOAuthClient, FederatedOAuthError, ProviderAuthentication,
};
use iam_domain::service::ProviderLinkService;
use mockall::mock;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use uuid::Uuid;

const REDIRECT: &str = "https://iam.example/callback";
const STATE: &str = "SignedProviderLinkStateSentinel";
const CODE: &str = "ProviderLinkAuthorizationCodeSentinel";
const AUTHORIZATION_URL: &str = "https://github.example/authorize?scope=read%3Auser";

// Empty mock expectations are deliberate strict guards, not implementations of
// account/link/email policy. Real business UUID/identity behavior belongs to IT.
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

#[derive(Default)]
struct Calls {
    // Separate overrides also detect calls hidden by legacy trait defaults.
    legacy: [usize; 5],
    authorize: Vec<AuthorizeAuthenticationRequest>,
    complete: Vec<CompleteAuthorizationRequest>,
}

struct Spy {
    calls: Mutex<Calls>,
    writer: Option<Arc<Writer>>,
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
        Err(FederatedOAuthError::ExchangeCode)
    }
}

type UseCase = LinkProviderUseCaseImpl<MockUsers, MockEmails, MockTokens>;

fn use_case(writer: Option<Arc<Writer>>) -> (UseCase, Arc<Spy>) {
    let spy = Arc::new(Spy {
        calls: Mutex::new(Calls::default()),
        writer,
    });
    let client: Arc<dyn FederatedOAuthClient> = spy.clone();
    // BOTH slugs are admitted: wrong-provider tests must reach the binding guard,
    // not accidentally pass because the factory has no client for that provider.
    let clients = Arc::new(HashMap::from([
        ("github".parse().expect("known provider"), client.clone()),
        ("gitlab".parse().expect("known provider"), client),
    ]));
    let service = Arc::new(ProviderLinkService::new(
        Arc::new(MockUsers::new()),
        Arc::new(MockEmails::new()),
        Arc::new(MockTokens::new()),
    ));
    (LinkProviderUseCaseImpl::new(clients, service), spy)
}

const fn target() -> Uuid {
    Uuid::from_u128(7)
}

const fn operation(relink: bool) -> OAuthOperation {
    if relink {
        OAuthOperation::Relink { user_id: target() }
    } else {
        OAuthOperation::Link { user_id: target() }
    }
}

fn begun(pkce: bool) -> BegunOAuthTransaction {
    let verifier = URL_SAFE_NO_PAD.encode([17; 32]);
    BegunOAuthTransaction {
        authentication_nonce: URL_SAFE_NO_PAD.encode([107; 32]),
        code_challenge: pkce.then(|| URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))),
        code_challenge_method: pkce.then(|| "S256".into()),
    }
}

// Test-only storage keeps a consumed, double-purged row after returning originals.
// It deliberately does not claim SQL commit, primary-clock or locking coverage.
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

async fn consumed_context(
    relink: bool,
) -> (
    Arc<Writer>,
    ConsumedOAuthTransaction,
    CompleteAuthorizationRequest,
) {
    let input = ConsumeOAuthTransaction {
        nonce_hash: [1; 32],
        state_hash: [2; 32],
        browser_nonce_hash: [3; 32],
        provider: "github".parse().expect("known provider"),
        operation: operation(relink),
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
    // Snapshot is fixture-only and precedes consume; never extract capability fields.
    let expected = {
        let row = writer.row();
        CompleteAuthorizationRequest {
            code: CODE.into(),
            redirect_uri: row.redirect_uri.clone(),
            expected_nonce: row
                .authentication_nonce
                .clone()
                .expect("fixture canonical nonce"),
            code_verifier: row.pkce_verifier.clone(),
        }
    };
    let capability = ConsumedOAuthTransaction::consume(writer.as_ref(), input)
        .await
        .expect("canonical matching fixture must yield real capability");
    assert!(capability.operation() == &operation(relink));
    assert!(capability.target_user_id() == Some(target()));
    assert!(capability.provider().as_str() == "github");
    assert!(capability.redirect_uri() == REDIRECT);
    writer.assert_purged();
    (writer, capability, expected)
}

async fn callback(
    use_case: &UseCase,
    relink: bool,
    user_id: Uuid,
    provider: Provider,
    redirect: String,
    consumed: ConsumedOAuthTransaction,
) -> Result<LinkProviderResponse, LinkProviderError> {
    if relink {
        use_case
            .relink_provider(user_id, provider, CODE.into(), redirect, consumed)
            .await
    } else {
        use_case
            .link_provider(user_id, provider, CODE.into(), redirect, consumed)
            .await
    }
}

// G-8 / §3 obligation: both START paths forward BEGIN's exact public context.
// Scenario: configured LINK/RELINK START, with or without PKCE.
// Oracle: one v2 authorize, exact nonce/state/redirect/challenge pair and returned
// URL, zero complete/legacy/repository calls. Scope is not returned by this API.
// Mutation caught: existing v1 authorize, nonce reconstruction, PKCE omission.
#[rstest::rstest]
#[case::link_pkce(false, true)]
#[case::link_no_pkce(false, false)]
#[case::relink_pkce(true, true)]
#[case::relink_no_pkce(true, false)]
#[tokio::test]
async fn start_forwards_begin_context_once_to_v2_authorize(
    #[case] relink: bool,
    #[case] pkce: bool,
) {
    let (use_case, spy) = use_case(None);
    let begun = begun(pkce);
    let provider = "github".parse().expect("known provider");
    let result = if relink {
        use_case
            .generate_relink_start_url(provider, REDIRECT.into(), STATE.into(), &begun)
            .await
    } else {
        use_case
            .generate_start_url(provider, REDIRECT.into(), STATE.into(), &begun)
            .await
    };
    let calls = spy.calls();
    assert_eq!(calls.authorize.len(), 1, "G-8: START uses v2 exactly once");
    assert_eq!(calls.legacy, [0; 5], "G-8: no legacy authorize");
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

// G-5 / §5 obligation: complete refusal precedes every business effect.
// Scenario: real consumed LINK/RELINK of the correct target/provider/redirect;
// the configured v2 port refuses, while each v1 port also refuses independently.
// Oracle: exactly one complete with original secrets, zero legacy/re-authorize,
// sanitized domain error and strict repository mocks observing no invocation.
// Mutation caught: existing v1 flow, retry/fallback, lookup or write before refusal.
#[rstest::rstest]
#[case::link(false)]
#[case::relink(true)]
#[tokio::test]
async fn complete_refusal_precedes_every_business_lookup_and_write(#[case] relink: bool) {
    let (writer, capability, expected) = consumed_context(relink).await;
    let (use_case, spy) = use_case(Some(writer.clone()));
    let result = callback(
        &use_case,
        relink,
        target(),
        "github".parse().expect("known provider"),
        REDIRECT.into(),
        capability,
    )
    .await;
    let calls = spy.calls();
    assert_eq!(
        calls.complete.len(),
        1,
        "G-5: callback uses complete v2 once"
    );
    assert_eq!(calls.legacy, [0; 5], "G-5: no fallback or profile call");
    assert!(calls.authorize.is_empty());
    let request = &calls.complete[0];
    assert!(request.code == expected.code);
    assert!(request.redirect_uri == expected.redirect_uri);
    assert!(
        request.expected_nonce == expected.expected_nonce,
        "G-5: original nonce"
    );
    assert!(
        request.code_verifier == expected.code_verifier,
        "G-5: original verifier"
    );
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("G-5: refused completion cannot mutate a provider link"),
    };
    assert!(matches!(
        error,
        LinkProviderError::DomainError(DomainError::OAuth2Error(_))
    ));
    for secret in [
        Some(expected.code.as_str()),
        Some(expected.expected_nonce.as_str()),
        expected.code_verifier.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        assert!(
            !error.to_string().contains(secret),
            "G-5: sanitized Display"
        );
        assert!(
            !format!("{error:?}").contains(secret),
            "G-5: sanitized Debug"
        );
    }
    writer.assert_purged();
}

#[derive(Clone, Copy)]
enum Mismatch {
    Operation,
    Target,
    Provider,
    Redirect,
}

// G-8 / §5 obligation: operation/target/provider/redirect guards precede port I/O.
// Scenario: FIRST mint an otherwise valid capability and admit BOTH providers,
// THEN change exactly one use-case argument, preserving all other preconditions.
// Oracle: InvalidToken and zero v2/v1/repository calls. This is a post-consume
// use-case guard, NOT a browser/state verifier or unconsumed-row preservation test.
// Mutation caught: removal of any of the four guards, including target retargeting.
#[rstest::rstest]
#[case::link_operation(false, Mismatch::Operation)]
#[case::link_target(false, Mismatch::Target)]
#[case::link_provider(false, Mismatch::Provider)]
#[case::link_redirect(false, Mismatch::Redirect)]
#[case::relink_operation(true, Mismatch::Operation)]
#[case::relink_target(true, Mismatch::Target)]
#[case::relink_provider(true, Mismatch::Provider)]
#[case::relink_redirect(true, Mismatch::Redirect)]
#[tokio::test]
async fn bound_context_mismatch_refuses_before_any_client_or_repository(
    #[case] relink: bool,
    #[case] mismatch: Mismatch,
) {
    let (writer, capability, _) = consumed_context(relink).await;
    let (use_case, spy) = use_case(Some(writer.clone()));
    let mut called_relink = relink;
    let mut user_id = target();
    let mut provider = "github".parse().expect("known provider");
    let mut redirect = REDIRECT.to_string();
    match mismatch {
        Mismatch::Operation => called_relink = !relink,
        Mismatch::Target => user_id = Uuid::from_u128(8),
        Mismatch::Provider => provider = "gitlab".parse().expect("known provider"),
        Mismatch::Redirect => redirect.push_str("/other"),
    }
    let result = callback(
        &use_case,
        called_relink,
        user_id,
        provider,
        redirect,
        capability,
    )
    .await;
    assert!(matches!(
        result,
        Err(LinkProviderError::DomainError(DomainError::InvalidToken))
    ));
    let calls = spy.calls();
    assert!(
        calls.complete.is_empty(),
        "G-8: mismatch must precede complete"
    );
    assert!(calls.authorize.is_empty());
    assert_eq!(
        calls.legacy, [0; 5],
        "G-8: mismatch must precede legacy I/O"
    );
    writer.assert_purged();
}
