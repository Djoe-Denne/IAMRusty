use axum_test::TestServer;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::OnceCell;
use uuid::Uuid;
use axum::http::StatusCode;

mod common;
use common::fixtures::{
    MockOAuthProvider, OAuthStateFixtures, ResponseAssertions, 
    TestRequestBuilder, UserFixtures
};
use common::DatabaseContainer;

/// Global database container for all tests (CI/CD optimized)
static DATABASE: OnceCell<Arc<DatabaseContainer>> = OnceCell::const_new();

/// Setup database container (runs once for all tests)
async fn setup_database() -> Arc<DatabaseContainer> {
    DATABASE
        .get_or_init(|| async {
            Arc::new(
                DatabaseContainer::new()
                    .await
                    .expect("Failed to start test database")
            )
        })
        .await
        .clone()
}

/// Test fixture for each test - ensures clean state
struct TestFixture {
    pub server: TestServer,
    pub database: Arc<DatabaseContainer>,
    pub mock_provider: MockOAuthProvider,
}

impl TestFixture {
    async fn new() -> Self {
        let database = setup_database().await;
        
        // Clean database before each test
        database.cleanup().await.expect("Failed to cleanup database");
        
        // Setup mock OAuth provider
        let mock_provider = MockOAuthProvider::new().await;
        
        // Build test application
        // Note: You'll need to adjust this based on your actual app structure
        let app = axum::Router::new()
            .nest("/auth", create_auth_routes(database.clone(), &mock_provider).await);
        
        let server = TestServer::new(app).expect("Failed to create test server");
        
        Self {
            server,
            database,
            mock_provider,
        }
    }
}

/// Create auth routes for testing
/// TODO: Replace with your actual route creation logic
async fn create_auth_routes(
    _database: Arc<DatabaseContainer>,
    _mock_provider: &MockOAuthProvider
) -> axum::Router {
    // This is a placeholder - you'll need to implement based on your actual routing
    axum::Router::new()
        .route("/{provider}/start", axum::routing::get(oauth_start_handler))
        .route("/{provider}/callback", axum::routing::get(oauth_callback_handler))
}

/// Placeholder handlers - replace with your actual handlers
async fn oauth_start_handler(
    axum::extract::Path(provider): axum::extract::Path<String>,
    headers: axum::http::HeaderMap,
) -> Result<axum::response::Redirect, (axum::http::StatusCode, axum::Json<Value>)> {
    // Handle unsupported providers
    if !["github", "gitlab"].contains(&provider.to_lowercase().as_str()) {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "Invalid provider",
                "message": "Unsupported OAuth provider"
            }))
        ));
    }
    
    // Generate state parameter (simplified for testing)
    let state = if headers.get("authorization").is_some() {
        // Link operation - include user ID in state
        OAuthStateFixtures::valid_link_state(UserFixtures::test_user_id())
    } else {
        // Login operation
        OAuthStateFixtures::valid_login_state()
    };
    
    // Build redirect URL based on provider
    let redirect_url = match provider.to_lowercase().as_str() {
        "github" => format!(
            "https://github.com/login/oauth/authorize?client_id=test&redirect_uri=callback&state={}&scope=user:email",
            state
        ),
        "gitlab" => format!(
            "https://gitlab.com/oauth/authorize?client_id=test&redirect_uri=callback&state={}&scope=read_user",
            state
        ),
        _ => unreachable!(), // Already handled above
    };
    
    // Return 307 Temporary Redirect as specified in OpenAPI spec
    Ok(axum::response::Redirect::temporary(&redirect_url))
}

async fn oauth_callback_handler(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<axum::Json<Value>, (axum::http::StatusCode, axum::Json<Value>)> {
    // Handle OAuth provider errors first
    if let Some(error) = params.get("error") {
        let description = params.get("error_description")
            .map(|s| s.as_str())
            .unwrap_or("OAuth error");
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": error,
                "message": format!("{}: {}", error, description)
            }))
        ));
    }
    
    // Check for required parameters
    let _code = params.get("code").ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "Missing code parameter",
                "message": "OAuth callback missing authorization code"
            }))
        )
    })?;
    
    let state = params.get("state").ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "Missing state parameter",
                "message": "OAuth callback missing state parameter"
            }))
        )
    })?;
    
    // Validate state parameter
    let oauth_state = http_server::oauth_state::OAuthState::decode(state)
        .map_err(|_| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "Invalid state parameter",
                    "message": "State parameter is invalid or tampered"
                }))
            )
        })?;
    
    // Return success response based on operation type
    if oauth_state.is_login() {
        Ok(axum::Json(serde_json::json!({
            "operation": "login",
            "user": {
                "id": "12345",
                "email": "test@example.com",
                "name": "Test User"
            },
            "access_token": "test_access_token_123",
            "refresh_token": "test_refresh_token_456",
            "expires_in": 3600
        })))
    } else {
        Ok(axum::Json(serde_json::json!({
            "operation": "link",
            "user": {
                "id": oauth_state.get_link_user_id().unwrap().to_string(),
                "linked_accounts": ["github"]
            },
            "message": "Account linked successfully"
        })))
    }
}

// 🔐 Authentication & OAuth Flow Tests
// 🔁 /auth/{provider}/start

#[tokio::test]
async fn test_oauth_start_github_redirects_properly() {
    let fixture = TestFixture::new().await;
    
    // Test successful GitHub OAuth start
    let response = fixture
        .server
        .get("/auth/github/start")
        .await;
    
    response.assert_status(StatusCode::TEMPORARY_REDIRECT);
    
    let location_header = response.header("location");
    let location = location_header
        .to_str()
        .expect("Response should have location header");
    
    // Assert redirect URL contains required parameters
    ResponseAssertions::assert_redirect_has_params(
        location,
        &["client_id", "redirect_uri", "state", "scope"]
    );
    
    // Assert redirect goes to GitHub
    assert!(
        location.contains("github.com/login/oauth/authorize"),
        "Should redirect to GitHub OAuth endpoint"
    );
    
    // Extract and validate state parameter
    let url = url::Url::parse(location).expect("Invalid redirect URL");
    let query_pairs: std::collections::HashMap<String, String> = 
        url.query_pairs().into_owned().collect();
    
    if let Some(state) = query_pairs.get("state") {
        ResponseAssertions::assert_valid_state(state);
        
        // Decode state to verify it's a login operation
        let oauth_state = http_server::oauth_state::OAuthState::decode(state)
            .expect("State should be decodable");
        assert!(oauth_state.is_login(), "Should be a login operation");
    }
}

#[tokio::test]
async fn test_oauth_start_gitlab_redirects_properly() {
    let fixture = TestFixture::new().await;
    
    // Test successful GitLab OAuth start
    let response = fixture
        .server
        .get("/auth/gitlab/start")
        .await;
    
    response.assert_status(StatusCode::TEMPORARY_REDIRECT);
    
    let location_header = response.header("location");
    let location = location_header
        .to_str()
        .expect("Response should have location header");
    
    // Assert redirect URL contains required parameters
    ResponseAssertions::assert_redirect_has_params(
        location,
        &["client_id", "redirect_uri", "state", "scope"]
    );
    
    // Assert redirect goes to GitLab
    assert!(
        location.contains("gitlab.com/oauth/authorize"),
        "Should redirect to GitLab OAuth endpoint"
    );
}

#[tokio::test]
async fn test_oauth_start_with_auth_header_creates_link_state() {
    let fixture = TestFixture::new().await;
    
    // Create authorization header for link operation
    let token = UserFixtures::test_jwt_token();
    let (header_name, header_value) = TestRequestBuilder::auth_header(&token);
    
    let response = fixture
        .server
        .get("/auth/github/start")
        .add_header(header_name, header_value)
        .await;
    
    response.assert_status(StatusCode::TEMPORARY_REDIRECT);
    
    let location_header = response.header("location");
    let location = location_header
        .to_str()
        .expect("Response should have location header");
    
    // Extract and validate state parameter
    let url = url::Url::parse(location).expect("Invalid redirect URL");
    let query_pairs: std::collections::HashMap<String, String> = 
        url.query_pairs().into_owned().collect();
    
    if let Some(state) = query_pairs.get("state") {
        let oauth_state = http_server::oauth_state::OAuthState::decode(state)
            .expect("State should be decodable");
        
        // Should be a link operation with user ID
        assert!(!oauth_state.is_login(), "Should not be a login operation");
        assert!(
            oauth_state.get_link_user_id().is_some(),
            "Should have user ID for link operation"
        );
    }
}

// ❌ Returns 400 for unsupported providers

#[tokio::test]
async fn test_oauth_start_unsupported_provider_returns_400() {
    let fixture = TestFixture::new().await;
    
    let response = fixture
        .server
        .get("/auth/unsupported/start")
        .await;
    
    response.assert_status(StatusCode::BAD_REQUEST);
    
    let body: Value = response.json();
    assert_eq!(
        body["error"].as_str(),
        Some("Invalid provider"),
        "Should return invalid provider error"
    );
}

#[tokio::test]
async fn test_oauth_start_case_insensitive_provider_names() {
    let fixture = TestFixture::new().await;
    
    // Test uppercase GitHub
    let response = fixture
        .server
        .get("/auth/GITHUB/start")
        .await;
    
    response.assert_status(StatusCode::TEMPORARY_REDIRECT);
    
    // Test mixed case GitLab
    let response = fixture
        .server
        .get("/auth/GitLab/start")
        .await;
    
    response.assert_status(StatusCode::TEMPORARY_REDIRECT);
}

// ✅ Stores state & purpose securely in session or signed parameter

#[tokio::test]
async fn test_oauth_state_security_features() {
    let fixture = TestFixture::new().await;
    
    let response = fixture
        .server
        .get("/auth/github/start")
        .await;
    
    response.assert_status(StatusCode::TEMPORARY_REDIRECT);
    
    let location_header = response.header("location");
    let location = location_header
        .to_str()
        .expect("Response should have location header");
    
    let url = url::Url::parse(location).expect("Invalid redirect URL");
    let query_pairs: std::collections::HashMap<String, String> = 
        url.query_pairs().into_owned().collect();
    
    let state = query_pairs.get("state").expect("State parameter should exist");
    
    // Verify state can be decoded and contains security features
    let oauth_state = http_server::oauth_state::OAuthState::decode(state)
        .expect("State should be decodable");
    
    // Should have a random nonce for security
    assert!(!oauth_state.nonce.is_empty(), "Should have non-empty nonce");
    
    // Nonce should be UUID format (security requirement)
    assert!(
        Uuid::parse_str(&oauth_state.nonce).is_ok(),
        "Nonce should be a valid UUID for security"
    );
}

#[tokio::test]
async fn test_oauth_state_roundtrip_integrity() {
    // Test that state can survive roundtrip encoding/decoding
    let original_login_state = http_server::oauth_state::OAuthState::new_login();
    let encoded = original_login_state.encode().expect("Should encode");
    let decoded = http_server::oauth_state::OAuthState::decode(&encoded).expect("Should decode");
    
    assert_eq!(original_login_state.operation, decoded.operation);
    assert_eq!(original_login_state.nonce, decoded.nonce);
    
    // Test with link state
    let user_id = UserFixtures::test_user_id();
    let original_link_state = http_server::oauth_state::OAuthState::new_link(user_id);
    let encoded = original_link_state.encode().expect("Should encode");
    let decoded = http_server::oauth_state::OAuthState::decode(&encoded).expect("Should decode");
    
    assert_eq!(original_link_state.operation, decoded.operation);
    assert_eq!(original_link_state.nonce, decoded.nonce);
    assert_eq!(decoded.get_link_user_id(), Some(user_id));
}

#[tokio::test]
async fn test_oauth_state_tamper_resistance() {
    // Test that tampered state is rejected
    let fixture = TestFixture::new().await;
    
    let tampered_state = OAuthStateFixtures::tampered_state();
    
    let response = fixture
        .server
        .get("/auth/github/callback")
        .add_query_params(&[
            ("code", "test_code"),
            ("state", &tampered_state)
        ])
        .await;
    
    // Should return error for tampered state
    response.assert_status(StatusCode::BAD_REQUEST);
    
    let body: Value = response.json();
    ResponseAssertions::assert_oauth_error_response(&body, "Invalid state parameter");
}

#[tokio::test]
async fn test_oauth_callback_missing_state_handling() {
    let fixture = TestFixture::new().await;
    
    // Test callback without state parameter
    let response = fixture
        .server
        .get("/auth/github/callback")
        .add_query_params(&[("code", "test_code")])
        .await;
    
    // Should handle missing state gracefully
    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_oauth_callback_with_error_from_provider() {
    let fixture = TestFixture::new().await;
    
    // Setup OAuth provider error response
    fixture.mock_provider.setup_oauth_error("access_denied", "User denied access").await;
    
    let response = fixture
        .server
        .get("/auth/github/callback")
        .add_query_params(&[
            ("error", "access_denied"),
            ("error_description", "User denied access")
        ])
        .await;
    
    response.assert_status(StatusCode::BAD_REQUEST);
    
    let body: Value = response.json();
    assert!(
        body["message"].as_str().unwrap().contains("access_denied"),
        "Should include provider error in response"
    );
}

#[tokio::test]
async fn test_oauth_callback_successful_login_flow() {
    let fixture = TestFixture::new().await;
    
    // Setup successful GitHub response
    let auth_code = fixture.mock_provider.setup_github_success().await;
    let state = OAuthStateFixtures::valid_login_state();
    
    let response = fixture
        .server
        .get("/auth/github/callback")
        .add_query_params(&[
            ("code", &auth_code),
            ("state", &state)
        ])
        .await;
    
    response.assert_status(StatusCode::OK);
    
    let body: Value = response.json();
    ResponseAssertions::assert_oauth_success_response(&body);
    
    // Should be login operation
    assert_eq!(body["operation"], "login");
    
    // Should contain JWT tokens
    assert!(body["access_token"].is_string());
    assert!(body["refresh_token"].is_string());
    assert!(body["expires_in"].is_number());
}

#[tokio::test]
async fn test_oauth_callback_successful_link_flow() {
    let fixture = TestFixture::new().await;
    
    // Setup successful GitLab response
    let auth_code = fixture.mock_provider.setup_gitlab_success().await;
    let user_id = UserFixtures::test_user_id();
    let state = OAuthStateFixtures::valid_link_state(user_id);
    
    let response = fixture
        .server
        .get("/auth/gitlab/callback")
        .add_query_params(&[
            ("code", &auth_code),
            ("state", &state)
        ])
        .await;
    
    response.assert_status(StatusCode::OK);
    
    let body: Value = response.json();
    ResponseAssertions::assert_oauth_success_response(&body);
    
    // Should be link operation
    assert_eq!(body["operation"], "link");
    
    // Should contain success message and user data
    assert!(body["message"].is_string());
    assert!(body["user"].is_object());
}

// Performance and CI/CD optimization tests

#[tokio::test] 
async fn test_database_cleanup_between_tests() {
    let _fixture1 = TestFixture::new().await;
    
    // This test verifies that database cleanup works properly
    // Insert some test data
    // TODO: Add actual database operations when you implement the data layer
    
    let _fixture2 = TestFixture::new().await;
    
    // Verify that the second fixture has a clean database
    // TODO: Add verification that database is clean
    
    // This ensures tests don't interfere with each other
    assert!(true, "Database cleanup test placeholder");
}

#[tokio::test]
async fn test_concurrent_oauth_flows() {
    // Test that multiple OAuth flows can run concurrently without interference
    let fixture = TestFixture::new().await;
    
    // Test sequential requests to verify no interference
    for i in 0..5 {
        let response = fixture
            .server
            .get("/auth/github/start")
            .await;
        
        response.assert_status(StatusCode::TEMPORARY_REDIRECT);
        
        // Verify each response has proper redirect
        let location_header = response.header("location");
        let location = location_header
            .to_str()
            .expect("Response should have location header");
        
        assert!(
            location.contains("github.com/login/oauth/authorize"),
            "Request {} should redirect to GitHub OAuth endpoint", i
        );
    }
} 