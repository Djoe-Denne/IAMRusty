//! STORY-004 follow-up: defensive writer and HTTP boundary regressions.

use super::{
    begin_enrollment, clock, code, confirm, invalid_code, login, password_session, session_count,
    successful_login, ENROLLMENT, PASSWORD, PERIOD,
};
use crate::{common, fixture_cleanup, fixtures::DbFixtures};
use anyhow::{ensure, Context as _};
use reqwest::{header, Client, Response, StatusCode};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serde_json::{json, Value};
use uuid::Uuid;

const ATTEMPT_LIMIT: i32 = 5;

#[tokio::test]
#[serial_test::serial]
async fn active_setup_cannot_restart_or_cancel_and_expired_setup_cannot_activate(
) -> anyhow::Result<()> {
    let (fixture, base, client) = Box::pin(common::setup_test_server())
        .await
        .map_err(|_| anyhow::anyhow!("could not start the IAM test fixture"))?;
    fixture_cleanup::run(&fixture, Box::pin(async {
        let db = fixture.db();
        let owner = owner_id(&db, "totpregressionstate").await?;
        let access = password_session(&client, &base, owner).await?;
        let endpoint = format!("{base}{ENROLLMENT}");
        let (expired, expired_seed) = begin_enrollment(&client, &endpoint, &access).await?;
        let malformed = "totp-malformed-fixture-sentinel";
        assert_denial(
            confirm(&client, &endpoint, &access, expired, PASSWORD, malformed).await?,
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_totp_confirmation",
            &[malformed],
        ).await?;
        let arranged = db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE totp_credentials SET expires_at = NOW() - INTERVAL '1 minute' \
             WHERE user_id = $1 AND enrollment_id = $2 AND last_used_step IS NULL AND expires_at > NOW()",
            [owner.into(), expired.into()],
        )).await?;
        ensure!(arranged.rows_affected() == 1, "T1 fixture must expire the actual pending enrollment");

        let expired_code = code(&expired_seed, current_step()?);
        assert_denial(
            confirm(&client, &endpoint, &access, expired, PASSWORD, &expired_code).await?,
            StatusCode::NOT_FOUND,
            "enrollment_not_found",
            &[&expired_code],
        ).await?;
        let access = password_session(&client, &base, owner).await?;

        let (active, _) = activate(&client, &endpoint, &access).await?;
        let baseline = session_count(&db, owner).await?;
        assert_denial(
            client.post(&endpoint).bearer_auth(&access).send().await?,
            StatusCode::CONFLICT,
            "enrollment_not_available",
            &[],
        ).await?;
        assert_denial(
            client.delete(format!("{endpoint}/{active}")).bearer_auth(&access).send().await?,
            StatusCode::NOT_FOUND,
            "enrollment_not_found",
            &[],
        ).await?;
        assert_denial(
            login(&client, &base, PASSWORD, None).await?,
            StatusCode::UNAUTHORIZED,
            "totp_required",
            &[],
        ).await?;
        let retained = db.query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT last_used_step FROM totp_credentials \
             WHERE user_id = $1 AND enrollment_id = $2 AND last_used_step IS NOT NULL",
            [owner.into(), active.into()],
        )).await?;
        ensure!(retained.is_some(), "T1 rejected mutations must retain the same active enrollment");
        ensure!(session_count(&db, owner).await? == baseline, "T1 no password-only session after rejected mutations");
        Ok(())
    })).await
}

#[tokio::test]
#[serial_test::serial]
async fn next_totp_submission_at_the_resolved_account_limit_cannot_issue_session(
) -> anyhow::Result<()> {
    let (fixture, base, client) = Box::pin(common::setup_test_server())
        .await
        .map_err(|_| anyhow::anyhow!("could not start the IAM test fixture"))?;
    fixture_cleanup::run(&fixture, Box::pin(async {
        let db = fixture.db();
        let owner = owner_id(&db, "totpregressionbudget").await?;
        let alias = "totp-budget-alias@example.com";
        DbFixtures::user_email()
            .user_id(owner)
            .email(alias)
            .with_verified(true)
            .commit(db.clone())
            .await?;
        let alias_login = login_as(&client, &base, alias, None).await?;
        let _ = successful_login(alias_login, owner).await?;
        let access = password_session(&client, &base, owner).await?;
        let (active, seed) = activate(&client, &format!("{base}{ENROLLMENT}"), &access).await?;
        let baseline = session_count(&db, owner).await?;

        // Arrange an earlier activation and a near-limit account. No wait for a
        // time step and no burst of requests to fill the rate window.
        let arranged = db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE totp_credentials \
             SET last_used_step = 0, login_window_started_at = NOW(), login_attempts = $3 \
             WHERE user_id = $1 AND enrollment_id = $2 AND last_used_step IS NOT NULL AND login_attempts = 0",
            [owner.into(), active.into(), (ATTEMPT_LIMIT - 1).into()],
        )).await?;
        ensure!(arranged.rows_affected() == 1, "T2 fixture must position the actual active account below its limit");
        let invalid = invalid_code(&seed, current_step()?);
        assert_denial(
            login(&client, &base, PASSWORD, Some(&invalid)).await?,
            StatusCode::UNAUTHORIZED,
            "invalid_credentials",
            &[&invalid],
        ).await?;
        assert_attempt_state(&db, owner, active).await?;
        ensure!(session_count(&db, owner).await? == baseline, "T2 invalid admitted submission must be sessionless");

        let fresh = code(&seed, current_step()?);
        assert_denial(
            login_as(&client, &base, alias, Some(&fresh)).await?,
            StatusCode::TOO_MANY_REQUESTS,
            "totp_throttled",
            &[&fresh],
        ).await?;
        assert_attempt_state(&db, owner, active).await?;
        ensure!(session_count(&db, owner).await? == baseline, "T2 throttled valid code must be sessionless");
        Ok(())
    })).await
}

#[tokio::test]
#[serial_test::serial]
async fn listener_without_a_totp_key_cannot_enroll_or_issue_a_protected_password_session(
) -> anyhow::Result<()> {
    let (fixture, base, client) = Box::pin(common::setup_test_server())
        .await
        .map_err(|_| anyhow::anyhow!("could not start the IAM test fixture"))?;
    fixture_cleanup::run(
        &fixture,
        Box::pin(async {
            let db = fixture.db();
            let protected = owner_id(&db, "totpregressionprotected").await?;
            let access = password_session(&client, &base, protected).await?;
            let (id, seed) = activate(&client, &format!("{base}{ENROLLMENT}"), &access).await?;
            let arranged = db
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "UPDATE totp_credentials SET last_used_step = 0 \
             WHERE user_id = $1 AND enrollment_id = $2 AND last_used_step > 0",
                    [protected.into(), id.into()],
                ))
                .await?;
            ensure!(
                arranged.rows_affected() == 1,
                "T3 fixture must retain an active factor with a fresh step available"
            );
            let protected_baseline = session_count(&db, protected).await?;
            let eligible_email = "totp-no-key-eligible@example.com";
            let eligible = DbFixtures::create_user_with_email_password(
                &db,
                eligible_email,
                PASSWORD,
                Some("totpregressioneligible"),
            )
            .await?
            .id();
            let (no_key, no_key_client) =
                Box::pin(common::setup_test_replica_without_totp_key(&fixture))
                    .await
                    .map_err(|_| {
                        anyhow::anyhow!("could not start the keyless IAM test listener")
                    })?;
            // Positive control: the keyless listener is live, on the same writer,
            // and can issue an ordinary session for the unprotected account.
            let response = login_as(&no_key_client, &no_key, eligible_email, None).await?;
            let session = successful_login(response, eligible).await?;
            let access = session["access_token"]
                .as_str()
                .context("unprotected access token absent")?;
            ensure!(
                session_count(&db, eligible).await? == 1,
                "T3 unprotected control session must persist"
            );
            assert_denial(
                no_key_client
                    .post(format!("{no_key}{ENROLLMENT}"))
                    .bearer_auth(access)
                    .send()
                    .await?,
                StatusCode::SERVICE_UNAVAILABLE,
                "totp_unavailable",
                &[],
            )
            .await?;
            let unexpected = db
                .query_one(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT user_id FROM totp_credentials WHERE user_id = $1",
                    [eligible.into()],
                ))
                .await?;
            ensure!(
                unexpected.is_none(),
                "T3 keyless listener must not create pending credentials"
            );
            assert_denial(
                login(&no_key_client, &no_key, PASSWORD, None).await?,
                StatusCode::SERVICE_UNAVAILABLE,
                "totp_unavailable",
                &[],
            )
            .await?;
            let app_code = code(&seed, current_step()?);
            assert_denial(
                login(&no_key_client, &no_key, PASSWORD, Some(&app_code)).await?,
                StatusCode::SERVICE_UNAVAILABLE,
                "totp_unavailable",
                &[&app_code],
            )
            .await?;
            ensure!(
                session_count(&db, protected).await? == protected_baseline,
                "T3 keyless listener must not persist a protected session"
            );
            let response = login(&client, &base, PASSWORD, Some(&app_code)).await?;
            let _ = successful_login(response, protected).await?;
            ensure!(
                session_count(&db, protected).await? == protected_baseline + 1,
                "T3 the keyed listener must still accept the same unconsumed proof"
            );
            Ok(())
        }),
    )
    .await
}

async fn owner_id(db: &std::sync::Arc<DatabaseConnection>, username: &str) -> anyhow::Result<Uuid> {
    Ok(
        DbFixtures::create_user_with_email_password(db, super::EMAIL, PASSWORD, Some(username))
            .await?
            .id(),
    )
}

fn current_step() -> anyhow::Result<u64> {
    Ok(clock()?.as_secs() / PERIOD)
}

async fn activate(
    client: &Client,
    endpoint: &str,
    access: &str,
) -> anyhow::Result<(Uuid, [u8; 20])> {
    let (id, seed) = begin_enrollment(client, endpoint, access).await?;
    let app_code = code(&seed, current_step()?);
    let response = confirm(client, endpoint, access, id, PASSWORD, &app_code).await?;
    ensure!(
        response.status() == StatusCode::NO_CONTENT,
        "fixture activation failed (got {})",
        response.status()
    );
    Ok((id, seed))
}

async fn login_as(
    client: &Client,
    base: &str,
    email: &str,
    code: Option<&str>,
) -> reqwest::Result<Response> {
    let mut body = json!({ "email": email, "password": PASSWORD });
    if let Some(code) = code {
        body["totp_code"] = json!(code);
    }
    client
        .post(format!("{base}/api/auth/login"))
        .json(&body)
        .send()
        .await
}

async fn assert_attempt_state(
    db: &DatabaseConnection,
    owner: Uuid,
    id: Uuid,
) -> anyhow::Result<()> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT login_attempts, last_used_step FROM totp_credentials \
         WHERE user_id = $1 AND enrollment_id = $2",
            [owner.into(), id.into()],
        ))
        .await?
        .context("T2 writer attempt state absent")?;
    ensure!(
        row.try_get::<i32>("", "login_attempts")? == ATTEMPT_LIMIT
            && row.try_get::<i64>("", "last_used_step")? == 0,
        "T2 account must reach and stay at its limit without consuming an app step"
    );
    Ok(())
}

async fn assert_denial(
    response: Response,
    status: StatusCode,
    error_code: &str,
    sentinels: &[&str],
) -> anyhow::Result<()> {
    ensure!(
        response.status() == status,
        "{error_code}: expected {status}, got {}",
        response.status()
    );
    ensure!(
        !response.headers().contains_key(header::SET_COOKIE),
        "denial cannot set a cookie"
    );
    let body = response
        .text()
        .await
        .map_err(|_| anyhow::anyhow!("could not read denial body"))?;
    ensure!(
        !body.contains(PASSWORD)
            && !body.contains("otpauth://")
            && sentinels.iter().all(|sentinel| !body.contains(*sentinel)),
        "denial must not reflect credential material"
    );
    let parsed: Value =
        serde_json::from_str(&body).map_err(|_| anyhow::anyhow!("invalid denial envelope"))?;
    ensure!(
        parsed.as_object().is_some_and(|fields| fields.len() == 1)
            && parsed["error"]
                .as_object()
                .is_some_and(|fields| fields.len() == 3)
            && parsed["error"]["error_code"] == error_code
            && parsed["error"]["status"] == status.as_u16()
            && parsed["error"]["message"].is_string(),
        "denial must be a closed error envelope without credentials"
    );
    Ok(())
}
