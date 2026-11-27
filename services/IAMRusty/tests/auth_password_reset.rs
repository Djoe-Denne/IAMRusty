// Include common test utilities and fixtures

mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
#[path = "fixtures/mod.rs"]
mod fixtures;
mod utils;

use base64::Engine as _;
use common::{setup_test_server, setup_test_server_with_mock_events};
use fixtures::DbFixtures;
use iam_domain::entity::events::IamDomainEvent;
use reqwest::StatusCode;
use serde_json::{json, Value};
use serial_test::serial;
use tokio::time::Duration;

// 🔐 Password Reset Tests
// Tests for the complete password reset flow: request → validate → confirm

// ============================================================================
// 📧 Password Reset Request Tests - POST /auth/password/reset-request
// ============================================================================

/// Tests password reset request for an existing user with email/password authentication.
/// Verifies that the endpoint returns a success message (without revealing user existence for security).
/// This ensures the happy path works for legitimate reset requests.
/// ALSO VERIFIES: That a `PasswordResetRequested` event IS published for valid requests.
#[tokio::test]
#[serial]
async fn test_password_reset_request_existing_user_success() {
    let (fixture, base_url, client, mock_event_publisher) =
        Box::pin(setup_test_server_with_mock_events())
            .await
            .expect("Failed to setup test server");

    // Create a user with email/password authentication
    let user_email = "reset-test@example.com";
    let user_password = "securepassword123";

    DbFixtures::create_user_with_email_password(
        &fixture.db(),
        user_email,
        user_password,
        Some("resetuser"),
    )
    .await
    .expect("Failed to create test user");

    // Request password reset
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-request"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "email": user_email
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 for security (no user enumeration)
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("reset link has been sent"));

    // ✅ VERIFY: Event should be published for valid password reset request
    let events = mock_event_publisher
        .wait_for_event_count(1, Duration::from_secs(5))
        .await;
    assert_eq!(
        events.len(),
        1,
        "Should publish exactly one event for valid password reset"
    );

    let event = &events[0];
    assert_eq!(
        event.event_type, "password_reset_requested",
        "Event should be PasswordResetRequested"
    );

    // Parse the JSON data as IamDomainEvent to handle the envelope structure
    let event_domain: IamDomainEvent = serde_json::from_str(event.json_data.as_str()).unwrap();

    // Extract the PasswordResetRequested event
    match event_domain {
        IamDomainEvent::PasswordResetRequested(password_reset_event) => {
            // Check the email in the event data
            assert_eq!(
                password_reset_event.email, user_email,
                "Event should contain the correct email"
            );
        }
        _ => panic!("Expected PasswordResetRequested event"),
    }
}

/// Tests password reset request for a non-existent email address.
/// Verifies that the endpoint returns the same success message to prevent user enumeration attacks.
/// This security measure prevents attackers from discovering which emails are registered.
/// ALSO VERIFIES: That NO event is published for non-existent emails (security).
#[tokio::test]
#[serial]
async fn test_password_reset_request_nonexistent_email_security() {
    let (_fixture, base_url, client, mock_event_publisher) =
        Box::pin(setup_test_server_with_mock_events())
            .await
            .expect("Failed to setup test server");

    // Request password reset for non-existent email
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-request"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "email": "nonexistent@example.com"
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 for security (no user enumeration)
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("reset link has been sent"));

    // ✅ VERIFY: NO event should be published for non-existent email (security)
    let events = mock_event_publisher.get_published_events();
    assert_eq!(
        events.len(),
        0,
        "Should NOT publish any events for non-existent email"
    );
}

/// Tests password reset request for a user who only has OAuth authentication (no password).
/// Verifies that the endpoint returns a success message without revealing authentication methods.
/// This prevents attackers from learning whether users have password authentication enabled.
/// ALSO VERIFIES: That NO event is published for OAuth-only users (security).
#[tokio::test]
#[serial]
async fn test_password_reset_request_oauth_only_user_security() {
    let (fixture, base_url, client, mock_event_publisher) =
        Box::pin(setup_test_server_with_mock_events())
            .await
            .expect("Failed to setup test server");

    // Create a user that only has OAuth authentication (no password)
    let (_user, _provider_token) = DbFixtures::create_user_with_oauth_provider(
        &fixture.db(),
        "oauth-only@example.com",
        "oauthuser",
        "github",
    )
    .await
    .expect("Failed to create OAuth user");

    // Request password reset for OAuth-only user
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-request"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "email": "oauth-only@example.com"
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 for security (no user enumeration)
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("reset link has been sent"));

    // ✅ VERIFY: NO event should be published for OAuth-only user (security)
    let events = mock_event_publisher.get_published_events();
    assert_eq!(
        events.len(),
        0,
        "Should NOT publish any events for OAuth-only user without password"
    );
}

/// Tests password reset request with various invalid email formats.
/// Verifies that malformed emails are rejected with proper validation errors.
/// This ensures input validation catches obviously invalid data before processing.
#[tokio::test]
#[serial]
async fn test_password_reset_request_invalid_email_format() {
    let (_fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    let invalid_emails = vec![
        "invalid-email",
        "missing@domain",
        "@missing-local.com",
        "spaces in@email.com",
        "double@@domain.com",
        "",
    ];

    for invalid_email in invalid_emails {
        let response = client
            .post(format!("{base_url}/api/auth/password/reset-request"))
            .header("Content-Type", "application/json")
            .json(&json!({
                "email": invalid_email
            }))
            .send()
            .await
            .expect("Failed to send request");

        // ✅ Should return 422 for invalid email format
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "Should return 422 for invalid email: {invalid_email}"
        );

        let body: Value = response.json().await.expect("Failed to parse response");
        assert!(body["error"]["message"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("email"));
    }
}

/// Tests password reset request with missing email field.
/// Verifies that requests without required fields are rejected with 422 status.
/// This ensures proper validation of required parameters.
#[tokio::test]
#[serial]
async fn test_password_reset_request_missing_email() {
    let (_fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Request password reset without email field
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-request"))
        .header("Content-Type", "application/json")
        .json(&json!({}))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 422 for missing required field
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// Tests password reset request with malformed JSON payload.
/// Verifies that syntactically invalid JSON is rejected with 400 status.
/// This ensures proper handling of malformed request bodies.
#[tokio::test]
#[serial]
async fn test_password_reset_request_malformed_json() {
    let (_fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Send malformed JSON
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-request"))
        .header("Content-Type", "application/json")
        .body("{ invalid json")
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 400 for malformed JSON
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// 🔍 Token Validation Tests - POST /auth/password/reset-validate
// ============================================================================

/// Tests validation of a valid, unexpired reset token.
/// Verifies that legitimate tokens are accepted and return masked email for privacy.
/// This allows UIs to confirm token validity before showing password reset forms.
#[tokio::test]
#[serial]
async fn test_password_reset_validate_valid_token() {
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Create user and generate reset token
    let user_email = "validate-test@example.com";
    let user_password = "securepassword123";

    let user = DbFixtures::create_user_with_email_password(
        &fixture.db(),
        user_email,
        user_password,
        Some("validateuser"),
    )
    .await
    .expect("Failed to create test user");

    let reset_token_fixture = DbFixtures::password_reset_token()
        .valid(user.id())
        .commit(fixture.db())
        .await
        .expect("Failed to create reset token");
    let reset_token = reset_token_fixture.token();

    // Validate the token
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-validate"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "token": reset_token
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 for valid token
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["valid"], true);

    // Email should be masked for privacy
    let masked_email = body["email"].as_str().unwrap();
    assert!(masked_email.contains('*'));
    assert!(!masked_email.contains("validate-test"));
}

/// Tests validation of an expired reset token.
/// Verifies that expired tokens are rejected to prevent use of old reset links.
/// This ensures tokens have a limited lifespan for security.
#[tokio::test]
#[serial]
async fn test_password_reset_validate_expired_token() {
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Create user and generate expired reset token
    let user_email = "expired-test@example.com";
    let user_password = "securepassword123";

    let user = DbFixtures::create_user_with_email_password(
        &fixture.db(),
        user_email,
        user_password,
        Some("expireduser"),
    )
    .await
    .expect("Failed to create test user");

    let expired_token_fixture = DbFixtures::password_reset_token()
        .expired(user.id())
        .commit(fixture.db())
        .await
        .expect("Failed to create expired token");
    let expired_token = expired_token_fixture.token();

    // Validate the expired token
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-validate"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "token": expired_token
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 400 for expired token
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Invalid or expired"));
}

/// Tests validation of a previously used reset token.
/// Verifies that consumed tokens cannot be validated again to prevent reuse.
/// This ensures one-time use semantics for reset tokens.
#[tokio::test]
#[serial]
async fn test_password_reset_validate_used_token() {
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Create user and generate used reset token
    let user_email = "used-test@example.com";
    let user_password = "securepassword123";

    let user = DbFixtures::create_user_with_email_password(
        &fixture.db(),
        user_email,
        user_password,
        Some("useduser"),
    )
    .await
    .expect("Failed to create test user");

    let used_token_fixture = DbFixtures::password_reset_token()
        .used(user.id())
        .commit(fixture.db())
        .await
        .expect("Failed to create used token");
    let used_token = used_token_fixture.token();

    // Validate the used token
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-validate"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "token": used_token
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 400 for used token
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Invalid or expired"));
}

/// Tests validation of a completely invalid/non-existent token.
/// Verifies that fake or malformed tokens are rejected properly.
/// This ensures robust handling of invalid token attempts.
#[tokio::test]
#[serial]
async fn test_password_reset_validate_invalid_token() {
    let (_fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Validate completely invalid token
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-validate"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "token": "invalid-token-that-does-not-exist"
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 400 for invalid token
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Invalid or expired"));
}

/// Tests validation request with missing token field.
/// Verifies that requests without required token parameter are rejected.
/// This ensures proper validation of required fields.
#[tokio::test]
#[serial]
async fn test_password_reset_validate_missing_token() {
    let (_fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Validate without token field
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-validate"))
        .header("Content-Type", "application/json")
        .json(&json!({}))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 422 for missing required field
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
#[serial]
#[ignore = "opt-in OpenAPI check; run via it-native.ps1 -Suite iam -IamOpenApi"]
async fn iam_openapi_contract() {
    use std::path::Path;
    use tokio::{process::Command, time::timeout};

    assert!(
        std::env::var("RUSTYCOG_TEST_RUNNER_MODE").as_deref() == Ok("local")
            && std::env::var_os("RUSTYCOG_TEST_LEDGER_DIR").is_some(),
        "run the isolated fixture with it-native.ps1 -Suite iam -IamOpenApi"
    );
    let python = std::env::var_os("AIFORALL_SCHEMATHESIS_PYTHON")
        .expect("set AIFORALL_SCHEMATHESIS_PYTHON to the qualified Python interpreter");
    assert!(
        Path::new(&python).is_absolute() && Path::new(&python).is_file(),
        "use an absolute path to the Schemathesis virtual environment interpreter"
    );
    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ops/tests/security/iam_openapi.py");
    let (fixture, base_url, _) = Box::pin(setup_test_server())
        .await
        .expect("start the isolated IAM listener");
    let mut runner = Command::new(python);
    runner
        .args(["-I", "-B"])
        .arg(script)
        .arg(base_url)
        .kill_on_drop(true);
    let result = timeout(Duration::from_secs(30), runner.status()).await;
    common::cleanup_test_servers(&fixture)
        .await
        .expect("stop the isolated IAM listener");
    assert!(
        result
            .expect("OpenAPI runner exceeded 30 seconds")
            .expect("start the OpenAPI runner")
            .success(),
        "OpenAPI checks failed"
    );
}

// ============================================================================
// ✅ Password Reset Confirm Tests (Unauthenticated) - POST /auth/password/reset-confirm
// ============================================================================

#[tokio::test]
#[serial]
async fn recovery_commit_denies_unexpired_access_on_direct_iam() {
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("start direct IAM HTTP/PostgreSQL fixture");

    fixture_cleanup::run(&fixture, async {
        let email = "direct-recovery-cutoff@example.com";
        let old_password = "OldDirect1!";
        let new_password = "NewDirect2!";
        let user = DbFixtures::create_user_with_email_password(
            &fixture.db(),
            email,
            old_password,
            Some("direct_recovery"),
        )
        .await
        .expect("seed password account");
        let proof = DbFixtures::password_reset_token()
            .valid(user.id())
            .commit(fixture.db())
            .await
            .expect("seed valid recovery proof");

        let login_url = format!("{base_url}/api/auth/login");
        let profile_url = format!("{base_url}/api/me");
        let login = client
            .post(&login_url)
            .json(&json!({"email": email, "password": old_password}))
            .send()
            .await
            .expect("sign in before recovery");
        assert_eq!(login.status(), StatusCode::OK);
        let login: Value = login.json().await.expect("read initial session");
        let old_access = login["access_token"].as_str().expect("initial access JWT");
        let old_refresh = login["refresh_token"]
            .as_str()
            .expect("initial refresh token");

        // Use the verified token's signed expiry only as a timing guard, not for admission.
        let encoded_claims = old_access.split('.').nth(1).expect("JWT claims segment");
        let claims = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded_claims)
            .expect("decode JWT claims segment");
        let claims: Value = serde_json::from_slice(&claims).expect("read JWT claims");
        let expires_at = claims["exp"].as_i64().expect("JWT expiry");

        let baseline = client
            .patch(&profile_url)
            .bearer_auth(old_access)
            .json(&json!({"username": "before_recovery"}))
            .send()
            .await
            .expect("write protected profile with the initial JWT");
        assert_eq!(baseline.status(), StatusCode::OK);
        let baseline: Value = baseline.json().await.expect("read baseline profile");
        assert_eq!(baseline["id"], user.id().to_string());
        assert_eq!(baseline["username"], "before_recovery");

        let confirmation = client
            .post(format!("{base_url}/api/auth/password/reset-confirm"))
            .json(&json!({"token": proof.token(), "new_password": new_password}))
            .send()
            .await
            .expect("confirm recovery through the real link endpoint");
        assert_eq!(confirmation.status(), StatusCode::OK);
        let confirmation: Value = confirmation.json().await.expect("read confirmation");
        let fields = confirmation.as_object().expect("confirmation object");
        assert_eq!(fields.len(), 1, "confirmation must contain only a message");
        assert!(
            confirmation["message"].as_str() == Some("Password has been successfully reset"),
            "preserve the existing recovery confirmation"
        );

        let stale = client
            .patch(&profile_url)
            .bearer_auth(old_access)
            .json(&json!({"username": "after_recovery"}))
            .send()
            .await
            .expect("attempt a fresh protected write after reset commit");
        let stale_status = stale.status();
        assert!(
            expires_at > chrono::Utc::now().timestamp(),
            "the old JWT must still be unexpired after the protected response"
        );

        let renewal = client
            .post(format!("{base_url}/api/token/refresh"))
            .json(&json!({"refresh_token": old_refresh}))
            .send()
            .await
            .expect("try to renew the prior session");
        assert_eq!(renewal.status(), StatusCode::UNAUTHORIZED);
        let denial: Value = renewal.json().await.expect("read renewal denial");
        assert!(
            denial.get("access_token").is_none() && denial.get("refresh_token").is_none(),
            "denied renewal must not return a token pair"
        );

        let fresh = client
            .post(&login_url)
            .json(&json!({"email": email, "password": new_password}))
            .send()
            .await
            .expect("explicitly sign in with the new password");
        assert_eq!(fresh.status(), StatusCode::OK);
        let fresh: Value = fresh.json().await.expect("read new session");
        let fresh_access = fresh["access_token"].as_str().expect("new access JWT");
        let current = client
            .get(&profile_url)
            .bearer_auth(fresh_access)
            .send()
            .await
            .expect("read the profile through the fresh session");
        assert_eq!(current.status(), StatusCode::OK);
        let current: Value = current.json().await.expect("read current profile");
        assert_eq!(current["id"], user.id().to_string());
        assert_eq!(
            stale_status,
            StatusCode::UNAUTHORIZED,
            "recovery commit must deny a prior unexpired JWT on direct IAM"
        );
        assert_eq!(
            current["username"], "before_recovery",
            "the denied protected write must not change the profile"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn recovery_advances_issuance_epoch_but_request_and_signed_in_change_do_not() {
    let email = "recovery-epoch@example.com";
    let username = "recovery_epoch";
    let (fixture, base, client) = Box::pin(setup_test_server())
        .await
        .expect("start IAM HTTP/PostgreSQL fixture");
    fixture_cleanup::run(&fixture, async {
        let old = "OldEpoch1!";
        let changed = "ChangedEpoch2!";
        let recovered = "RecoveredEpoch3!";
        let user =
            DbFixtures::create_user_with_email_password(&fixture.db(), email, old, Some(username))
                .await
                .expect("seed password account");
        let login = |password: &str| {
            client
                .post(format!("{base}/api/auth/login"))
                .json(&json!({"email": email, "password": password}))
                .send()
        };
        let epoch = |session: &Value| {
            let segment = session["access_token"]
                .as_str()
                .and_then(|jwt| jwt.split('.').nth(1))
                .expect("access JWT claims segment");
            let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(segment)
                .expect("decode claims segment");
            let claims: Value = serde_json::from_slice(&bytes).expect("read claims");
            claims["auth_epoch"].clone()
        };
        let first = login(old).await.expect("login");
        assert_eq!(first.status(), StatusCode::OK);
        let first: Value = first.json().await.expect("read login");
        assert_eq!(epoch(&first), "0");
        let initial_access = first["access_token"].as_str().expect("initial access");
        let legacy_refresh = uuid::Uuid::new_v4().to_string();
        let legacy = DbFixtures::refresh_token()
            .valid(user.id())
            .token(&legacy_refresh[..])
            .commit(fixture.db())
            .await
            .expect("persist refresh row without an explicit epoch");
        assert_eq!(legacy.model().auth_epoch, "0");
        let request = client
            .post(format!("{base}/api/auth/password/reset-request"))
            .json(&json!({"email": email}))
            .send()
            .await
            .expect("request reset");
        assert_eq!(request.status(), StatusCode::OK);
        let renewed = client
            .post(format!("{base}/api/token/refresh"))
            .json(&json!({"refresh_token": legacy_refresh}))
            .send()
            .await
            .expect("renew after request");
        assert_eq!(renewed.status(), StatusCode::OK);
        let renewed: Value = renewed.json().await.expect("read renewal");
        assert_eq!(epoch(&renewed), "0");

        let change = client
            .post(format!("{base}/api/auth/password/reset-authenticated"))
            .bearer_auth(initial_access)
            .json(&json!({"current_password": old, "new_password": changed}))
            .send()
            .await
            .expect("change password while signed in");
        assert_eq!(change.status(), StatusCode::OK);
        let purged = client
            .post(format!("{base}/api/token/refresh"))
            .json(&json!({"refresh_token": renewed["refresh_token"]}))
            .send()
            .await
            .expect("try refresh purged by signed-in change");
        assert_eq!(purged.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            client
                .get(format!("{base}/api/me"))
                .bearer_auth(initial_access)
                .send()
                .await
                .expect("use initial access after signed-in change")
                .status(),
            StatusCode::OK
        );
        let changed_login = login(changed).await.expect("login with changed password");
        assert_eq!(changed_login.status(), StatusCode::OK);
        let changed_login: Value = changed_login.json().await.expect("read login");
        assert_eq!(epoch(&changed_login), "0");
        let before_recovery = client
            .post(format!("{base}/api/token/refresh"))
            .json(&json!({"refresh_token": changed_login["refresh_token"]}))
            .send()
            .await
            .expect("rotate before committing recovery");
        assert_eq!(before_recovery.status(), StatusCode::OK);
        let before_recovery: Value = before_recovery.json().await.expect("read rotation");
        assert_eq!(epoch(&before_recovery), "0");

        let proof = DbFixtures::password_reset_token()
            .valid(user.id())
            .commit(fixture.db())
            .await
            .expect("seed valid recovery proof");
        let confirm = || {
            client
                .post(format!("{base}/api/auth/password/reset-confirm"))
                .json(&json!({"token": proof.token(), "new_password": recovered}))
                .send()
        };
        assert_eq!(
            confirm().await.expect("confirm recovery").status(),
            StatusCode::OK
        );
        assert!(!confirm()
            .await
            .expect("replay recovery")
            .status()
            .is_success());
        let denied = client
            .post(format!("{base}/api/token/refresh"))
            .json(&json!({"refresh_token": before_recovery["refresh_token"]}))
            .send()
            .await
            .expect("try replacement refresh after committing recovery");
        assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
        let fresh = login(recovered).await.expect("login after recovery");
        assert_eq!(fresh.status(), StatusCode::OK);
        let fresh: Value = fresh.json().await.expect("read fresh session");
        assert_eq!(epoch(&fresh), "1");
        let rotated = client
            .post(format!("{base}/api/token/refresh"))
            .json(&json!({"refresh_token": fresh["refresh_token"]}))
            .send()
            .await
            .expect("rotate post-recovery refresh");
        assert_eq!(rotated.status(), StatusCode::OK);
        assert_eq!(epoch(&rotated.json().await.expect("read rotation")), "1");
    })
    .await;
    drop(fixture);

    // The previous fixture left advanced user and refresh epochs in the shared
    // test database. A fresh descriptor must not inherit either account name.
    let (fresh_fixture, _, _, _) = Box::pin(setup_test_server_with_mock_events())
        .await
        .expect("start a fresh fixture after recovery");
    fixture_cleanup::run(&fresh_fixture, async {
        let user = DbFixtures::create_user_with_email_password(
            &fresh_fixture.db(),
            email,
            "NewFixture1!",
            Some(username),
        )
        .await
        .expect("prior fixture must not retain the username or email");
        assert_eq!(user.model().auth_epoch, "0");
    })
    .await;
}

/// Tests successful password reset using a valid token (unauthenticated flow).
/// Verifies that password is actually changed and no auth tokens are returned.
/// This ensures secure password reset without automatic login (preventing token hijacking).
#[tokio::test]
#[serial]
async fn test_password_reset_confirm_unauthenticated_success() {
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Create user and generate reset token
    let user_email = "reset-confirm-test@example.com";
    let old_password = "oldpassword123";
    let new_password = "newpassword456";

    let user = DbFixtures::create_user_with_email_password(
        &fixture.db(),
        user_email,
        old_password,
        Some("resetuser"),
    )
    .await
    .expect("Failed to create test user");

    let reset_token_fixture = DbFixtures::password_reset_token()
        .valid(user.id())
        .commit(fixture.db())
        .await
        .expect("Failed to create reset token");
    let reset_token = reset_token_fixture.token();

    // Reset password using token
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-confirm"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "token": reset_token,
            "new_password": new_password
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 for successful reset
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("successfully reset"));

    // Should not return auth tokens - user must log in manually after password reset
    assert!(
        body["access_token"].is_null() || !body.as_object().unwrap().contains_key("access_token")
    );
    assert!(
        body["refresh_token"].is_null() || !body.as_object().unwrap().contains_key("refresh_token")
    );
    assert!(body["user"].is_null() || !body.as_object().unwrap().contains_key("user"));
    assert!(body["expires_in"].is_null() || !body.as_object().unwrap().contains_key("expires_in"));

    // Verify old password no longer works
    let login_response = client
        .post(format!("{base_url}/api/auth/login"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "email": user_email,
            "password": old_password
        }))
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);

    // Verify new password works
    let login_response = client
        .post(format!("{base_url}/api/auth/login"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "email": user_email,
            "password": new_password
        }))
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(login_response.status(), StatusCode::OK);
}

/// Tests that reset tokens cannot be reused after successful password reset.
/// Verifies that tokens are invalidated after first use to prevent replay attacks.
/// This ensures one-time use semantics and prevents token reuse vulnerabilities.
#[tokio::test]
#[serial]
async fn test_password_reset_confirm_token_reuse_prevention() {
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Create user and generate reset token
    let user_email = "reuse-test@example.com";
    let user_password = "oldpassword123";

    let user = DbFixtures::create_user_with_email_password(
        &fixture.db(),
        user_email,
        user_password,
        Some("reuseuser"),
    )
    .await
    .expect("Failed to create test user");

    let reset_token_fixture = DbFixtures::password_reset_token()
        .valid(user.id())
        .commit(fixture.db())
        .await
        .expect("Failed to create reset token");
    let reset_token = reset_token_fixture.token();

    // Use the token once
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-confirm"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "token": reset_token,
            "new_password": "newpassword456"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    // Try to use the same token again
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-confirm"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "token": reset_token,
            "new_password": "anothernewpassword789"
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should fail - token should be invalidated after first use
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Invalid or expired"));
}

/// Tests password reset with weak/invalid new passwords.
/// Verifies that password strength requirements are enforced during reset.
/// This ensures consistent password policies across all password change mechanisms.
#[tokio::test]
#[serial]
async fn test_password_reset_confirm_weak_password() {
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Create user and generate reset token
    let user_email = "weak-password-test@example.com";
    let user_password = "oldpassword123";

    let user = DbFixtures::create_user_with_email_password(
        &fixture.db(),
        user_email,
        user_password,
        Some("weakuser"),
    )
    .await
    .expect("Failed to create test user");

    let reset_token_fixture = DbFixtures::password_reset_token()
        .valid(user.id())
        .commit(fixture.db())
        .await
        .expect("Failed to create reset token");
    let reset_token = reset_token_fixture.token();

    let weak_passwords = vec![
        "",        // Empty
        "123",     // Too short
        "1234567", // Still too short (< 8 chars)
    ];

    for weak_password in weak_passwords {
        let response = client
            .post(format!("{base_url}/api/auth/password/reset-confirm"))
            .header("Content-Type", "application/json")
            .json(&json!({
                "token": reset_token,
                "new_password": weak_password
            }))
            .send()
            .await
            .expect("Failed to send request");

        // ✅ Should return 422 for weak password
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "Should reject weak password: '{weak_password}'"
        );
    }
}

/// Tests password reset with an invalid/non-existent token.
/// Verifies that fake tokens are rejected to prevent unauthorized password changes.
/// This ensures only legitimate reset tokens can change passwords.
#[tokio::test]
#[serial]
async fn test_password_reset_confirm_invalid_token() {
    let (_fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Reset password with invalid token
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-confirm"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "token": "completely-invalid-token",
            "new_password": "newpassword123"
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 400 for invalid token
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Invalid or expired"));
}

/// Tests password reset with missing required fields (token or `new_password`).
/// Verifies that incomplete requests are rejected with proper validation errors.
/// This ensures all required parameters are provided for password reset.
#[tokio::test]
#[serial]
async fn test_password_reset_confirm_missing_fields() {
    let (_fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    let test_cases = vec![
        (json!({"token": "some-token"}), "missing new_password"),
        (json!({"new_password": "password123"}), "missing token"),
        (json!({}), "missing all fields"),
    ];

    for (request_data, description) in test_cases {
        let response = client
            .post(format!("{base_url}/api/auth/password/reset-confirm"))
            .header("Content-Type", "application/json")
            .json(&request_data)
            .send()
            .await
            .expect("Failed to send request");

        // ✅ Should return 422 for missing required fields
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "Should return 422 for: {description}"
        );
    }
}

// ============================================================================
// 🔐 Authenticated Password Reset Tests - POST /auth/password/reset-authenticated
// ============================================================================

/// Tests successful password change for an authenticated user.
/// Verifies that current password verification works and no auth tokens are returned.
/// This allows authenticated users to change passwords without email-based reset flow.
#[tokio::test]
#[serial]
async fn test_password_reset_authenticated_success() {
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Create user and login to get token
    let user_email = "auth-reset-test@example.com";
    let current_password = "currentpassword123";
    let new_password = "newpassword456";

    let _user = DbFixtures::create_user_with_email_password(
        &fixture.db(),
        user_email,
        current_password,
        Some("authresetuser"),
    )
    .await
    .expect("Failed to create test user");

    // Login to get access token
    let login_response = client
        .post(format!("{base_url}/api/auth/login"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "email": user_email,
            "password": current_password
        }))
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(login_response.status(), StatusCode::OK);
    let login_body: Value = login_response
        .json()
        .await
        .expect("Failed to parse login response");
    let access_token = login_body["access_token"].as_str().unwrap();

    // Reset password with authentication
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-authenticated"))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {access_token}"))
        .json(&json!({
            "current_password": current_password,
            "new_password": new_password
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 for successful password change
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("successfully changed"));

    // Should not return auth tokens - user remains authenticated with existing token
    assert!(
        body["access_token"].is_null() || !body.as_object().unwrap().contains_key("access_token")
    );
    assert!(
        body["refresh_token"].is_null() || !body.as_object().unwrap().contains_key("refresh_token")
    );
    assert!(body["user"].is_null() || !body.as_object().unwrap().contains_key("user"));
    assert!(body["expires_in"].is_null() || !body.as_object().unwrap().contains_key("expires_in"));

    // Verify old password no longer works
    let login_response = client
        .post(format!("{base_url}/api/auth/login"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "email": user_email,
            "password": current_password
        }))
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);

    // Verify new password works
    let login_response = client
        .post(format!("{base_url}/api/auth/login"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "email": user_email,
            "password": new_password
        }))
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(login_response.status(), StatusCode::OK);
}

/// Tests password change with incorrect current password.
/// Verifies that the current password is validated to prevent unauthorized changes.
/// This ensures attackers can't change passwords without knowing the current one.
#[tokio::test]
#[serial]
async fn test_password_reset_authenticated_wrong_current_password() {
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Create user and login
    let user_email = "wrong-password-test@example.com";
    let current_password = "currentpassword123";

    DbFixtures::create_user_with_email_password(
        &fixture.db(),
        user_email,
        current_password,
        Some("wrongpassuser"),
    )
    .await
    .expect("Failed to create test user");

    // Login to get access token
    let login_response = client
        .post(format!("{base_url}/api/auth/login"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "email": user_email,
            "password": current_password
        }))
        .send()
        .await
        .expect("Failed to send login request");

    let login_body: Value = login_response
        .json()
        .await
        .expect("Failed to parse login response");
    let access_token = login_body["access_token"].as_str().unwrap();

    // Try to reset password with wrong current password
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-authenticated"))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {access_token}"))
        .json(&json!({
            "current_password": "wrongpassword",
            "new_password": "newpassword456"
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 400 for incorrect current password
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("incorrect"));
}

/// Tests password change without authentication header.
/// Verifies that the endpoint requires valid authentication for access.
/// This ensures only authenticated users can use this password change method.
#[tokio::test]
#[serial]
async fn test_password_reset_authenticated_no_auth_header() {
    let (_fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Try to reset password without authentication
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-authenticated"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "current_password": "currentpassword123",
            "new_password": "newpassword456"
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 401 for missing authentication
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Tests password change with invalid/malformed JWT token.
/// Verifies that invalid authentication tokens are rejected properly.
/// This ensures robust token validation for authenticated endpoints.
#[tokio::test]
#[serial]
async fn test_password_reset_authenticated_invalid_token() {
    let (_fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Try to reset password with invalid token
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-authenticated"))
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer invalid-jwt-token")
        .json(&json!({
            "current_password": "currentpassword123",
            "new_password": "newpassword456"
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 401 for invalid token
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Tests authenticated password change with weak new passwords.
/// Verifies that password strength requirements apply to authenticated changes.
/// This ensures consistent password policies regardless of change mechanism.
#[tokio::test]
#[serial]
async fn test_password_reset_authenticated_weak_password() {
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Create user and login
    let user_email = "weak-auth-test@example.com";
    let current_password = "currentpassword123";

    DbFixtures::create_user_with_email_password(
        &fixture.db(),
        user_email,
        current_password,
        Some("weakauthuser"),
    )
    .await
    .expect("Failed to create test user");

    // Login to get access token
    let login_response = client
        .post(format!("{base_url}/api/auth/login"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "email": user_email,
            "password": current_password
        }))
        .send()
        .await
        .expect("Failed to send login request");

    let login_body: Value = login_response
        .json()
        .await
        .expect("Failed to parse login response");
    let access_token = login_body["access_token"].as_str().unwrap();

    let weak_passwords = vec!["", "123", "1234567"];

    for weak_password in weak_passwords {
        let response = client
            .post(format!("{base_url}/api/auth/password/reset-authenticated"))
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&json!({
                "current_password": current_password,
                "new_password": weak_password
            }))
            .send()
            .await
            .expect("Failed to send request");

        // ✅ Should return 422 for weak password
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "Should reject weak password: '{weak_password}'"
        );
    }
}

// ============================================================================
// 🔄 Edge Case and Security Tests
// ============================================================================

/// Tests password reset workflow when multiple tokens exist for the same user.
/// Verifies behavior when a user has generated multiple reset tokens.
/// This documents the system's handling of concurrent reset token scenarios.
#[tokio::test]
#[serial]
async fn test_password_reset_workflow_multiple_tokens() {
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Create user
    let user_email = "multiple-tokens@example.com";
    let user_password = "oldpassword123";

    let user = DbFixtures::create_user_with_email_password(
        &fixture.db(),
        user_email,
        user_password,
        Some("multiuser"),
    )
    .await
    .expect("Failed to create test user");

    // Generate multiple reset tokens for the same user
    let token1_fixture = DbFixtures::password_reset_token()
        .valid(user.id())
        .commit(fixture.db())
        .await
        .expect("Failed to create first token");
    let token1 = token1_fixture.token();

    let token2_fixture = DbFixtures::password_reset_token()
        .valid(user.id())
        .commit(fixture.db())
        .await
        .expect("Failed to create second token");
    let token2 = token2_fixture.token();

    // Both tokens should be valid initially
    for token in [&token1, &token2] {
        let response = client
            .post(format!("{base_url}/api/auth/password/reset-validate"))
            .header("Content-Type", "application/json")
            .json(&json!({
                "token": token
            }))
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status(), StatusCode::OK);
    }

    // Use the first token to reset password
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-confirm"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "token": token1,
            "new_password": "newpassword456"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    // The second token should still be usable (depending on business logic)
    // This test documents the current behavior - modify if business rules change
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-confirm"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "token": token2,
            "new_password": "anothernewpassword789"
        }))
        .send()
        .await
        .expect("Failed to send request");

    // This could be either 200 (if multiple tokens allowed) or 400 (if all tokens invalidated)
    // Document the actual behavior here
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Tests password reset request with case-insensitive email matching.
/// Verifies that email lookup works regardless of case (upper/lower).
/// This ensures consistent behavior for users who type emails with different casing.
#[tokio::test]
#[serial]
async fn test_password_reset_case_insensitive_email() {
    let (fixture, base_url, client) = Box::pin(setup_test_server())
        .await
        .expect("Failed to setup test server");

    // Create user with lowercase email
    let user_email = "case-test@example.com";
    let user_password = "securepassword123";

    DbFixtures::create_user_with_email_password(
        &fixture.db(),
        user_email,
        user_password,
        Some("caseuser"),
    )
    .await
    .expect("Failed to create test user");

    // Request password reset with uppercase email
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-request"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "email": "CASE-TEST@EXAMPLE.COM"
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should handle case-insensitive email lookup
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("reset link has been sent"));
}

/// VERIFIES: Password reset request for existing user with email/password SHOULD publish `PasswordResetRequested` event
#[tokio::test]
#[serial]
async fn test_password_reset_request_event_published_for_valid_user() {
    let (fixture, base_url, client, mock_publisher) =
        Box::pin(setup_test_server_with_mock_events())
            .await
            .expect("Failed to setup test server with mock events");

    // Clear any existing events
    mock_publisher.clear_events();

    // Create a user with email/password authentication
    let user_email = "event-test@example.com";
    let user_password = "securepassword123";

    DbFixtures::create_user_with_email_password(
        &fixture.db(),
        user_email,
        user_password,
        Some("eventuser"),
    )
    .await
    .expect("Failed to create test user");

    // Request password reset
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-request"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "email": user_email
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 for security
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("reset link has been sent"));

    // 🎯 VERIFY: Event WAS published
    let events = mock_publisher
        .wait_for_event_type("password_reset_requested", Duration::from_secs(5))
        .await;
    assert!(
        !events.is_empty(),
        "PasswordResetRequested event should be published for valid user with password authentication");

    assert_eq!(
        events.len(),
        1,
        "Exactly one PasswordResetRequested event should be published"
    );
}

/// VERIFIES: Password reset request for non-existent email should NOT publish `PasswordResetRequested` event
#[tokio::test]
#[serial]
async fn test_password_reset_request_no_event_for_nonexistent_email() {
    let (_fixture, base_url, client, mock_publisher) =
        Box::pin(setup_test_server_with_mock_events())
            .await
            .expect("Failed to setup test server with mock events");

    // Clear any existing events
    mock_publisher.clear_events();

    // Request password reset for non-existent email
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-request"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "email": "nonexistent@example.com"
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 for security (same as valid case)
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("reset link has been sent"));

    // 🎯 VERIFY: Event was NOT published
    assert!(
        !mock_publisher.has_password_reset_requested_event(),
        "PasswordResetRequested event should NOT be published for non-existent email"
    );

    assert_eq!(
        mock_publisher.get_event_count(),
        0,
        "No events should be published for non-existent email"
    );
}

/// VERIFIES: Password reset request for OAuth-only user should NOT publish `PasswordResetRequested` event
#[tokio::test]
#[serial]
async fn test_password_reset_request_no_event_for_oauth_only_user() {
    let (fixture, base_url, client, mock_publisher) =
        Box::pin(setup_test_server_with_mock_events())
            .await
            .expect("Failed to setup test server with mock events");

    // Clear any existing events
    mock_publisher.clear_events();

    // Create a user that only has OAuth authentication (no password)
    let (_user, _provider_token) = DbFixtures::create_user_with_oauth_provider(
        &fixture.db(),
        "oauth-only-event@example.com",
        "oauthuser",
        "github",
    )
    .await
    .expect("Failed to create OAuth user");

    // Request password reset for OAuth-only user
    let response = client
        .post(format!("{base_url}/api/auth/password/reset-request"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "email": "oauth-only-event@example.com"
        }))
        .send()
        .await
        .expect("Failed to send request");

    // ✅ Should return 200 for security (same as valid case)
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse response");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("reset link has been sent"));

    // 🎯 VERIFY: Event was NOT published
    assert!(
        !mock_publisher.has_password_reset_requested_event(),
        "PasswordResetRequested event should NOT be published for OAuth-only user"
    );

    assert_eq!(
        mock_publisher.get_event_count(),
        0,
        "No events should be published for OAuth-only user"
    );
}
