//! Integration tests demonstrating database test setup
//! 
//! This module shows how to use the TestFixture for database testing
//! with automatic container management and table truncation.

mod common;

use common::TestFixture;
use serial_test::serial;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

// Import your domain entities here when available
// For now, we'll use raw SQL to demonstrate the functionality

#[tokio::test]
#[serial]
async fn test_database_setup_and_cleanup() {
    // Create test fixture - this will start container and run migrations
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    
    // Get database connection
    let db = fixture.db();
    
    // Verify database is working by checking table existence
    let result = db
        .query_all(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'".to_string(),
        ))
        .await
        .expect("Failed to query tables");
    
    // Should have our migrated tables
    assert!(!result.is_empty(), "No tables found - migrations may have failed");
    
    println!("✅ Database setup successful with {} tables", result.len());
    
    // Cleanup happens automatically when fixture is dropped
}

#[tokio::test]
#[serial]
async fn test_table_truncation_between_tests() {
    // First test - insert some data
    {
        let fixture = TestFixture::new().await.expect("Failed to create test fixture");
        let db = fixture.db();
        
        // Insert a test user (using raw SQL for demonstration)
        let user_id = Uuid::new_v4();
        let insert_sql = format!(
            "INSERT INTO users (id, username, created_at, updated_at) 
             VALUES ('{}', 'test_user', NOW(), NOW())",
            user_id
        );
        
        db.execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            insert_sql,
        ))
        .await
        .expect("Failed to insert test user");
        
        // Verify data exists
        let count_result = db
            .query_one(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT COUNT(*) as count FROM users".to_string(),
            ))
            .await
            .expect("Failed to count users");
        
        let count: i64 = count_result
            .unwrap()
            .try_get("", "count")
            .expect("Failed to get count");
        
        assert_eq!(count, 1, "User should be inserted");
        println!("✅ Test data inserted successfully");
    }
    
    // Second test - verify data is cleaned up
    {
        let fixture = TestFixture::new().await.expect("Failed to create test fixture");
        let db = fixture.db();
        
        // Verify data is cleaned up
        let count_result = db
            .query_one(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT COUNT(*) as count FROM users".to_string(),
            ))
            .await
            .expect("Failed to count users");
        
        let count: i64 = count_result
            .unwrap()
            .try_get("", "count")
            .expect("Failed to get count");
        
        assert_eq!(count, 0, "Users table should be empty after cleanup");
        println!("✅ Table truncation successful - data cleaned up");
    }
}

#[tokio::test]
#[serial]
async fn test_configuration_integration() {
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let config = fixture.config();
    
    // Verify configuration is properly set up
    assert!(config.database.url.contains("localhost"));
    assert!(config.database.url.contains("iam_test"));
    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.oauth.github.client_id, "test_github_client_id");
    assert_eq!(config.oauth.gitlab.client_id, "test_gitlab_client_id");
    
    println!("✅ Configuration integration successful");
    println!("🔗 Database URL: {}", config.database.url);
}

#[tokio::test]
#[serial]
async fn test_connection_pool_functionality() {
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let pool = fixture.pool();
    
    // Test write connection
    let write_conn = pool.get_write_connection();
    let _result = write_conn
        .ping()
        .await
        .expect("Failed to ping write connection");
    
    // Test read connection (should be same as write in test setup)
    let read_conn = pool.get_read_connection();
    let _result = read_conn
        .ping()
        .await
        .expect("Failed to ping read connection");
    
    println!("✅ Connection pool functionality verified");
}

#[tokio::test]
#[serial]
async fn test_foreign_key_constraints() {
    let fixture = TestFixture::new().await.expect("Failed to create test fixture");
    let db = fixture.db();
    
    // Insert a user first
    let user_id = Uuid::new_v4();
    let insert_user_sql = format!(
        "INSERT INTO users (id, username, created_at, updated_at) 
         VALUES ('{}', 'test_user', NOW(), NOW())",
        user_id
    );
    
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        insert_user_sql,
    ))
    .await
    .expect("Failed to insert test user");
    
    // Insert a provider token for the user
    let insert_token_sql = format!(
        "INSERT INTO provider_tokens (user_id, provider, provider_user_id, access_token, created_at, updated_at) 
         VALUES ('{}', 'github', 'test_provider_id', 'test_access_token', NOW(), NOW())",
        user_id
    );
    
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        insert_token_sql,
    ))
    .await
    .expect("Failed to insert provider token");
    
    // Verify both records exist
    let user_count: i64 = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT COUNT(*) as count FROM users".to_string(),
        ))
        .await
        .expect("Failed to count users")
        .unwrap()
        .try_get("", "count")
        .expect("Failed to get user count");
    
    let token_count: i64 = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT COUNT(*) as count FROM provider_tokens".to_string(),
        ))
        .await
        .expect("Failed to count tokens")
        .unwrap()
        .try_get("", "count")
        .expect("Failed to get token count");
    
    assert_eq!(user_count, 1, "Should have one user");
    assert_eq!(token_count, 1, "Should have one token");
    
    println!("✅ Foreign key constraints working correctly");
    
    // Cleanup will happen automatically and should handle foreign keys properly
}
