//! QA coverage90 A: pure account, email and completion-token state transitions.
use chrono::{Duration, Utc};
use iam_domain::entity::{
    email_verification::EmailVerification,
    password_reset_token::PasswordResetToken,
    registration_token::{RegistrationFlow, RegistrationTokenClaims},
    user::User,
    user_email::UserEmail,
};
use uuid::Uuid;

#[test]
fn completing_password_registration_preserves_identity_credentials_and_avatar() {
    let mut user = User::new_incomplete_with_password(
        "test-password-hash".into(),
        Some("https://aiforall.test/avatar".into()),
    );
    let before = user.clone();
    user.updated_at = Utc::now() - Duration::days(1);
    assert!(user.is_registration_incomplete());
    assert!(!user.is_registration_complete());
    user.complete_registration("reader".into());
    assert!(user.is_registration_complete());
    assert!(!user.is_registration_incomplete());
    assert_eq!(user.username.as_deref(), Some("reader"));
    assert_eq!((user.id, user.created_at), (before.id, before.created_at));
    assert_eq!(
        (user.password_hash, user.avatar_url),
        (before.password_hash, before.avatar_url)
    );
    assert!(user.updated_at >= before.updated_at);
}

#[test]
fn partial_profile_updates_do_not_clear_existing_fields_or_change_credentials() {
    let mut user = User::new_with_password(
        "reader".into(),
        "original-hash".into(),
        Some("https://aiforall.test/old".into()),
    );
    let before = user.clone();
    user.update_profile(None, Some("https://aiforall.test/new".into()));
    assert_eq!(user.username, before.username);
    assert_eq!(
        user.avatar_url.as_deref(),
        Some("https://aiforall.test/new")
    );
    user.update_profile(Some("renamed-reader".into()), None);
    assert_eq!(user.username.as_deref(), Some("renamed-reader"));
    assert_eq!(
        user.avatar_url.as_deref(),
        Some("https://aiforall.test/new")
    );
    user.update_profile(None, None);
    assert_eq!(user.username.as_deref(), Some("renamed-reader"));
    assert_eq!(user.password_hash, before.password_hash);
    assert_eq!((user.id, user.created_at), (before.id, before.created_at));
}

#[test]
fn enabling_and_rotating_password_keeps_registration_state_and_identity() {
    let mut user = User::new("reader".into(), None);
    let before = user.clone();
    assert!(!user.has_password());
    user.update_password("first-test-hash".into());
    assert!(user.has_password());
    assert_eq!(user.password_hash.as_deref(), Some("first-test-hash"));
    user.update_password("rotated-test-hash".into());
    assert_eq!(user.password_hash.as_deref(), Some("rotated-test-hash"));
    assert!(user.is_registration_complete());
    assert_eq!(
        (user.id, user.username, user.created_at),
        (before.id, before.username, before.created_at)
    );
}

#[test]
fn verifying_and_promoting_email_preserves_address_and_ownership() {
    let mut email =
        UserEmail::new_secondary(Uuid::from_u128(42), "reader@aiforall.test".into(), false);
    let before = email.clone();
    email.updated_at = Utc::now() - Duration::days(1);
    email.verify();
    assert!(email.is_verified);
    assert!(!email.is_primary);
    assert!(email.updated_at >= before.updated_at);
    email.set_as_primary();
    assert!(email.is_primary);
    assert!(email.is_verified);
    email.set_as_secondary();
    assert!(!email.is_primary);
    assert!(email.is_verified);
    assert_eq!(
        (email.id, email.user_id, email.email, email.created_at),
        (before.id, before.user_id, before.email, before.created_at)
    );
}

#[test]
fn used_or_expired_reset_tokens_cannot_be_valid_even_when_the_raw_token_matches() {
    let raw = "reset-ticket-coverage90";
    let mut token = PasswordResetToken::new(Uuid::from_u128(42), raw, 24);
    let before = token.clone();
    assert!(token.is_valid());
    assert!(!token.verify_token("different-ticket"));
    token.mark_as_used();
    assert!(token.is_used());
    assert!(!token.is_valid());
    assert!(token.verify_token(raw));
    assert_eq!(
        (token.id, token.user_id, token.token_hash, token.expires_at),
        (
            before.id,
            before.user_id,
            before.token_hash,
            before.expires_at
        )
    );
    let expired = PasswordResetToken::new(Uuid::from_u128(42), raw, -24);
    assert!(expired.is_expired());
    assert!(!expired.is_used());
    assert!(!expired.is_valid());
}

#[test]
fn registration_claims_keep_flow_subject_expiry_and_legacy_decode_distinct() {
    let user_id = Uuid::from_u128(42);
    for (claims, flow) in [
        (
            RegistrationTokenClaims::new(user_id, "reader@aiforall.test".into()),
            RegistrationFlow::EmailPassword,
        ),
        (
            RegistrationTokenClaims::new_oauth(user_id, "reader@aiforall.test".into()),
            RegistrationFlow::OAuth,
        ),
    ] {
        assert_eq!(claims.exp - claims.iat, 86_400);
        assert_eq!(claims.get_user_id().expect("valid subject ID"), user_id);
        assert!(claims.is_registration_token());
        assert!(!claims.is_expired());
        assert_eq!(claims.is_oauth_flow(), flow == RegistrationFlow::OAuth);
        assert_eq!(
            claims.is_email_password_flow(),
            flow == RegistrationFlow::EmailPassword
        );
        let body = serde_json::to_value(&claims).expect("claims serialize");
        assert!(body.get("provider_info").is_none());
        assert!(body.get("iss").is_none());
        assert!(body.get("aud").is_none());
        let decoded: RegistrationTokenClaims =
            serde_json::from_value(body).expect("legacy claims decode");
        assert_eq!(decoded.flow, flow);
        assert_eq!(decoded.get_user_id().expect("decoded subject ID"), user_id);
    }
}

#[test]
fn registration_claims_reject_wrong_subject_invalid_user_id_and_expired_time() {
    let mut claims =
        RegistrationTokenClaims::new(Uuid::from_u128(42), "reader@aiforall.test".into());
    claims.sub = "access".into();
    claims.user_id = "not-a-uuid".into();
    claims.exp = Utc::now().timestamp() - 86_400;
    assert!(!claims.is_registration_token());
    assert!(claims.get_user_id().is_err());
    assert!(claims.is_expired());
}

#[test]
fn email_verification_expiry_does_not_depend_on_account_or_token_contents() {
    let live = EmailVerification::new(
        "reader@aiforall.test".into(),
        "verification-ticket".into(),
        24,
    );
    let expired = EmailVerification::new(
        "reader@aiforall.test".into(),
        "verification-ticket".into(),
        -24,
    );
    assert!(!live.is_expired());
    assert!(expired.is_expired());
    assert_eq!(live.expires_at - live.created_at, Duration::hours(24));
}
