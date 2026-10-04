//! C1: use actual inbound flows; never insert refresh tokens as test arrangement.
mod common;
#[path = "support/fixture_cleanup.rs"]
mod fixture_cleanup;
#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "support/owned_task.rs"]
mod owned_task;
mod utils;
#[path = "support/writer_barrier.rs"]
mod writer_barrier;

use common::setup_test_server;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

async fn assert_persisted(db: &DatabaseConnection, token: &str) {
    let hash = iam_domain::entity::token::RefreshToken::hash_token(token);
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT COUNT(*) AS count FROM refresh_tokens WHERE token = $1 AND is_valid AND expires_at > NOW()", [hash.into()]))
        .await.expect("read persisted session").expect("count row");
    assert_eq!(
        row.try_get::<i64>("", "count").expect("count"),
        1,
        "issued refresh must be persisted exactly once"
    );
}

#[tokio::test]
#[serial]
async fn registration_and_password_login_refresh_are_real_persisted_sessions() {
    let (fixture, base, client) = setup_test_server().await.expect("IAM harness");
    fixture_cleanup::run(&fixture, async {
        let email = format!("session-{}@example.com", Uuid::new_v4());
        let username = format!("s{}", &Uuid::new_v4().simple().to_string()[..15]);
        let password = "SessionTest1a!";
        let signup = client
            .post(format!("{base}/api/auth/signup"))
            .json(&json!({"email": email, "password": password}))
            .send()
            .await
            .expect("signup");
        assert_eq!(signup.status(), 202);
        let signup: Value = signup.json().await.expect("signup JSON");
        let db = fixture.db();
        let verified = db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE user_emails SET is_verified=true WHERE email=$1",
                [email.clone().into()],
            ))
            .await
            .expect("verified-email arrangement before session completion");
        assert_eq!(verified.rows_affected(), 1);
        let complete = client
            .post(format!("{base}/api/auth/complete-registration"))
            .json(
                &json!({"registration_token": signup["registration_token"], "username": username}),
            )
            .send()
            .await
            .expect("complete registration");
        assert_eq!(complete.status(), 200);
        let complete: Value = complete.json().await.expect("registration JSON");
        let registration_refresh = complete["refresh_token"]
            .as_str()
            .expect("registration refresh");
        assert_persisted(db.as_ref(), registration_refresh).await;
        let rotated = client
            .post(format!("{base}/api/token/refresh"))
            .json(&json!({"refresh_token": registration_refresh}))
            .send()
            .await
            .expect("registration refresh request");
        assert_eq!(rotated.status(), 200);
        let rotated: Value = rotated.json().await.expect("rotation JSON");
        assert_persisted(
            db.as_ref(),
            rotated["refresh_token"].as_str().expect("replacement"),
        )
        .await;
        // Email verification is arrangement, not a fabricated authentication session.
        let result = db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE user_emails SET is_verified = true WHERE email = $1",
                [email.clone().into()],
            ))
            .await
            .expect("verify registered email");
        assert_eq!(result.rows_affected(), 1);
        let login = client
            .post(format!("{base}/api/auth/login"))
            .json(&json!({"email": email, "password": password}))
            .send()
            .await
            .expect("password login");
        assert_eq!(login.status(), 200);
        let login: Value = login.json().await.expect("login JSON");
        let refresh = login["refresh_token"].as_str().expect("password refresh");
        assert_persisted(db.as_ref(), refresh).await;
        let refresh_response = client
            .post(format!("{base}/api/token/refresh"))
            .json(&json!({"refresh_token": refresh}))
            .send()
            .await
            .expect("password refresh request");
        assert_eq!(refresh_response.status(), 200);
        let renewed: Value = refresh_response.json().await.expect("refresh JSON");
        assert_persisted(
            db.as_ref(),
            renewed["refresh_token"].as_str().expect("replacement"),
        )
        .await;
        assert_eq!(
            client
                .post(format!("{base}/api/token/refresh"))
                .json(&json!({"refresh_token": refresh}))
                .send()
                .await
                .expect("replay request")
                .status(),
            401
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn public_password_reset_and_refresh_rotation_linearize_without_a_surviving_old_chain() {
    let (fixture, base, client) = setup_test_server()
        .await
        .expect("IAM HTTP/Postgres harness");
    fixture_cleanup::run(&fixture, async {
    let db = fixture.db();
    let email = "causal-reset-rotate@example.com";
    let old_password = "CausalOld1a!";
    let new_password = "CausalNew2b!";
    let user = fixtures::DbFixtures::create_user_with_email_password(
        &db,
        email,
        old_password,
        Some("causalreset"),
    )
    .await
    .expect("real password account");
    let login = client
        .post(format!("{base}/api/auth/login"))
        .json(&json!({"email": email, "password": old_password}))
        .send()
        .await
        .expect("login");
    assert_eq!(login.status(), 200);
    let login: Value = login.json().await.expect("login JSON");
    let refresh = login["refresh_token"]
        .as_str()
        .expect("issued refresh")
        .to_owned();
    assert_persisted(db.as_ref(), &refresh).await;
    let reset = fixtures::DbFixtures::password_reset_token()
        .valid(user.id())
        .commit(db.clone())
        .await
        .expect("reset authorization fixture");
    let reset_token = reset.token().to_owned();
    let separate = writer_barrier::independent_writer(db.as_ref()).await;
    let (held, holder) = writer_barrier::hold(&separate, "users", user.id()).await;
    let reset_request = {
        let client = client.clone();
        let base = base.clone();
        owned_task::spawn(async move {
            client
                .post(format!("{base}/api/auth/password/reset-confirm"))
                .json(&json!({"token": reset_token, "new_password": new_password}))
                .send()
                .await
                .expect("HTTP reset")
        })
    };
    let rotate_request = {
        let client = client.clone();
        let base = base.clone();
        let refresh = refresh.clone();
        owned_task::spawn(async move {
            client
                .post(format!("{base}/api/token/refresh"))
                .json(&json!({"refresh_token": refresh}))
                .send()
                .await
                .expect("HTTP rotation")
        })
    };
    writer_barrier::wait_for_blocked(db.as_ref(), holder, 2).await;
    held.commit()
        .await
        .expect("release real HTTP writer barrier");
    assert_eq!(reset_request.await.expect("reset task").status(), 200);
    let rotated = rotate_request.await.expect("rotation task");
    assert!(
        matches!(rotated.status().as_u16(), 200 | 401),
        "rotation is ordered before or after reset, never 500"
    );
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT (SELECT COUNT(*) FROM refresh_tokens WHERE user_id=$1) AS refreshes, (SELECT COUNT(*) FROM password_reset_tokens WHERE user_id=$1) AS resets", [user.id().into()]))
        .await.expect("post-race writer state").expect("counts");
    assert_eq!(
        row.try_get::<i64>("", "refreshes").expect("refresh count"),
        0
    );
    assert_eq!(row.try_get::<i64>("", "resets").expect("reset count"), 0);
    assert_eq!(
        client
            .post(format!("{base}/api/auth/login"))
            .json(&json!({"email": email, "password": old_password}))
            .send()
            .await
            .expect("old-password denial")
            .status(),
        401
    );
    assert_eq!(
        client
            .post(format!("{base}/api/token/refresh"))
            .json(&json!({"refresh_token": refresh}))
            .send()
            .await
            .expect("old refresh denial")
            .status(),
        401
    );
    let post_reset = client
        .post(format!("{base}/api/auth/login"))
        .json(&json!({"email": email, "password": new_password}))
        .send()
        .await
        .expect("genuine post-reset login");
    assert_eq!(post_reset.status(), 200);
    let post_reset: Value = post_reset.json().await.expect("new session JSON");
    assert_persisted(
        db.as_ref(),
        post_reset["refresh_token"]
            .as_str()
            .expect("new credential session"),
    )
    .await;
    }).await;
}
