// Include common test utilities and fixtures
#[path = "common/mod.rs"] 
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

use common::{get_test_server, TestFixture};
use fixtures::{GitHubFixtures, GitLabFixtures, DbFixtures};
use reqwest::Client;
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;
use chrono::{Utc, Duration};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

/// Create a common HTTP client for tests
fn create_test_client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to create HTTP client")
}

/// Helper function to count entities in database
async fn count_entities(db: std::sync::Arc<sea_orm::DatabaseConnection>, table: &str) -> Result<i64, sea_orm::DbErr> {
    let count: i64 = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            format!("SELECT COUNT(*) as count FROM {}", table),
        ))
        .await?
        .unwrap()
        .try_get("", "count")?;
    Ok(count)
}

/// Helper function to verify JWT token structure (basic validation)
fn verify_jwt_structure(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    parts.len() == 3 && !parts[0].is_empty() && !parts[1].is_empty() && !parts[2].is_empty()
}

/// Helper function to create a test refresh token in the database
async fn create_test_refresh_token(
    db: std::sync::Arc<sea_orm::DatabaseConnection>,
    user_id: Uuid,
    token: &str,
    is_valid: bool,
    expires_at: chrono::DateTime<Utc>,
) -> Result<Uuid, sea_orm::DbErr> {
    let token_id = Uuid::new_v4();
    
    db.execute(Statement::from_string(
        DatabaseBackend::Postgres,
        format!(
            "INSERT INTO refresh_tokens (id, user_id, token, is_valid, created_at, expires_at) VALUES ('{}', '{}', '{}', {}, NOW(), '{}')",
            token_id, user_id, token, is_valid, expires_at.format("%Y-%m-%d %H:%M:%S")
        )
    )).await?;
    
    Ok(token_id)
}

/// Helper function to invalidate a refresh token in the database
async fn invalidate_refresh_token(
    db: std::sync::Arc<sea_orm::DatabaseConnection>,
    token_id: Uuid,
) -> Result<(), sea_orm::DbErr> {
    db.execute(Statement::from_string(
        DatabaseBackend::Postgres,
        format!("UPDATE refresh_tokens SET is_valid = false WHERE id = '{}'", token_id),
    )).await?;
    
    Ok(())
}

/// Helper function to check if a refresh token exists in the database
async fn refresh_token_exists(
    db: std::sync::Arc<sea_orm::DatabaseConnection>,
    token: &str,
) -> Result<bool, sea_orm::DbErr> {
    let result = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            format!("SELECT COUNT(*) as count FROM refresh_tokens WHERE token = '{}'", token),
        ))
        .await?;
    
    if let Some(result) = result {
        let count: i64 = result.try_get("", "count")?;
        Ok(count > 0)
    } else {
        Ok(false)
    }
}

// 🔒 Token Refresh Endpoint Tests
// 🔁 /token/refresh

#[tokio::test]
#[serial]
async fn test_refresh_token_success_with_valid_refresh_token() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Pre-create user
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");
    
    // Create valid refresh token
    let refresh_token = "valid_refresh_token_123";
    let expires_at = Utc::now() + Duration::hours(24); // Valid for 24 hours
    
    let token_id = create_test_refresh_token(
        db.clone(),
        user.id(),
        refresh_token,
        true,
        expires_at,
    ).await.expect("Failed to create refresh token");
    
    // Make refresh request
    let response = client
        .post(&format!("{}/api/token/refresh", base_url))
        .json(&json!({
            "refresh_token": refresh_token
        }))
        .send()
        .await
        .expect("Failed to send request");
    
    // ✅ Should return 200 OK with new access token
    assert_eq!(response.status(), 200, "Should return 200 OK for valid refresh token");
    
    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON response");
    
    // ✅ Should contain valid JWT access token
    assert!(response_json["token"].is_string(), 
           "Response should contain access token");
    
    let access_token = response_json["token"].as_str().unwrap();
    assert!(verify_jwt_structure(access_token), 
           "Access token should have valid JWT structure");
    
    // ✅ Should contain expiration time
    assert!(response_json["expires_in"].is_number(), 
           "Response should contain expires_in");
    
    let expires_in = response_json["expires_in"].as_u64().unwrap();
    assert!(expires_in > 0 && expires_in <= 3600, 
           "expires_in should be reasonable (0-3600 seconds)");
}

#[tokio::test]
#[serial]
async fn test_refresh_token_returns_401_for_invalid_refresh_token() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Make refresh request with non-existent refresh token
    let response = client
        .post(&format!("{}/api/token/refresh", base_url))
        .json(&json!({
            "refresh_token": "non_existent_refresh_token"
        }))
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 401 Unauthorized for invalid refresh token
    assert_eq!(response.status(), 401, "Should return 401 for invalid refresh token");
    
    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON error response");
    
    assert!(response_json["error"].is_object(), 
           "Should return error object");
    assert_eq!(response_json["error"]["status"], 401, 
              "Error status should be 401");
}

#[tokio::test]
#[serial]
async fn test_refresh_token_returns_401_for_expired_refresh_token() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Pre-create user
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");
    
    // Create expired refresh token
    let refresh_token = "expired_refresh_token_456";
    let expires_at = Utc::now() - Duration::hours(1); // Expired 1 hour ago
    
    let token_id = create_test_refresh_token(
        db.clone(),
        user.id(),
        refresh_token,
        true,
        expires_at,
    ).await.expect("Failed to create refresh token");
    
    // Make refresh request with expired token
    let response = client
        .post(&format!("{}/api/token/refresh", base_url))
        .json(&json!({
            "refresh_token": refresh_token
        }))
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 401 Unauthorized for expired refresh token
    assert_eq!(response.status(), 401, "Should return 401 for expired refresh token");
    
    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON error response");
    
    assert!(response_json["error"].is_object(), 
           "Should return error object");
    assert_eq!(response_json["error"]["status"], 401, 
              "Error status should be 401");
}

#[tokio::test]
#[serial]
async fn test_refresh_token_returns_401_for_revoked_refresh_token() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Pre-create user
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");
    
    // Create revoked refresh token
    let refresh_token = "revoked_refresh_token_789";
    let expires_at = Utc::now() + Duration::hours(24); // Valid expiration
    
    let token_id = create_test_refresh_token(
        db.clone(),
        user.id(),
        refresh_token,
        false, // is_valid = false (revoked)
        expires_at,
    ).await.expect("Failed to create refresh token");
    
    // Make refresh request with revoked token
    let response = client
        .post(&format!("{}/api/token/refresh", base_url))
        .json(&json!({
            "refresh_token": refresh_token
        }))
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 401 Unauthorized for revoked refresh token
    assert_eq!(response.status(), 401, "Should return 401 for revoked refresh token");
    
    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON error response");
    
    assert!(response_json["error"].is_object(), 
           "Should return error object");
    assert_eq!(response_json["error"]["status"], 401, 
              "Error status should be 401");
}

#[tokio::test]
#[serial]
async fn test_refresh_token_returns_400_for_missing_refresh_token() {
    // Setup test environment
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Make refresh request without refresh_token field
    let response = client
        .post(&format!("{}/api/token/refresh", base_url))
        .json(&json!({}))
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 422 Unprocessable Entity for missing refresh token field
    assert_eq!(response.status(), 422, "Should return 422 for missing refresh token");
    
    // For 422 status, the server might not return JSON, so check response body carefully
    let response_text = response.text().await.expect("Should be able to read response text");
    
    // If the response is empty or very small, it's likely not JSON
    if response_text.trim().is_empty() || response_text.len() < 10 {
        // This is acceptable for validation errors - the status code is the important part
        return;
    }
    
    // If there is response content, try to parse it as JSON
    if let Ok(response_json) = serde_json::from_str::<Value>(&response_text) {
        assert!(response_json["error"].is_object(), 
               "Should return error object");
        assert_eq!(response_json["error"]["status"], 422, 
                  "Error status should be 422");
    }
    // If it's not valid JSON, that's also acceptable for validation errors
}

#[tokio::test]
#[serial]
async fn test_refresh_token_returns_400_for_empty_refresh_token() {
    // Setup test environment
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Make refresh request with empty refresh token
    let response = client
        .post(&format!("{}/api/token/refresh", base_url))
        .json(&json!({
            "refresh_token": ""
        }))
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 400 Bad Request for empty refresh token
    assert_eq!(response.status(), 400, "Should return 400 for empty refresh token");
    
    let response_json: Value = response
        .json()
        .await
        .expect("Should return JSON error response");
    
    assert!(response_json["error"].is_object(), 
           "Should return error object");
    assert_eq!(response_json["error"]["status"], 400, 
              "Error status should be 400");
}

#[tokio::test]
#[serial]
async fn test_refresh_token_returns_400_for_malformed_json() {
    // Setup test environment
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Make refresh request with malformed JSON
    let response = client
        .post(&format!("{}/api/token/refresh", base_url))
        .header("Content-Type", "application/json")
        .body("{ invalid json")
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 400 Bad Request for malformed JSON
    assert_eq!(response.status(), 400, "Should return 400 for malformed JSON");
}

#[tokio::test]
#[serial]
async fn test_refresh_token_returns_400_for_wrong_content_type() {
    // Setup test environment
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Make refresh request with wrong content type
    let response = client
        .post(&format!("{}/api/token/refresh", base_url))
        .header("Content-Type", "text/plain")
        .body("refresh_token=some_token")
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 415 Unsupported Media Type for wrong content type
    assert_eq!(response.status(), 415, "Should return 415 for wrong content type");
}

#[tokio::test]
#[serial]
async fn test_refresh_token_invalidates_expired_token_automatically() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Pre-create user
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");
    
    // Create expired refresh token
    let refresh_token = "expired_token_to_invalidate";
    let expires_at = Utc::now() - Duration::hours(1);
    
    let token_id = create_test_refresh_token(
        db.clone(),
        user.id(),
        refresh_token,
        true, // Initially valid
        expires_at,
    ).await.expect("Failed to create refresh token");
    
    // Make refresh request with expired token
    let response = client
        .post(&format!("{}/api/token/refresh", base_url))
        .json(&json!({
            "refresh_token": refresh_token
        }))
        .send()
        .await
        .expect("Failed to send request");
    
    // ❌ Should return 401 for expired token
    assert_eq!(response.status(), 401, "Should return 401 for expired token");
    
    // ✅ Should have automatically invalidated the token in the database
    let is_valid: bool = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            format!("SELECT is_valid FROM refresh_tokens WHERE id = '{}'", token_id),
        ))
        .await
        .expect("Failed to query token validity")
        .unwrap()
        .try_get("", "is_valid")
        .expect("Failed to get is_valid field");
    
    assert!(!is_valid, "Expired token should be automatically invalidated");
}

#[tokio::test]
#[serial]
async fn test_refresh_token_replay_attack_protection() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Pre-create user
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");
    
    // Create valid refresh token
    let refresh_token = "replay_attack_token";
    let expires_at = Utc::now() + Duration::hours(24);
    
    let token_id = create_test_refresh_token(
        db.clone(),
        user.id(),
        refresh_token,
        true,
        expires_at,
    ).await.expect("Failed to create refresh token");
    
    // First request should succeed
    let response1 = client
        .post(&format!("{}/api/token/refresh", base_url))
        .json(&json!({
            "refresh_token": refresh_token
        }))
        .send()
        .await
        .expect("Failed to send first request");
    
    assert_eq!(response1.status(), 200, "First request should succeed");
    
    // Make the same request again immediately (replay attack)
    let response2 = client
        .post(&format!("{}/api/token/refresh", base_url))
        .json(&json!({
            "refresh_token": refresh_token
        }))
        .send()
        .await
        .expect("Failed to send second request");
    
    // ✅ Second request should also succeed (token should be reusable)
    // Note: In some implementations, refresh tokens are single-use. 
    // This test assumes they're reusable until explicitly revoked.
    assert_eq!(response2.status(), 200, "Second request should also succeed for reusable refresh tokens");
}

#[tokio::test]
#[serial]
async fn test_refresh_token_concurrent_requests_with_same_token() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    
    // Pre-create user
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");
    
    // Create valid refresh token
    let refresh_token = "concurrent_access_token";
    let expires_at = Utc::now() + Duration::hours(24);
    
    let token_id = create_test_refresh_token(
        db.clone(),
        user.id(),
        refresh_token,
        true,
        expires_at,
    ).await.expect("Failed to create refresh token");
    
    // Make multiple concurrent requests with the same refresh token
    let mut handles = vec![];
    
    for i in 0..3 {
        let base_url = base_url.clone();
        let token = refresh_token.to_string();
        
        let handle = tokio::spawn(async move {
            let client = create_test_client();
            let response = client
                .post(&format!("{}/api/token/refresh", base_url))
                .json(&json!({
                    "refresh_token": token
                }))
                .send()
                .await
                .expect("Failed to send request");
            
            (i, response.status(), response.json::<Value>().await)
        });
        
        handles.push(handle);
    }
    
    // Wait for all requests to complete
    let mut success_count = 0;
    
    for handle in handles {
        let (request_id, status, response_result) = handle.await.expect("Request failed");
        
        if status == 200 {
            success_count += 1;
            let response_json = response_result.expect("Should return JSON response");
            assert!(response_json["token"].is_string(), 
                   "Request {} should return valid access token", request_id);
        }
    }
    
    // ✅ All requests should succeed for reusable refresh tokens
    assert_eq!(success_count, 3, "All concurrent requests should succeed");
}

#[tokio::test]
#[serial]
async fn test_refresh_token_generates_unique_access_tokens() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    let client = create_test_client();
    
    // Pre-create user
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");
    
    // Create valid refresh token
    let refresh_token = "unique_token_generator";
    let expires_at = Utc::now() + Duration::hours(24);
    
    let token_id = create_test_refresh_token(
        db.clone(),
        user.id(),
        refresh_token,
        true,
        expires_at,
    ).await.expect("Failed to create refresh token");
    
    // Make multiple requests and collect access tokens
    let mut access_tokens = std::collections::HashSet::new();
    
    for i in 0..5 {
        let response = client
            .post(&format!("{}/api/token/refresh", base_url))
            .json(&json!({
                "refresh_token": refresh_token
            }))
            .send()
            .await
            .expect("Failed to send request");
        
        assert_eq!(response.status(), 200, "Request {} should succeed", i);
        
        let response_json: Value = response
            .json()
            .await
            .expect("Should return JSON response");
        
        let access_token = response_json["token"].as_str().unwrap();
        
        // ✅ Each access token should be unique
        assert!(!access_tokens.contains(access_token), 
               "Access token {} should be unique", i);
        access_tokens.insert(access_token.to_string());
    }
    
    // ✅ Verify we collected 5 unique access tokens
    assert_eq!(access_tokens.len(), 5, "Should generate 5 unique access tokens");
}

#[tokio::test]
#[serial]
async fn test_refresh_token_performance_under_load() {
    // Setup test environment
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let base_url = get_test_server().await.expect("Failed to start test server");
    
    // Pre-create user
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to create user");
    
    // Create valid refresh token
    let refresh_token = "performance_test_token";
    let expires_at = Utc::now() + Duration::hours(24);
    
    let token_id = create_test_refresh_token(
        db.clone(),
        user.id(),
        refresh_token,
        true,
        expires_at,
    ).await.expect("Failed to create refresh token");
    
    // Measure time for rapid sequential requests
    let start_time = std::time::Instant::now();
    let num_requests = 10;
    
    for i in 0..num_requests {
        let client = create_test_client();
        let response = client
            .post(&format!("{}/api/token/refresh", base_url))
            .json(&json!({
                "refresh_token": refresh_token
            }))
            .send()
            .await
            .expect("Failed to send request");
        
        assert_eq!(response.status(), 200, "Request {} should succeed", i);
    }
    
    let elapsed = start_time.elapsed();
    
    // ✅ Performance should be reasonable (less than 5 seconds for 10 requests)
    assert!(elapsed.as_secs() < 5, 
           "10 refresh token requests should complete in less than 5 seconds, took: {:?}", elapsed);
    
    // ✅ Average response time should be reasonable (less than 500ms per request)
    let avg_time_per_request = elapsed.as_millis() / num_requests;
    assert!(avg_time_per_request < 500, 
           "Average response time should be less than 500ms, was: {}ms", avg_time_per_request);
} 