mod fixtures;

use fixtures::{GitHubFixtures, GitLabFixtures};
use fixtures::github::*;
use fixtures::gitlab::*;

#[tokio::test]
async fn example_github_oauth_flow() {
    // Create GitHub service fixture
    let github = GitHubFixtures::service().await;
    
    // Mock successful token exchange
    github
        .oauth_token(
            200,
            GitHubTokenRequest::valid(),
            GitHubTokenResponse::success(),
        )
        .await;
    
    // Mock successful user profile fetch
    github
        .user_profile(
            200,
            GitHubUserRequest::authenticated(),
            GitHubUser::arthur(),
        )
        .await;
    
    // Your test logic would go here...
    // For example, making HTTP requests to your OAuth endpoints
    // and verifying they interact correctly with the mocked GitHub API
    
    println!("✅ GitHub OAuth flow mocked successfully");
    println!("🔗 Mock server URL: {}", github.base_url());
}

#[tokio::test]
async fn example_gitlab_oauth_flow() {
    // Create GitLab service fixture
    let gitlab = GitLabFixtures::service().await;
    
    // Mock successful token exchange
    gitlab
        .oauth_token(
            200,
            GitLabTokenRequest::valid(),
            GitLabTokenResponse::success(),
        )
        .await;
    
    // Mock successful user profile fetch
    gitlab
        .user_profile(
            200,
            GitLabUserRequest::authenticated(),
            GitLabUser::alice(),
        )
        .await;
    
    println!("✅ GitLab OAuth flow mocked successfully");
    println!("🔗 Mock server URL: {}", gitlab.base_url());
}

#[tokio::test]
async fn example_error_scenarios() {
    let github = GitHubFixtures::service().await;
    
    // Mock various error scenarios using individual calls
    github.setup_failed_token_exchange_invalid_code().await;
    github.setup_failed_user_profile_unauthorized().await;
    github.setup_rate_limit_exceeded().await;
    
    println!("✅ GitHub error scenarios mocked successfully");
}

#[tokio::test]
async fn example_custom_user_data() {
    let github = GitHubFixtures::service().await;
    
    // Create custom user data using builder pattern
    let custom_user = GitHubUser::create()
        .id(99999)
        .login("custom_test_user")
        .email(Some("custom@test.com"))
        .avatar_url(None::<String>)
        .build();
    
    // Mock with custom user data
    github
        .user_profile(
            200,
            GitHubUserRequest::authenticated(),
            custom_user,
        )
        .await;
    
    println!("✅ Custom user data mocked successfully");
}

#[tokio::test]
async fn example_multi_provider_setup() {
    // Setup both GitHub and GitLab mocks in the same test
    let github = GitHubFixtures::service().await;
    let gitlab = GitLabFixtures::service().await;
    
    // Setup GitHub success flow
    github.setup_successful_token_exchange().await;
    github.setup_successful_user_profile_arthur().await;
    
    // Setup GitLab success flow
    gitlab.setup_successful_token_exchange().await;
    gitlab.setup_successful_user_profile_alice().await;
    
    println!("✅ Multi-provider OAuth flows mocked successfully");
    println!("🔗 GitHub mock URL: {}", github.base_url());
    println!("🔗 GitLab mock URL: {}", gitlab.base_url());
} 