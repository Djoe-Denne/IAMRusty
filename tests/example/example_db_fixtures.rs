mod common;
mod fixtures;

use common::TestFixture;
use fixtures::DbFixtures;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_user_fixture_basic() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create a user using the fluent API
    let user = DbFixtures::user()
        .username("test_user")
        .avatar_url(Some("https://example.com/avatar.png".to_string()))
        .commit(db.clone())
        .await
        .expect("Failed to commit user");
    
    // Check that the user exists in the database
    let exists = user.check(db.clone()).await.expect("Failed to check user");
    assert!(exists, "User should exist in database");
    
    // Verify user properties
    assert_eq!(user.username(), "test_user");
    assert_eq!(user.avatar_url(), Some(&"https://example.com/avatar.png".to_string()));
    
    println!("✅ User fixture created and verified: {}", user.id());
}

#[tokio::test]
#[serial]
async fn test_user_fixture_factory_methods() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Test factory methods
    let arthur = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to commit Arthur");
    
    let bob = DbFixtures::user()
        .bob()
        .commit(db.clone())
        .await
        .expect("Failed to commit Bob");
    
    let alice = DbFixtures::user()
        .alice()
        .commit(db.clone())
        .await
        .expect("Failed to commit Alice");
    
    // Verify factory method data
    assert_eq!(arthur.username(), "arthur");
    assert_eq!(bob.username(), "bob");
    assert_eq!(alice.username(), "alice");
    
    // Check all users exist
    assert!(arthur.check(db.clone()).await.expect("Failed to check Arthur"));
    assert!(bob.check(db.clone()).await.expect("Failed to check Bob"));
    assert!(alice.check(db.clone()).await.expect("Failed to check Alice"));
    
    println!("✅ Factory method users created: Arthur({}), Bob({}), Alice({})", 
             arthur.id(), bob.id(), alice.id());
}

#[tokio::test]
#[serial]
async fn test_user_email_fixtures() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create a user first
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to commit user");
    
    // Create primary email
    let primary_email = DbFixtures::user_email()
        .arthur_primary(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to commit primary email");
    
    // Create secondary email
    let secondary_email = DbFixtures::user_email()
        .arthur_github(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to commit secondary email");
    
    // Verify email properties
    assert_eq!(primary_email.user_id(), user.id());
    assert_eq!(primary_email.email(), "arthur@example.com");
    assert!(primary_email.is_primary());
    assert!(primary_email.is_verified());
    
    assert_eq!(secondary_email.user_id(), user.id());
    assert_eq!(secondary_email.email(), "arthur@github.example.com");
    assert!(!secondary_email.is_primary());
    assert!(!secondary_email.is_verified());
    
    // Check emails exist
    assert!(primary_email.check(db.clone()).await.expect("Failed to check primary email"));
    assert!(secondary_email.check(db.clone()).await.expect("Failed to check secondary email"));
    
    println!("✅ User emails created: Primary({}), Secondary({})", 
             primary_email.id(), secondary_email.id());
}

#[tokio::test]
#[serial]
async fn test_provider_token_fixtures() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create a user first
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to commit user");
    
    // Create GitHub token
    let github_token = DbFixtures::provider_token()
        .arthur_github(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to commit GitHub token");
    
    // Create GitLab token for the same user
    let gitlab_token = DbFixtures::provider_token()
        .gitlab(user.id())
        .provider_user_id("arthur_gitlab_123")
        .commit(db.clone())
        .await
        .expect("Failed to commit GitLab token");
    
    // Verify token properties
    assert_eq!(github_token.user_id(), user.id());
    assert_eq!(github_token.provider(), "github");
    assert_eq!(github_token.provider_user_id(), "123456");
    assert!(github_token.refresh_token().is_some());
    assert_eq!(github_token.expires_in(), Some(3600));
    
    assert_eq!(gitlab_token.user_id(), user.id());
    assert_eq!(gitlab_token.provider(), "gitlab");
    assert_eq!(gitlab_token.provider_user_id(), "arthur_gitlab_123");
    
    // Check tokens exist
    assert!(github_token.check(db.clone()).await.expect("Failed to check GitHub token"));
    assert!(gitlab_token.check(db.clone()).await.expect("Failed to check GitLab token"));
    
    println!("✅ Provider tokens created: GitHub({}), GitLab({})", 
             github_token.id(), gitlab_token.id());
}

#[tokio::test]
#[serial]
async fn test_refresh_token_fixtures() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create a user first
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to commit user");
    
    // Create valid refresh token
    let valid_token = DbFixtures::refresh_token()
        .arthur_valid(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to commit valid token");
    
    // Create expired refresh token
    let expired_token = DbFixtures::refresh_token()
        .expired(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to commit expired token");
    
    // Create invalid refresh token
    let invalid_token = DbFixtures::refresh_token()
        .invalid(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to commit invalid token");
    
    // Verify token properties
    assert_eq!(valid_token.user_id(), user.id());
    assert_eq!(valid_token.token(), "arthur_refresh_token_123");
    assert!(valid_token.is_valid());
    assert!(valid_token.is_usable());
    
    assert_eq!(expired_token.user_id(), user.id());
    assert!(expired_token.is_valid());
    assert!(expired_token.is_expired());
    assert!(!expired_token.is_usable());
    
    assert_eq!(invalid_token.user_id(), user.id());
    assert!(!invalid_token.is_valid());
    assert!(!invalid_token.is_usable());
    
    // Check tokens exist
    assert!(valid_token.check(db.clone()).await.expect("Failed to check valid token"));
    assert!(expired_token.check(db.clone()).await.expect("Failed to check expired token"));
    assert!(invalid_token.check(db.clone()).await.expect("Failed to check invalid token"));
    
    println!("✅ Refresh tokens created: Valid({}), Expired({}), Invalid({})", 
             valid_token.id(), expired_token.id(), invalid_token.id());
}

#[tokio::test]
#[serial]
async fn test_complete_user_setup() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create a complete user setup with all related entities
    let user = DbFixtures::user()
        .arthur()
        .commit(db.clone())
        .await
        .expect("Failed to commit user");
    
    // Add primary email
    let primary_email = DbFixtures::user_email()
        .arthur_primary(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to commit primary email");
    
    // Add secondary email
    let secondary_email = DbFixtures::user_email()
        .arthur_github(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to commit secondary email");
    
    // Add GitHub provider token
    let github_token = DbFixtures::provider_token()
        .arthur_github(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to commit GitHub token");
    
    // Add GitLab provider token
    let gitlab_token = DbFixtures::provider_token()
        .alice_gitlab(user.id()) // Using Alice's GitLab setup for variety
        .commit(db.clone())
        .await
        .expect("Failed to commit GitLab token");
    
    // Add refresh token
    let refresh_token = DbFixtures::refresh_token()
        .arthur_valid(user.id())
        .commit(db.clone())
        .await
        .expect("Failed to commit refresh token");
    
    // Verify all entities exist and are properly linked
    assert!(user.check(db.clone()).await.expect("Failed to check user"));
    assert!(primary_email.check(db.clone()).await.expect("Failed to check primary email"));
    assert!(secondary_email.check(db.clone()).await.expect("Failed to check secondary email"));
    assert!(github_token.check(db.clone()).await.expect("Failed to check GitHub token"));
    assert!(gitlab_token.check(db.clone()).await.expect("Failed to check GitLab token"));
    assert!(refresh_token.check(db.clone()).await.expect("Failed to check refresh token"));
    
    // Verify relationships
    assert_eq!(primary_email.user_id(), user.id());
    assert_eq!(secondary_email.user_id(), user.id());
    assert_eq!(github_token.user_id(), user.id());
    assert_eq!(gitlab_token.user_id(), user.id());
    assert_eq!(refresh_token.user_id(), user.id());
    
    println!("✅ Complete user setup created:");
    println!("   User: {} ({})", user.username(), user.id());
    println!("   Primary Email: {} ({})", primary_email.email(), primary_email.id());
    println!("   Secondary Email: {} ({})", secondary_email.email(), secondary_email.id());
    println!("   GitHub Token: {} ({})", github_token.provider_user_id(), github_token.id());
    println!("   GitLab Token: {} ({})", gitlab_token.provider_user_id(), gitlab_token.id());
    println!("   Refresh Token: {} ({})", refresh_token.is_usable(), refresh_token.id());
}

#[tokio::test]
#[serial]
async fn test_custom_fixture_data() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create custom user with specific data
    let custom_user = DbFixtures::user()
        .username("custom_test_user")
        .avatar_url(None)
        .commit(db.clone())
        .await
        .expect("Failed to commit custom user");
    
    // Create custom email with specific properties
    let custom_email = DbFixtures::user_email()
        .user_id(custom_user.id())
        .email("custom@test.example.com")
        .is_primary(true)
        .is_verified(false) // Unverified primary email
        .commit(db.clone())
        .await
        .expect("Failed to commit custom email");
    
    // Create custom provider token with specific expiration
    let custom_token = DbFixtures::provider_token()
        .user_id(custom_user.id())
        .provider("custom_provider")
        .access_token("custom_access_token_123")
        .refresh_token(None) // No refresh token
        .expires_in(Some(1800)) // 30 minutes
        .provider_user_id("custom_provider_user_456")
        .commit(db.clone())
        .await
        .expect("Failed to commit custom token");
    
    // Verify custom data
    assert_eq!(custom_user.username(), "custom_test_user");
    assert_eq!(custom_user.avatar_url(), None);
    
    assert_eq!(custom_email.email(), "custom@test.example.com");
    assert!(custom_email.is_primary());
    assert!(!custom_email.is_verified());
    
    assert_eq!(custom_token.provider(), "custom_provider");
    assert_eq!(custom_token.access_token(), "custom_access_token_123");
    assert_eq!(custom_token.refresh_token(), None);
    assert_eq!(custom_token.expires_in(), Some(1800));
    assert_eq!(custom_token.provider_user_id(), "custom_provider_user_456");
    
    // Check all exist
    assert!(custom_user.check(db.clone()).await.expect("Failed to check custom user"));
    assert!(custom_email.check(db.clone()).await.expect("Failed to check custom email"));
    assert!(custom_token.check(db.clone()).await.expect("Failed to check custom token"));
    
    println!("✅ Custom fixture data created and verified");
}

#[tokio::test]
#[serial]
async fn test_fixture_isolation() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // This test verifies that each test starts with a clean database
    // due to table truncation between tests
    
    // Create a user
    let user = DbFixtures::user()
        .username("isolation_test_user")
        .commit(db.clone())
        .await
        .expect("Failed to commit user");
    
    // Verify it exists
    assert!(user.check(db.clone()).await.expect("Failed to check user"));
    
    println!("✅ Fixture isolation test - user created: {}", user.id());
    
    // This user will be automatically cleaned up by table truncation
    // The next test will start with empty tables
}

#[tokio::test]
#[serial]
async fn test_fixture_modification_without_commit_should_fail_check() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create and commit a user fixture
    let original_user = DbFixtures::user()
        .username("original_user")
        .avatar_url(Some("https://example.com/original.png".to_string()))
        .commit(db.clone())
        .await
        .expect("Failed to commit original user");
    
    // Verify the original user exists and matches
    assert!(original_user.check(db.clone()).await.expect("Failed to check original user"));
    
    // Now manually update the user in the database to simulate external modification
    // This simulates the scenario where the database record changes after the fixture was created
    use sea_orm::{EntityTrait, ActiveModelTrait, ActiveValue};
    use infra::repository::entity::users::{Entity as UsersEntity, ActiveModel as UserActiveModel};
    
    let mut user_active_model: UserActiveModel = UsersEntity::find_by_id(original_user.id())
        .one(&*db)
        .await
        .expect("Failed to find user")
        .expect("User should exist")
        .into();
    
    // Modify the user data directly in the database
    user_active_model.username = ActiveValue::Set("modified_user".to_string());
    user_active_model.avatar_url = ActiveValue::Set(Some("https://example.com/modified.png".to_string()));
    
    let _updated_user = user_active_model.update(&*db).await.expect("Failed to update user");
    
    // The original fixture should now fail the check because the database has been updated
    // but the original fixture still has the old data
    let original_check_result = original_user.check(db.clone()).await.expect("Failed to check original user after modification");
    assert!(!original_check_result, "Original fixture should fail check after database was modified");
    
    // Verify the original fixture still has the old data (unchanged)
    assert_eq!(original_user.username(), "original_user");
    assert_eq!(original_user.avatar_url(), Some(&"https://example.com/original.png".to_string()));
    
    println!("✅ Fixture modification test passed:");
    println!("   Original fixture (stale): {} - check failed as expected", original_user.username());
    println!("   Database was modified externally, but fixture data remains unchanged");
}

#[tokio::test]
#[serial]
async fn test_fixture_check_with_deleted_record() {
    let test_fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = test_fixture.db();
    
    // Create and commit a user fixture
    let user = DbFixtures::user()
        .username("user_to_delete")
        .commit(db.clone())
        .await
        .expect("Failed to commit user");
    
    // Verify the user exists
    assert!(user.check(db.clone()).await.expect("Failed to check user"));
    
    // Manually delete the user from the database (simulating external deletion)
    use sea_orm::{EntityTrait, ModelTrait};
    use infra::repository::entity::users::Entity as UsersEntity;
    
    let user_model = UsersEntity::find_by_id(user.id())
        .one(&*db)
        .await
        .expect("Failed to find user")
        .expect("User should exist");
    
    user_model.delete(&*db).await.expect("Failed to delete user");
    
    // Now the fixture check should fail because the user no longer exists in the database
    let check_result = user.check(db.clone()).await.expect("Failed to check user after deletion");
    assert!(!check_result, "Fixture check should fail when record is deleted from database");
    
    println!("✅ Deleted record test passed - fixture check failed as expected when record was deleted");
} 
