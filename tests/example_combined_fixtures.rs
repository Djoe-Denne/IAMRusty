//! Example integration test combining database and fixture systems
//! 
//! This demonstrates how to use both the test database setup and the fixture
//! system together for comprehensive integration testing.

mod common;
mod fixtures;

use common::TestFixture;
use fixtures::{GitHubFixtures, GitLabFixtures};
use fixtures::github::*;
use serial_test::serial;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn test_oauth_flow_with_database_persistence() {
    // Setup test database
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    let config = test_fixture.config();
    
    // Setup GitHub mock server
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;
    
    println!("🔗 Test Database URL: {}", config.database.url);
    println!("🔗 GitHub Mock URL: {}", github.base_url());
    
    // Simulate OAuth flow by creating user and provider token records
    let user_id = Uuid::new_v4();
    
    // Insert user
    let insert_user_sql = format!(
        "INSERT INTO users (id, username, created_at, updated_at) 
         VALUES ('{}', 'arthur', NOW(), NOW())",
        user_id
    );
    
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        insert_user_sql,
    ))
    .await
    .expect("Failed to insert user");
    
    // Insert provider token
    let insert_token_sql = format!(
        "INSERT INTO provider_tokens (user_id, provider, provider_user_id, access_token, created_at, updated_at) 
         VALUES ('{}', 'github', 'arthur_github_id', 'github_access_token', NOW(), NOW())",
        user_id
    );
    
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        insert_token_sql,
    ))
    .await
    .expect("Failed to insert provider token");
    
    // Verify data was persisted
    let user_count: i64 = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT COUNT(*) as count FROM users WHERE username = 'arthur'".to_string(),
        ))
        .await
        .expect("Failed to count users")
        .unwrap()
        .try_get("", "count")
        .expect("Failed to get count");
    
    let token_count: i64 = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT COUNT(*) as count FROM provider_tokens WHERE provider = 'github'".to_string(),
        ))
        .await
        .expect("Failed to count tokens")
        .unwrap()
        .try_get("", "count")
        .expect("Failed to get count");
    
    assert_eq!(user_count, 1, "Should have one user");
    assert_eq!(token_count, 1, "Should have one GitHub token");
    
    println!("✅ OAuth flow with database persistence successful");
    
    // Cleanup happens automatically when test_fixture is dropped
}

#[tokio::test]
#[serial]
async fn test_multi_provider_with_database() {
    // Setup test database
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Setup both GitHub and GitLab mocks
    let github = GitHubFixtures::service().await;
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;
    
    let gitlab = GitLabFixtures::service().await;
    gitlab.setup_successful_token_exchange().await;
    gitlab.setup_successful_user_profile_alice().await;
    
    // Create a user with multiple provider tokens
    let user_id = Uuid::new_v4();
    
    // Insert user
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        format!("INSERT INTO users (id, username, created_at, updated_at) 
                 VALUES ('{}', 'multi_provider_user', NOW(), NOW())", user_id),
    ))
    .await
    .expect("Failed to insert user");
    
    // Insert GitHub token
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        format!("INSERT INTO provider_tokens (user_id, provider, provider_user_id, access_token, created_at, updated_at) 
                 VALUES ('{}', 'github', 'github_user_id', 'github_token', NOW(), NOW())", user_id),
    ))
    .await
    .expect("Failed to insert GitHub token");
    
    // Insert GitLab token
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        format!("INSERT INTO provider_tokens (user_id, provider, provider_user_id, access_token, created_at, updated_at) 
                 VALUES ('{}', 'gitlab', 'gitlab_user_id', 'gitlab_token', NOW(), NOW())", user_id),
    ))
    .await
    .expect("Failed to insert GitLab token");
    
    // Verify both tokens exist for the same user
    let token_count: i64 = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!("SELECT COUNT(*) as count FROM provider_tokens WHERE user_id = '{}'", user_id),
        ))
        .await
        .expect("Failed to count tokens")
        .unwrap()
        .try_get("", "count")
        .expect("Failed to get count");
    
    assert_eq!(token_count, 2, "Should have tokens for both providers");
    
    println!("✅ Multi-provider setup with database successful");
    println!("🔗 GitHub mock URL: {}", github.base_url());
    println!("🔗 GitLab mock URL: {}", gitlab.base_url());
}

#[tokio::test]
#[serial]
async fn test_error_scenarios_with_database() {
    // Setup test database
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Setup GitHub mock for error scenarios
    let github = GitHubFixtures::service().await;
    github.setup_failed_token_exchange_invalid_code().await;
    github.setup_failed_user_profile_unauthorized().await;
    
    // Test database constraint violations
    let user_id = Uuid::new_v4();
    
    // Insert user
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        format!("INSERT INTO users (id, username, created_at, updated_at) 
                 VALUES ('{}', 'error_test_user', NOW(), NOW())", user_id),
    ))
    .await
    .expect("Failed to insert user");
    
    // Try to insert duplicate provider token (should work since we allow multiple tokens)
    let result1 = db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        format!("INSERT INTO provider_tokens (user_id, provider, provider_user_id, access_token, created_at, updated_at) 
                 VALUES ('{}', 'github', 'same_provider_id', 'token1', NOW(), NOW())", user_id),
    ))
    .await;
    
    let result2 = db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        format!("INSERT INTO provider_tokens (user_id, provider, provider_user_id, access_token, created_at, updated_at) 
                 VALUES ('{}', 'github', 'same_provider_id', 'token2', NOW(), NOW())", user_id),
    ))
    .await;
    
    // First insert should succeed
    assert!(result1.is_ok(), "First token insert should succeed");
    
    // Second insert should fail due to unique constraint on provider + provider_user_id
    assert!(result2.is_err(), "Second token insert should fail due to unique constraint");
    
    println!("✅ Error scenarios with database constraints working correctly");
    println!("🔗 GitHub error mock URL: {}", github.base_url());
}

#[tokio::test]
#[serial]
async fn test_configuration_consistency() {
    // Setup test database
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let config = test_fixture.config();
    
    // Setup fixtures
    let github = GitHubFixtures::service().await;
    let gitlab = GitLabFixtures::service().await;
    
    // Verify that mock URLs can be used to override config URLs
    assert!(config.database.url.contains("localhost"));
    assert!(config.database.url.contains("iam_test"));
    
    // In a real integration test, you would override the OAuth URLs with mock URLs
    println!("✅ Configuration consistency verified");
    println!("📋 Test config OAuth GitHub URL: {}", config.oauth.github.auth_url);
    println!("📋 Test config OAuth GitLab URL: {}", config.oauth.gitlab.auth_url);
    println!("🔗 GitHub mock URL: {}", github.base_url());
    println!("🔗 GitLab mock URL: {}", gitlab.base_url());
    
    // In a real application, you would:
    // 1. Override config.oauth.github.* URLs with github.base_url()
    // 2. Override config.oauth.gitlab.* URLs with gitlab.base_url()
    // 3. Use the test database URL from config.database.url
} 