//! G-1/G-2: contract `iam-oauthconnect-v2.md` §§2–4, not SQL evidence.
//! The writer below models a committed consume under a local mutex. Real writer
//! locking, commit failures, migration and network ordering remain integration gaps.

use super::{
    BeginOAuthTransaction, BegunOAuthTransaction, ConsumeOAuthTransaction,
    ConsumedOAuthTransaction, OAuthOperation, OAuthTransaction, OAuthTransactionError,
};
use crate::error::DomainError;
use crate::port::repository::OAuthTransactionWriteRepository;
use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use idp_connect_contract::{
    AuthorizeAuthenticationRequest, AuthorizeResponse, CompleteAuthorizationRequest,
    FederatedOAuthClient, FederatedOAuthError, ProviderAuthentication, ProviderTokens,
    ProviderUserProfile,
};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use uuid::Uuid;

#[derive(Default)]
struct WriterState {
    row: Option<OAuthTransaction>,
    creates: usize,
    consumes: usize,
    committed_consumes: usize,
}

#[derive(Default)]
struct Writer {
    state: Mutex<WriterState>,
    reject_create: bool,
    omit_returned_consumed_marker: bool,
}

// Fixture-only copying: no Clone or secret getter is added to the domain API.
fn copy_record(tx: &OAuthTransaction) -> OAuthTransaction {
    OAuthTransaction {
        id: tx.id,
        nonce_hash: tx.nonce_hash,
        state_hash: tx.state_hash,
        browser_nonce_hash: tx.browser_nonce_hash,
        provider: tx.provider.clone(),
        operation: tx.operation.clone(),
        redirect_uri: tx.redirect_uri.clone(),
        expires_at: tx.expires_at,
        consumed_at: tx.consumed_at,
        pkce_verifier: tx.pkce_verifier.clone(),
        authentication_nonce: tx.authentication_nonce.clone(),
    }
}

impl Writer {
    fn state(&self) -> MutexGuard<'_, WriterState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn snapshot(&self) -> OAuthTransaction {
        copy_record(self.state().row.as_ref().expect("fixture row exists"))
    }

    fn install(&self, input: &BeginOAuthTransaction, nonce: Option<String>) {
        self.state().row = Some(OAuthTransaction {
            id: Uuid::from_u128(42),
            nonce_hash: input.nonce_hash,
            state_hash: input.state_hash,
            browser_nonce_hash: input.browser_nonce_hash,
            provider: input.provider.clone(),
            operation: input.operation.clone(),
            redirect_uri: input.redirect_uri.clone(),
            expires_at: input.expires_at,
            consumed_at: None,
            pkce_verifier: input
                .pkce_required
                .then(|| URL_SAFE_NO_PAD.encode([17; 32])),
            authentication_nonce: nonce,
        });
    }
}

#[async_trait]
impl OAuthTransactionWriteRepository for Writer {
    async fn create(&self, transaction: &OAuthTransaction) -> Result<(), OAuthTransactionError> {
        let mut state = self.state();
        state.creates += 1;
        if self.reject_create {
            return Err(OAuthTransactionError::Storage);
        }
        state.row = Some(copy_record(transaction));
        Ok(())
    }

    async fn consume(
        &self,
        input: &ConsumeOAuthTransaction,
    ) -> Result<Option<OAuthTransaction>, OAuthTransactionError> {
        let mut state = self.state();
        state.consumes += 1;
        let Some(row) = state.row.as_mut() else {
            return Ok(None);
        };
        // Model writer predicates independently of the domain's post-writer guard.
        if row.consumed_at.is_some()
            || row.nonce_hash != input.nonce_hash
            || row.state_hash != input.state_hash
            || row.browser_nonce_hash != input.browser_nonce_hash
            || row.provider != input.provider
            || row.operation != input.operation
            || row.redirect_uri != input.redirect_uri
            || row.expires_at != input.expires_at
            || row.expires_at <= Utc::now().timestamp()
        {
            return Ok(None);
        }
        let now = Utc::now().timestamp();
        let mut original = copy_record(row);
        row.consumed_at = Some(now);
        row.pkce_verifier = None;
        row.authentication_nonce = None;
        original.consumed_at = (!self.omit_returned_consumed_marker).then_some(now);
        state.committed_consumes += 1;
        // The guard drops before return; storage retains a consumed, purged row.
        Ok(Some(original))
    }

    async fn purge_expired(&self, _batch_size: u32) -> Result<u64, OAuthTransactionError> {
        panic!("cleanup is outside these BEGIN/consume scenarios")
    }
}

fn begin_input(operation: &str, pkce_required: bool) -> BeginOAuthTransaction {
    let user_id = Uuid::from_u128(7);
    let operation = match operation {
        "login" => OAuthOperation::Login,
        "link" => OAuthOperation::Link { user_id },
        "relink" => OAuthOperation::Relink { user_id },
        _ => panic!("unknown fixture operation"),
    };
    BeginOAuthTransaction {
        nonce_hash: [1; 32],
        state_hash: [2; 32],
        browser_nonce_hash: [3; 32],
        provider: "github".parse().expect("known provider"),
        operation,
        redirect_uri: "https://iam.example/callback".into(),
        expires_at: Utc::now().timestamp() + 300,
        pkce_required,
    }
}

fn callback(input: &BeginOAuthTransaction) -> ConsumeOAuthTransaction {
    ConsumeOAuthTransaction {
        nonce_hash: input.nonce_hash,
        state_hash: input.state_hash,
        browser_nonce_hash: input.browser_nonce_hash,
        provider: input.provider.clone(),
        operation: input.operation.clone(),
        redirect_uri: input.redirect_uri.clone(),
        expires_at: input.expires_at,
    }
}

fn assert_canonical_nonce(value: &str) {
    assert!(value.len() == 43, "G-1: nonce must contain 43 characters");
    let decoded = URL_SAFE_NO_PAD.decode(value);
    assert!(decoded.is_ok(), "G-1: nonce must decode without padding");
    let decoded = decoded.expect("decoding already checked without exposing input");
    assert!(decoded.len() == 32, "G-1: nonce must encode 32 bytes");
    assert!(
        URL_SAFE_NO_PAD.encode(&decoded) == value,
        "G-1: nonce must round-trip canonically"
    );
}

fn assert_committed_and_purged(writer: &Writer) {
    let stored = writer.snapshot();
    assert!(stored.consumed_at.is_some(), "G-2: stored row is consumed");
    assert!(
        stored.pkce_verifier.is_none(),
        "G-2: stored verifier purged"
    );
    assert!(
        stored.authentication_nonce.is_none(),
        "G-2: stored authentication nonce purged"
    );
    assert_eq!(writer.state().committed_consumes, 1);
}

// G-2 supporting fixture check, NOT a production storage guarantee.
// Scenario: call the writer port directly with an installed, matching row.
// Oracle: original secrets/consumed marker returned, both stored secrets purged.
// Mutation caught: a broken test double returning the already-purged record.
#[tokio::test]
async fn writer_fixture_returns_original_secrets_after_purging_its_stored_row() {
    let writer = Writer::default();
    let input = begin_input("login", true);
    writer.install(&input, Some(URL_SAFE_NO_PAD.encode([107; 32])));
    let before = writer.snapshot();
    let returned = writer
        .consume(&callback(&input))
        .await
        .expect("fixture writer accepts matching input")
        .expect("fixture writer returns original record");
    assert!(returned.consumed_at.is_some());
    assert!(returned.authentication_nonce == before.authentication_nonce);
    assert!(returned.pkce_verifier == before.pkce_verifier);
    assert_committed_and_purged(&writer);
}

// Obligation G-1: BEGIN persists before publishing; PKCE never replaces the nonce.
// Scenario: each operation begins, with/without PKCE, on an empty writer.
// Oracle: exact stored/published canonical nonce; challenge hashes stored verifier.
// Mutation caught: scaffold None/empty nonce, no-PKCE bypass, verifier as challenge.
#[rstest::rstest]
#[case::login_pkce("login", true)]
#[case::login_no_pkce("login", false)]
#[case::link_pkce("link", true)]
#[case::link_no_pkce("link", false)]
#[case::relink_pkce("relink", true)]
#[case::relink_no_pkce("relink", false)]
#[tokio::test]
async fn begin_persists_canonical_nonce_before_returning_public_context(
    #[case] operation: &str,
    #[case] pkce: bool,
) {
    let writer = Writer::default();
    let input = begin_input(operation, pkce);
    let begun = OAuthTransaction::begin(&writer, input.clone())
        .await
        .expect("valid BEGIN must succeed");
    let stored = writer.snapshot();
    assert_eq!(writer.state().creates, 1);
    assert!(stored.consumed_at.is_none());
    assert!(stored.operation == input.operation);
    assert!(stored.expires_at == input.expires_at);
    assert_canonical_nonce(&begun.authentication_nonce);
    assert!(
        stored.authentication_nonce.as_deref() == Some(begun.authentication_nonce.as_str()),
        "G-1: writer must already hold exactly the published nonce"
    );
    // Fixed bindings exclude accidental reuse of these fixture authorities. This
    // is not an entropy/independent-OsRng proof; source review is also required.
    for binding in [input.nonce_hash, input.state_hash, input.browser_nonce_hash] {
        assert!(
            begun.authentication_nonce != URL_SAFE_NO_PAD.encode(binding),
            "G-1: authentication nonce must not be a binding hash"
        );
    }
    if pkce {
        let verifier = stored
            .pkce_verifier
            .as_deref()
            .expect("PKCE verifier stored");
        let expected = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        assert!(begun.code_challenge.as_deref() == Some(expected.as_str()));
        assert!(begun.code_challenge_method.as_deref() == Some("S256"));
        assert!(
            begun.authentication_nonce != verifier,
            "G-1: nonce is not verifier"
        );
        assert!(
            begun.authentication_nonce != expected,
            "G-1: nonce is not challenge"
        );
        assert!(
            !format!("{begun:?}").contains(verifier),
            "G-1: Debug hides verifier"
        );
    } else {
        assert!(stored.pkce_verifier.is_none());
        assert!(begun.code_challenge.is_none());
        assert!(begun.code_challenge_method.is_none());
    }
}

// G-1 scenario: create fails after valid admission. Oracle: Storage, no context/row.
// Mutation caught: swallowing writer failure and returning successful BEGIN.
#[tokio::test]
async fn begin_create_failure_returns_storage_without_success_context() {
    let writer = Writer {
        reject_create: true,
        ..Writer::default()
    };
    let result = OAuthTransaction::begin(&writer, begin_input("login", true)).await;
    assert!(matches!(result, Err(OAuthTransactionError::Storage)));
    assert_eq!(writer.state().creates, 1);
    assert!(writer.state().row.is_none());
}

// G-1 scenario: invalid TTL or empty redirect, then a comfortably valid long TTL.
// Oracle: invalid input never reaches create; 540s succeeds without clock equality.
// Mutation caught: accepting expired/>600s TTL or rejecting every long-lived BEGIN.
#[tokio::test]
async fn begin_rejects_invalid_admission_before_writer_but_allows_ttl_margin() {
    for (ttl, redirect) in [
        (-60, "https://iam.example/callback"),
        (660, "https://iam.example/callback"),
        (300, ""),
    ] {
        let writer = Writer::default();
        let mut input = begin_input("login", true);
        input.expires_at = Utc::now().timestamp() + ttl;
        input.redirect_uri = redirect.into();
        assert!(matches!(
            OAuthTransaction::begin(&writer, input).await,
            Err(OAuthTransactionError::InvalidTransaction)
        ));
        assert_eq!(writer.state().creates, 0);
        assert!(writer.state().row.is_none());
    }
    let writer = Writer::default();
    let mut valid = begin_input("login", false);
    valid.expires_at = Utc::now().timestamp() + 540;
    let expected_expiry = valid.expires_at;
    let result = OAuthTransaction::begin(&writer, valid).await;
    assert!(result.is_ok(), "G-1: TTL safely within 600s is admitted");
    assert_eq!(writer.state().creates, 1);
    assert_eq!(writer.snapshot().expires_at, expected_expiry);
}

// G-2 scenario: fixture installs a valid durable nonce, independent of BEGIN.
// Oracle: exactly one capability across competing consumes; replay does not mutate.
// Mutation caught: accepting None from writer on replay. Mutex is NOT a DB lock proof.
#[rstest::rstest]
#[case::login("login")]
#[case::link("link")]
#[case::relink("relink")]
#[tokio::test]
async fn canonical_nonce_allows_one_capability_and_replay_is_a_noop(#[case] operation: &str) {
    let writer = Writer::default();
    let input = begin_input(operation, true);
    writer.install(&input, Some(URL_SAFE_NO_PAD.encode([107; 32])));
    let expected_verifier = writer.snapshot().pkce_verifier;
    let callback = callback(&input);
    let (first, second) = tokio::join!(
        ConsumedOAuthTransaction::consume(&writer, callback.clone()),
        ConsumedOAuthTransaction::consume(&writer, callback.clone())
    );
    let capability = match (first, second) {
        (Ok(capability), Err(OAuthTransactionError::InvalidTransaction))
        | (Err(OAuthTransactionError::InvalidTransaction), Ok(capability)) => capability,
        _ => panic!("G-2: exactly one capability and one invalid replay required"),
    };
    assert!(capability.operation() == &input.operation);
    assert!(capability.redirect_uri() == input.redirect_uri);
    assert!(
        capability.pkce_verifier() == expected_verifier.as_deref(),
        "G-2: original verifier returned"
    );
    // Nonce forwarding from the private capability awaits the complete helper API.
    assert_committed_and_purged(&writer);
    let consumed_at = writer.snapshot().consumed_at;
    assert!(matches!(
        ConsumedOAuthTransaction::consume(&writer, callback).await,
        Err(OAuthTransactionError::InvalidTransaction)
    ));
    assert_committed_and_purged(&writer);
    assert!(writer.snapshot().consumed_at == consumed_at);
    assert_eq!(writer.state().consumes, 3);
}

// G-2 scenario: another browser presents an otherwise valid callback, then owner.
// Oracle: first refuses with both secrets intact, owner still consumes successfully.
// Mutation caught: a mismatched callback burning the original nonce.
#[tokio::test]
async fn wrong_browser_preserves_nonce_until_the_bound_callback_consumes() {
    let writer = Writer::default();
    let input = begin_input("link", true);
    writer.install(&input, Some(URL_SAFE_NO_PAD.encode([107; 32])));
    let before = writer.snapshot();
    let mut wrong = callback(&input);
    wrong.browser_nonce_hash = [99; 32];
    assert!(matches!(
        ConsumedOAuthTransaction::consume(&writer, wrong).await,
        Err(OAuthTransactionError::InvalidTransaction)
    ));
    let after = writer.snapshot();
    assert!(after.consumed_at.is_none());
    assert!(
        after.authentication_nonce == before.authentication_nonce,
        "G-2: nonce unchanged on mismatch"
    );
    assert!(
        after.pkce_verifier == before.pkce_verifier,
        "G-2: verifier unchanged on mismatch"
    );
    assert_eq!(writer.state().committed_consumes, 0);
    let result = ConsumedOAuthTransaction::consume(&writer, callback(&input)).await;
    assert!(result.is_ok(), "G-2: owner must still obtain capability");
    assert_committed_and_purged(&writer);
}

// G-2 scenario: faulty adapter returns an original record without consumed marker.
// Oracle: domain refuses despite valid bindings/nonce. This is a pure guard witness.
// Mutation caught: deleting the existing consumed_at post-writer validation.
#[tokio::test]
async fn valid_nonce_does_not_bypass_missing_writer_consumed_marker() {
    let writer = Writer {
        omit_returned_consumed_marker: true,
        ..Writer::default()
    };
    let input = begin_input("login", true);
    writer.install(&input, Some(URL_SAFE_NO_PAD.encode([107; 32])));
    let result = ConsumedOAuthTransaction::consume(&writer, callback(&input)).await;
    assert!(matches!(
        result,
        Err(OAuthTransactionError::InvalidTransaction)
    ));
    assert_eq!(writer.state().consumes, 1);
    assert_committed_and_purged(&writer);
}

// G-1 scenario: actual public BEGIN type contains a sentinel nonce.
// Oracle: Debug omits the nonce, without dumping any secret on assertion failure.
// Mutation caught: deriving Debug on BegunOAuthTransaction.
#[test]
fn begun_debug_redacts_authentication_nonce_sentinel() {
    let begun = BegunOAuthTransaction {
        authentication_nonce: "AuthenticationNonceSentinel".into(),
        code_challenge: Some("PkceChallengeSentinel".into()),
        code_challenge_method: Some("S256".into()),
    };
    let debug = format!("{begun:?}");
    assert!(
        !debug.contains("AuthenticationNonceSentinel"),
        "G-1: Debug leaked nonce"
    );
}

// G-2 / contract §4 Q-2: keep this expected-failure group last, not weakened.
// Scenario: valid bindings, but legacy/corrupt durable nonce; then repeat callback.
// Oracle AFTER scenario: writer consumed/purged once BEFORE InvalidTransaction,
// no capability, and replay cannot recover. No client exists here: no network API
// is invoked by this test. HTTP zero-v1/complete receipts belong to other authors.
// Mutation caught: scaffold consume accepting any nonce, permissive base64 parsing.
#[rstest::rstest]
#[case::legacy_null(None)]
#[case::empty(Some(String::new()))]
#[case::short_31_bytes(Some(URL_SAFE_NO_PAD.encode([107; 31])))]
#[case::long_33_bytes(Some(URL_SAFE_NO_PAD.encode([107; 33])))]
#[case::illegal_alphabet(Some("!".repeat(43)))]
#[case::padded(Some(format!("{}=", URL_SAFE_NO_PAD.encode([107; 32]))))]
#[case::noncanonical_trailing_bits(Some(format!("{}B", "A".repeat(42))))]
#[tokio::test]
async fn legacy_or_corrupt_nonce_is_consumed_and_purged_before_generic_refusal(
    #[case] nonce: Option<String>,
) {
    let writer = Writer::default();
    let input = begin_input("login", true);
    writer.install(&input, nonce);
    let callback = callback(&input);
    let result = ConsumedOAuthTransaction::consume(&writer, callback.clone()).await;
    // Check fixture's commit/purge precondition before the intended RED oracle.
    assert_committed_and_purged(&writer);
    assert_eq!(writer.state().consumes, 1);
    assert!(
        matches!(result, Err(OAuthTransactionError::InvalidTransaction)),
        "G-2: consumed legacy/corrupt nonce must not grant a capability"
    );
    assert!(matches!(
        ConsumedOAuthTransaction::consume(&writer, callback).await,
        Err(OAuthTransactionError::InvalidTransaction)
    ));
    assert_committed_and_purged(&writer);
    assert_eq!(writer.state().consumes, 2);
}

struct CompletionReceipt {
    request: CompleteAuthorizationRequest,
    saw_consumed_and_purged: bool,
}

#[derive(Default)]
struct CompletionCalls {
    // authorize, exchange, profile, authorize-PKCE, exchange-PKCE: separate counts
    // ensure an override cannot conceal fallback through a default trait method.
    legacy: [usize; 5],
    authorize_v2: usize,
    completions: Vec<CompletionReceipt>,
}

struct CompletionSpy {
    writer: Arc<Writer>,
    calls: Mutex<CompletionCalls>,
    failure: Option<FederatedOAuthError>,
}

impl CompletionSpy {
    fn calls(&self) -> MutexGuard<'_, CompletionCalls> {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

// Port-level canned identity, independent of the helper implementation/request.
// This is not a Connect HTTP response or a cryptographic identity proof.
fn completion_authentication() -> ProviderAuthentication {
    ProviderAuthentication {
        tokens: ProviderTokens {
            access_token: "HelperAccessTokenSentinel".into(),
            refresh_token: Some("HelperRefreshTokenSentinel".into()),
            expires_in: Some(1234),
        },
        profile: ProviderUserProfile {
            id: "github-user-42".into(),
            username: "iam-helper-user".into(),
            email: None,
            avatar_url: None,
            email_verified: false,
        },
    }
}

#[async_trait]
impl FederatedOAuthClient for CompletionSpy {
    async fn authorize(
        &self,
        _redirect_uri: &str,
        _state: &str,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        self.calls().legacy[0] += 1;
        Err(FederatedOAuthError::Authorize)
    }

    async fn exchange_code(
        &self,
        _code: &str,
        _redirect_uri: &str,
    ) -> Result<ProviderTokens, FederatedOAuthError> {
        self.calls().legacy[1] += 1;
        Err(FederatedOAuthError::ExchangeCode)
    }

    async fn user_profile(
        &self,
        _access_token: &str,
    ) -> Result<ProviderUserProfile, FederatedOAuthError> {
        self.calls().legacy[2] += 1;
        Err(FederatedOAuthError::UserProfile)
    }

    async fn authorize_with_pkce(
        &self,
        _redirect_uri: &str,
        _state: &str,
        _code_challenge: Option<&str>,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        self.calls().legacy[3] += 1;
        Err(FederatedOAuthError::Authorize)
    }

    async fn exchange_code_with_pkce(
        &self,
        _code: &str,
        _redirect_uri: &str,
        _code_verifier: Option<&str>,
    ) -> Result<ProviderTokens, FederatedOAuthError> {
        self.calls().legacy[4] += 1;
        Err(FederatedOAuthError::ExchangeCode)
    }

    async fn authorize_authentication(
        &self,
        _request: &AuthorizeAuthenticationRequest,
    ) -> Result<AuthorizeResponse, FederatedOAuthError> {
        self.calls().authorize_v2 += 1;
        Err(FederatedOAuthError::Authorize)
    }

    async fn complete_authorization(
        &self,
        request: &CompleteAuthorizationRequest,
    ) -> Result<ProviderAuthentication, FederatedOAuthError> {
        // Observe storage AT the outbound port call, not just after completion.
        let stored = self.writer.snapshot();
        let saw_consumed_and_purged = stored.consumed_at.is_some()
            && stored.pkce_verifier.is_none()
            && stored.authentication_nonce.is_none()
            && self.writer.state().committed_consumes == 1;
        self.calls().completions.push(CompletionReceipt {
            request: request.clone(),
            saw_consumed_and_purged,
        });
        if let Some(error) = self.failure {
            return Err(error);
        }
        Ok(completion_authentication())
    }
}

fn completion_setup(
    pkce: bool,
    failure: Option<FederatedOAuthError>,
) -> (
    BeginOAuthTransaction,
    CompletionSpy,
    CompleteAuthorizationRequest,
) {
    let input = begin_input("relink", pkce);
    let writer = Arc::new(Writer::default());
    writer.install(&input, Some(URL_SAFE_NO_PAD.encode([107; 32])));
    // Only the fixture snapshots secrets; the capability is obtained by consume.
    let original = writer.snapshot();
    let expected = CompleteAuthorizationRequest {
        code: "HelperAuthorizationCodeSentinel".into(),
        redirect_uri: original.redirect_uri,
        code_verifier: original.pkce_verifier,
        expected_nonce: original
            .authentication_nonce
            .expect("fixture canonical nonce"),
    };
    let spy = CompletionSpy {
        writer,
        calls: Mutex::new(CompletionCalls::default()),
        failure,
    };
    (input, spy, expected)
}

fn assert_exact_completion(spy: &CompletionSpy, expected: &CompleteAuthorizationRequest) {
    let calls = spy.calls();
    assert_eq!(calls.legacy, [0; 5], "G-2: no legacy port operation");
    assert_eq!(calls.authorize_v2, 0, "G-2: callback must not re-authorize");
    assert_eq!(calls.completions.len(), 1, "G-2: one v2 call, no retry");
    let receipt = &calls.completions[0];
    assert!(
        receipt.saw_consumed_and_purged,
        "G-2: writer consumption/double-purge precedes complete"
    );
    assert!(
        receipt.request.code == expected.code,
        "G-2: exact callback code"
    );
    assert!(
        receipt.request.redirect_uri == expected.redirect_uri,
        "G-2: redirect belongs to consumed context"
    );
    assert!(
        receipt.request.expected_nonce == expected.expected_nonce,
        "G-2: original durable nonce, not a replacement"
    );
    assert!(
        receipt.request.code_verifier == expected.code_verifier,
        "G-2: original optional verifier, not challenge or replacement"
    );
}

fn assert_same_authentication(actual: &ProviderAuthentication) {
    let expected = completion_authentication();
    assert!(actual.tokens.access_token == expected.tokens.access_token);
    assert!(actual.tokens.refresh_token == expected.tokens.refresh_token);
    assert!(actual.tokens.expires_in == expected.tokens.expires_in);
    assert!(actual.profile.id == expected.profile.id);
    assert!(actual.profile.username == expected.profile.username);
    assert!(actual.profile.email == expected.profile.email);
    assert!(actual.profile.avatar_url == expected.profile.avatar_url);
    assert!(actual.profile.email_verified == expected.profile.email_verified);
}

// Obligation G-2 / §2: only committed context supplies the secrets to complete.
// Scenario: consume a matching row, then invoke the real helper once, ±PKCE.
// Oracle: exact captured request, storage already purged at call time, one v2/
// zero v1 operations, and complete tokens+identity returned unchanged.
// Mutation caught: regenerated nonce, verifier/challenge confusion, split v1 flow.
#[rstest::rstest]
#[case::pkce(true)]
#[case::no_pkce(false)]
#[tokio::test]
async fn complete_forwards_original_consumed_context_once_v2(#[case] pkce: bool) {
    let (input, spy, expected) = completion_setup(pkce, None);
    let capability = ConsumedOAuthTransaction::consume(spy.writer.as_ref(), callback(&input))
        .await
        .expect("canonical matching fixture must yield real capability");
    let actual = capability
        .complete_authorization(&spy, &expected.code)
        .await
        .expect("v2 port returns complete authentication fixture");
    assert_exact_completion(&spy, &expected);
    assert_same_authentication(&actual);
    assert_committed_and_purged(&spy.writer);
}

fn assert_completion_error_is_sanitized(
    error: &DomainError,
    expected: &CompleteAuthorizationRequest,
) {
    assert!(
        matches!(error, DomainError::OAuth2Error(_)),
        "G-2: generic OAuth error"
    );
    let display = error.to_string();
    let debug = format!("{error:?}");
    // The port enum carries no message/body: malicious HTTP-body reflection is
    // tested at the transport boundary, not simulated by inventing an error API.
    let authentication = completion_authentication();
    for secret in [
        Some(expected.code.as_str()),
        Some(expected.expected_nonce.as_str()),
        expected.code_verifier.as_deref(),
        Some(authentication.tokens.access_token.as_str()),
        authentication.tokens.refresh_token.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        assert!(
            !display.contains(secret),
            "G-2: Display must not reveal secrets"
        );
        assert!(
            !debug.contains(secret),
            "G-2: Debug must not reveal secrets"
        );
    }
}

// Obligation G-2 / §2: v2 failure is generic, with no retry/fallback/profile call.
// Scenario: a real consumed capability gets ExchangeCode or UserProfile, ±PKCE.
// Oracle: sanitized DomainError, exact single receipt, still consumed/purged.
// Mutation caught: retrying completion, v1 fallback, echoing consumed secrets.
#[rstest::rstest]
#[case::exchange_pkce(true, FederatedOAuthError::ExchangeCode)]
#[case::exchange_no_pkce(false, FederatedOAuthError::ExchangeCode)]
#[case::profile_pkce(true, FederatedOAuthError::UserProfile)]
#[case::profile_no_pkce(false, FederatedOAuthError::UserProfile)]
#[tokio::test]
async fn complete_failure_is_generic_without_retry_or_legacy_fallback(
    #[case] pkce: bool,
    #[case] failure: FederatedOAuthError,
) {
    let (input, spy, expected) = completion_setup(pkce, Some(failure));
    let capability = ConsumedOAuthTransaction::consume(spy.writer.as_ref(), callback(&input))
        .await
        .expect("canonical matching fixture must yield real capability");
    let result = capability
        .complete_authorization(&spy, &expected.code)
        .await;
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("G-2: refused completion must not return partial authentication"),
    };
    assert_completion_error_is_sanitized(&error, &expected);
    assert_exact_completion(&spy, &expected);
    assert_committed_and_purged(&spy.writer);
}

// Obligation G-2 / §4: legacy/corrupt context never reaches the helper's port.
// Scenario: try consume, calling the helper only if a capability is granted.
// Oracle: generic refusal after purge, zero calls of every client operation.
// Mutation caught: accepting legacy context and synthesizing a nonce at completion.
#[rstest::rstest]
#[case::legacy(None)]
#[case::corrupt(Some("CorruptNonceSentinel".into()))]
#[tokio::test]
async fn refused_nonce_context_never_calls_completion_client(#[case] nonce: Option<String>) {
    let (input, spy, expected) = completion_setup(true, None);
    spy.writer.install(&input, nonce);
    let consumed = ConsumedOAuthTransaction::consume(spy.writer.as_ref(), callback(&input)).await;
    if let Ok(capability) = &consumed {
        let _ = capability
            .complete_authorization(&spy, &expected.code)
            .await;
    }
    assert!(matches!(
        consumed,
        Err(OAuthTransactionError::InvalidTransaction)
    ));
    assert_committed_and_purged(&spy.writer);
    let calls = spy.calls();
    assert!(
        calls.completions.is_empty(),
        "G-2: no complete without capability"
    );
    assert_eq!(calls.authorize_v2, 0);
    assert_eq!(calls.legacy, [0; 5]);
}
