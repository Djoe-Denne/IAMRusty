//! STORY-004: prove enrollment and password login on the real IAM HTTP/Postgres boundary.

mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
mod fixtures;
#[path = "support/totp_app_oracle.rs"]
mod totp_app_oracle;

use anyhow::{ensure, Context as _};
use common::{setup_test_replica, setup_test_server_with_totp, TestFixture};
use fixtures::DbFixtures;
use iam_domain::entity::token::RefreshToken;
use reqwest::{header, Client, Response, StatusCode};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serde_json::{json, Value};
use std::time::Duration;
use totp_app_oracle::{clock, code, seed_from_uri, PERIOD};
use uuid::Uuid;

const ENROLLMENT: &str = "/api/me/totp/enrollment";
const EMAIL: &str = "totp-enrollment@example.com";
const PASSWORD: &str = "IsolatedTotpTest123!";
const WRONG_PASSWORD: &str = "IsolatedWrongPassword123!";

#[tokio::test]
#[serial_test::serial]
async fn totp_enrollment_is_optional_and_rejected_confirmation_stays_inactive() -> anyhow::Result<()>
{
    let (fixture, base, client) = Box::pin(setup_test_server_with_totp())
        .await
        .map_err(|_| anyhow::anyhow!("could not start the IAM test fixture"))?;
    fixture_cleanup::run(
        &fixture,
        Box::pin(exercise_enrollment(&fixture, &base, &client)),
    )
    .await
}

async fn exercise_enrollment(
    fixture: &TestFixture,
    base: &str,
    client: &Client,
) -> anyhow::Result<()> {
    let owner = DbFixtures::create_user_with_email_password(
        &fixture.db(),
        EMAIL,
        PASSWORD,
        Some("totpenrollment"),
    )
    .await?;
    let endpoint = format!("{base}{ENROLLMENT}");
    let access = password_session(client, base, owner.id()).await?;
    let (id, seed) = begin_enrollment(client, &endpoint, &access).await?;

    let response = client.post(&endpoint).send().await?;
    ensure!(
        response.status() == StatusCode::UNAUTHORIZED,
        "E1: enrollment must require an authenticated platform account (got {})",
        response.status()
    );
    let _incomplete = password_session(client, base, owner.id()).await?;
    let step = usable_step(None).await?;
    let invalid = invalid_code(&seed, step);
    let rejected = confirm(client, &endpoint, &access, id, PASSWORD, &invalid).await?;
    ensure!(
        rejected.status() == StatusCode::UNAUTHORIZED,
        "E2: a well-formed but invalid app code must not activate TOTP (got {})",
        rejected.status()
    );
    let _after_invalid = password_session(client, base, owner.id()).await?;

    let valid = code(&seed, usable_step(None).await?);
    let rejected = confirm(client, &endpoint, &access, id, WRONG_PASSWORD, &valid).await?;
    ensure!(
        rejected.status() == StatusCode::UNAUTHORIZED,
        "E1: an authentic app code does not replace current-password proof (got {})",
        rejected.status()
    );
    let _after_wrong_password = password_session(client, base, owner.id()).await?;

    let response = client
        .delete(format!("{endpoint}/{id}"))
        .bearer_auth(&access)
        .send()
        .await?;
    ensure!(
        response.status() == StatusCode::NO_CONTENT,
        "E2: an authenticated owner can cancel their pending enrollment (got {})",
        response.status()
    );
    let valid_after_cancel = code(&seed, usable_step(None).await?);
    let cancelled = confirm(
        client,
        &endpoint,
        &access,
        id,
        PASSWORD,
        &valid_after_cancel,
    )
    .await?;
    ensure!(
        cancelled.status() == StatusCode::NOT_FOUND,
        "E2: a cancelled enrollment cannot activate even with its valid app code (got {})",
        cancelled.status()
    );
    let _after_cancel = password_session(client, base, owner.id()).await?;
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn totp_activation_gates_password_sessions_and_consumes_each_app_step_once(
) -> anyhow::Result<()> {
    let (fixture, base, client) = Box::pin(setup_test_server_with_totp())
        .await
        .map_err(|_| anyhow::anyhow!("could not start the IAM test fixture"))?;
    fixture_cleanup::run(
        &fixture,
        Box::pin(exercise_session_gate(&fixture, &base, &client)),
    )
    .await
}

async fn exercise_session_gate(
    fixture: &TestFixture,
    base: &str,
    client: &Client,
) -> anyhow::Result<()> {
    let db = fixture.db();
    let owner =
        DbFixtures::create_user_with_email_password(&db, EMAIL, PASSWORD, Some("totpsession"))
            .await?;
    let owner_id = owner.id();
    let access = password_session(client, base, owner_id).await?;
    let original_count = session_count(&db, owner_id).await?;
    ensure!(
        original_count == 1,
        "one password-only control session must have persisted"
    );
    let endpoint = format!("{base}{ENROLLMENT}");
    let (id, seed) = begin_enrollment(client, &endpoint, &access).await?;
    let wrong_before = rejected_login(login(client, base, WRONG_PASSWORD, None).await?).await?;
    ensure!(
        wrong_before["error"]["error_code"] == "invalid_credentials",
        "password control"
    );
    let (replica, replica_client) = Box::pin(setup_test_replica(fixture))
        .await
        .map_err(|_| anyhow::anyhow!("could not start the second IAM test listener"))?;

    let activation_step = usable_step(None).await?;
    let first_code = code(&seed, activation_step);
    let activated = confirm(client, &endpoint, &access, id, PASSWORD, &first_code).await?;
    ensure!(
        activated.status() == StatusCode::NO_CONTENT,
        "E1: correct current password and independently generated first code must activate TOTP (got {})",
        activated.status()
    );
    let _ = rejected_login(login(&replica_client, &replica, PASSWORD, Some(&first_code)).await?)
        .await?;

    let challenge = rejected_login(login(client, base, PASSWORD, None).await?).await?;
    ensure!(
        challenge["error"]["error_code"] == "totp_required",
        "E3: correct password for an enrolled account must get the tokenless TOTP challenge"
    );
    let wrong_after = rejected_login(login(client, base, WRONG_PASSWORD, None).await?).await?;
    ensure!(
        wrong_before == wrong_after,
        "wrong password must not disclose enrollment state"
    );
    let mut step = usable_step(Some(activation_step)).await?;
    let mut fresh_code = code(&seed, step);
    // Adjacent steps can coincidentally display the same six digits.
    if fresh_code == first_code {
        step = usable_step(Some(step)).await?;
        fresh_code = code(&seed, step);
        ensure!(
            fresh_code != first_code,
            "distinct app-step control was not available"
        );
    }
    let invalid = invalid_code(&seed, step);
    let _ = rejected_login(login(client, base, PASSWORD, Some(&invalid)).await?).await?;
    let wrong_with_code =
        rejected_login(login(client, base, WRONG_PASSWORD, Some(&fresh_code)).await?).await?;
    ensure!(
        wrong_before == wrong_with_code,
        "a valid app code is not a substitute for the password"
    );
    ensure!(
        session_count(&db, owner_id).await? == original_count,
        "E3: activation, password-only challenge and rejected proofs must create no refresh session"
    );

    // Use two runtime roots to distinguish DB-atomic consumption from process-local replay guards.
    let (left, right) = tokio::try_join!(
        login(client, base, PASSWORD, Some(&fresh_code)),
        login(&replica_client, &replica, PASSWORD, Some(&fresh_code))
    )?;
    let (accepted, denied) = if left.status() == StatusCode::OK {
        (left, right)
    } else {
        (right, left)
    };
    let session = successful_login(accepted, owner_id).await?;
    let _ = rejected_login(denied).await?;
    let _ = rejected_login(login(client, base, PASSWORD, Some(&fresh_code)).await?).await?;
    ensure!(
        session_count(&db, owner_id).await? == original_count + 1,
        "E3: racing copies of the same app step must persist exactly one new session"
    );
    let refresh = session["refresh_token"]
        .as_str()
        .context("login refresh absent")?;
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT COUNT(*) AS n FROM refresh_tokens WHERE user_id = $1 AND token = $2 AND is_valid AND expires_at > NOW()",
            [owner_id.into(), RefreshToken::hash_token(refresh).into()],
        ))
        .await?
        .context("issued session count absent")?;
    ensure!(
        row.try_get::<i64>("", "n")? == 1,
        "accepted refresh must be the actual persisted session"
    );
    let access = session["access_token"]
        .as_str()
        .context("login access absent")?;
    let me = client
        .get(format!("{base}/api/me"))
        .bearer_auth(access)
        .send()
        .await?;
    ensure!(
        me.status() == StatusCode::OK,
        "accepted login must provide actual protected access"
    );
    let profile: Value = me
        .json()
        .await
        .map_err(|_| anyhow::anyhow!("invalid protected profile JSON"))?;
    ensure!(
        profile["id"]
            .as_str()
            .and_then(|value| Uuid::parse_str(value).ok())
            == Some(owner_id),
        "accepted access credential must identify the enrolled owner"
    );
    Ok(())
}

async fn begin_enrollment(
    client: &Client,
    endpoint: &str,
    access: &str,
) -> anyhow::Result<(Uuid, [u8; 20])> {
    let begin = client.post(endpoint).bearer_auth(access).send().await?;
    ensure!(
        begin.status() == StatusCode::CREATED,
        "E1: signed-in platform account must be able to start TOTP enrollment (got {})",
        begin.status()
    );
    ensure!(
        begin
            .headers()
            .get(header::CACHE_CONTROL)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.split(',').any(|part| part.trim() == "no-store")),
        "the provisioning response must not be cached"
    );
    let pending: Value = begin
        .json()
        .await
        .map_err(|_| anyhow::anyhow!("invalid provisioning JSON"))?;
    let id = pending["enrollment_id"]
        .as_str()
        .and_then(|value| Uuid::parse_str(value).ok())
        .context("provisioning response must identify the enrollment")?;
    let uri = pending["otpauth_uri"]
        .as_str()
        .context("provisioning URI absent")?;
    Ok((id, seed_from_uri(uri)?))
}

async fn confirm(
    client: &Client,
    endpoint: &str,
    access: &str,
    id: Uuid,
    password: &str,
    code: &str,
) -> reqwest::Result<Response> {
    client
        .post(format!("{endpoint}/confirm"))
        .bearer_auth(access)
        .json(&json!({
            "enrollment_id": id, "code": code, "current_password": password,
        }))
        .send()
        .await
}

async fn login(
    client: &Client,
    base: &str,
    password: &str,
    totp_code: Option<&str>,
) -> reqwest::Result<Response> {
    let mut body = json!({ "email": EMAIL, "password": password });
    if let Some(totp_code) = totp_code {
        body["totp_code"] = json!(totp_code);
    }
    client
        .post(format!("{base}/api/auth/login"))
        .json(&body)
        .send()
        .await
}

async fn rejected_login(response: Response) -> anyhow::Result<Value> {
    fn contains_credential(value: &Value) -> bool {
        match value {
            Value::Object(fields) => fields.iter().any(|(name, value)| {
                matches!(
                    name.as_str(),
                    "access_token" | "refresh_token" | "registration_token" | "token"
                ) || contains_credential(value)
            }),
            Value::Array(items) => items.iter().any(contains_credential),
            _ => false,
        }
    }
    ensure!(
        response.status() == StatusCode::UNAUTHORIZED,
        "E3: incomplete, invalid or replayed proof must be rejected (got {})",
        response.status()
    );
    ensure!(
        !response.headers().contains_key(header::SET_COOKIE),
        "rejected login must not establish a session cookie"
    );
    let body: Value = response
        .json()
        .await
        .map_err(|_| anyhow::anyhow!("invalid login rejection JSON"))?;
    ensure!(
        !contains_credential(&body),
        "rejected login must not expose any credential"
    );
    Ok(body)
}

async fn successful_login(response: Response, owner: Uuid) -> anyhow::Result<Value> {
    ensure!(
        response.status() == StatusCode::OK,
        "fully authenticated login must succeed (got {})",
        response.status()
    );
    let body: Value = response
        .json()
        .await
        .map_err(|_| anyhow::anyhow!("invalid login success JSON"))?;
    ensure!(
        body["user"]["id"]
            .as_str()
            .and_then(|value| Uuid::parse_str(value).ok())
            == Some(owner)
            && body["refresh_token"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && body["access_token"]
                .as_str()
                .is_some_and(|value| !value.is_empty()),
        "successful login must return the expected account and both credentials"
    );
    Ok(body)
}

async fn password_session(client: &Client, base: &str, owner: Uuid) -> anyhow::Result<String> {
    let response = login(client, base, PASSWORD, None).await?;
    let body = successful_login(response, owner)
        .await
        .context("E2/E3 password-only control")?;
    body["access_token"]
        .as_str()
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .context("password login must return an access token")
}

async fn session_count(db: &DatabaseConnection, owner: Uuid) -> anyhow::Result<i64> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT COUNT(*) AS n FROM refresh_tokens WHERE user_id = $1",
            [owner.into()],
        ))
        .await?
        .context("account session count absent")?;
    Ok(row.try_get("", "n")?)
}

fn invalid_code(seed: &[u8; 20], step: u64) -> String {
    let nearby: Vec<_> = (step.saturating_sub(2)..=step + 2)
        .map(|t| code(seed, t))
        .collect();
    (0..=nearby.len())
        .map(|n| format!("{n:06}"))
        .find(|candidate| !nearby.contains(candidate))
        .expect("more invalid candidates than nearby app codes")
}

/// Leave headroom for HTTP/password work without assuming the service accepts a prior step.
async fn usable_step(after: Option<u64>) -> anyhow::Result<u64> {
    tokio::time::timeout(Duration::from_secs(PERIOD + 5), async {
        loop {
            let now = clock()?;
            let step = now.as_secs() / PERIOD;
            if now.as_secs() % PERIOD <= 15 && after.is_none_or(|last| step > last) {
                return Ok(step);
            }
            let next = Duration::from_secs((step + 1) * PERIOD);
            tokio::time::sleep(next.saturating_sub(now)).await;
        }
    })
    .await
    .context("could not obtain a fresh app step")?
}

#[path = "support/totp_regressions.rs"]
mod totp_regressions;
