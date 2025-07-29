use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use hive_domain::{
    GitHubProviderService, GitHubOrgInfo, GitHubMember, SyncResult, 
    ProviderType, ExternalProviderService, ProviderInfo, DomainError
};

/// GitHub provider service implementation
pub struct GitHubProvider {
    client: reqwest::Client,
}

impl GitHubProvider {
    /// Create a new GitHub provider
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Extract GitHub organization name from config
    fn get_org_name(config: &Value) -> Result<String, DomainError> {
        config
            .get("org_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| DomainError::invalid_input("Missing 'org_name' in GitHub configuration"))
    }

    /// Extract GitHub access token from config
    fn get_access_token(config: &Value) -> Result<String, DomainError> {
        config
            .get("access_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| DomainError::invalid_input("Missing 'access_token' in GitHub configuration"))
    }

    /// Get GitHub API base URL from config
    fn get_base_url(config: &Value) -> String {
        config
            .get("base_url")
            .and_then(|v| v.as_str())
            .unwrap_or("https://api.github.com")
            .to_string()
    }
}

#[async_trait]
impl GitHubProviderService for GitHubProvider {
    /// Test connection to GitHub with provided configuration
    async fn test_connection(&self, config: &Value) -> Result<(), DomainError> {
        let _org_name = Self::get_org_name(config)?;
        let _access_token = Self::get_access_token(config)?;
        let _base_url = Self::get_base_url(config);

        // TODO: Implement actual GitHub API connection test
        // For now, just validate the configuration format
        Ok(())
    }

    /// Sync organization members from GitHub
    async fn sync_members(
        &self,
        config: &Value,
        _organization_id: &Uuid,
    ) -> Result<SyncResult, DomainError> {
        let _org_name = Self::get_org_name(config)?;
        let _access_token = Self::get_access_token(config)?;

        // TODO: Implement actual GitHub members sync
        // For now, return empty result
        Ok(SyncResult::new())
    }

    /// Get organization information from GitHub
    async fn get_organization_info(&self, config: &Value) -> Result<GitHubOrgInfo, DomainError> {
        let org_name = Self::get_org_name(config)?;
        let _access_token = Self::get_access_token(config)?;

        // TODO: Implement actual GitHub API call
        // For now, return mock data
        Ok(GitHubOrgInfo {
            login: org_name.clone(),
            name: Some(org_name),
            description: None,
            avatar_url: None,
            public_members: 0,
            private_members: 0,
        })
    }

    /// Get members from GitHub organization
    async fn get_members(&self, config: &Value) -> Result<Vec<GitHubMember>, DomainError> {
        let _org_name = Self::get_org_name(config)?;
        let _access_token = Self::get_access_token(config)?;

        // TODO: Implement actual GitHub API call
        // For now, return empty list
        Ok(vec![])
    }

    /// Check if user exists in GitHub organization
    async fn is_member(&self, config: &Value, _username: &str) -> Result<bool, DomainError> {
        let _org_name = Self::get_org_name(config)?;
        let _access_token = Self::get_access_token(config)?;

        // TODO: Implement actual GitHub API call
        // For now, return false
        Ok(false)
    }
}

#[async_trait]
impl ExternalProviderService for GitHubProvider {
    /// Get the provider type this service handles
    fn provider_type(&self) -> ProviderType {
        ProviderType::GitHub
    }

    /// Test connection with provided configuration
    async fn test_connection(&self, config: &Value) -> Result<(), DomainError> {
        GitHubProviderService::test_connection(self, config).await
    }

    /// Sync members/users from the external provider
    async fn sync_members(
        &self,
        config: &Value,
        organization_id: &Uuid,
    ) -> Result<SyncResult, DomainError> {
        GitHubProviderService::sync_members(self, config, organization_id).await
    }

    /// Get basic information about the external organization/group/space
    async fn get_provider_info(&self, config: &Value) -> Result<ProviderInfo, DomainError> {
        let github_info = self.get_organization_info(config).await?;
        
        Ok(ProviderInfo {
            name: github_info.name.unwrap_or(github_info.login),
            description: github_info.description,
            avatar_url: github_info.avatar_url,
            member_count: Some(github_info.public_members + github_info.private_members),
            metadata: serde_json::to_value(&github_info).unwrap_or_default(),
        })
    }

    /// Validate the provider configuration
    async fn validate_config(&self, config: &Value) -> Result<(), DomainError> {
        // Validate required fields
        Self::get_org_name(config)?;
        Self::get_access_token(config)?;
        
        // Validate base_url if provided
        if let Some(base_url) = config.get("base_url") {
            if let Some(url_str) = base_url.as_str() {
                if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
                    return Err(DomainError::invalid_input("Invalid base_url format"));
                }
            }
        }

        Ok(())
    }
}

impl Default for GitHubProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_provider_type() {
        let provider = GitHubProvider::new();
        assert!(matches!(provider.provider_type(), ProviderType::GitHub));
    }

    #[tokio::test]
    async fn test_validate_config_valid() {
        let provider = GitHubProvider::new();
        let config = json!({
            "org_name": "test-org",
            "access_token": "ghp_test123",
            "base_url": "https://api.github.com"
        });

        let result = provider.validate_config(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_config_missing_org_name() {
        let provider = GitHubProvider::new();
        let config = json!({
            "access_token": "ghp_test123"
        });

        let result = provider.validate_config(&config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_config_missing_access_token() {
        let provider = GitHubProvider::new();
        let config = json!({
            "org_name": "test-org"
        });

        let result = provider.validate_config(&config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_config_invalid_base_url() {
        let provider = GitHubProvider::new();
        let config = json!({
            "org_name": "test-org",
            "access_token": "ghp_test123",
            "base_url": "invalid-url"
        });

        let result = provider.validate_config(&config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_test_connection() {
        let provider = GitHubProvider::new();
        let config = json!({
            "org_name": "test-org",
            "access_token": "ghp_test123"
        });

        let result = provider.test_connection(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_organization_info() {
        let provider = GitHubProvider::new();
        let config = json!({
            "org_name": "test-org",
            "access_token": "ghp_test123"
        });

        let result = provider.get_organization_info(&config).await;
        assert!(result.is_ok());
        
        let info = result.unwrap();
        assert_eq!(info.login, "test-org");
        assert_eq!(info.name, Some("test-org".to_string()));
    }

    #[tokio::test]
    async fn test_sync_members() {
        let provider = GitHubProvider::new();
        let config = json!({
            "org_name": "test-org",
            "access_token": "ghp_test123"
        });
        let org_id = Uuid::new_v4();

        let result = provider.sync_members(&config, &org_id).await;
        assert!(result.is_ok());
        
        let sync_result = result.unwrap();
        assert_eq!(sync_result.members_processed, 0);
    }

    #[tokio::test]
    async fn test_get_members() {
        let provider = GitHubProvider::new();
        let config = json!({
            "org_name": "test-org",
            "access_token": "ghp_test123"
        });

        let result = provider.get_members(&config).await;
        assert!(result.is_ok());
        
        let members = result.unwrap();
        assert!(members.is_empty());
    }

    #[tokio::test]
    async fn test_is_member() {
        let provider = GitHubProvider::new();
        let config = json!({
            "org_name": "test-org",
            "access_token": "ghp_test123"
        });

        let result = provider.is_member(&config, "testuser").await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_get_provider_info() {
        let provider = GitHubProvider::new();
        let config = json!({
            "org_name": "test-org",
            "access_token": "ghp_test123"
        });

        let result = provider.get_provider_info(&config).await;
        assert!(result.is_ok());
        
        let info = result.unwrap();
        assert_eq!(info.name, "test-org");
        assert_eq!(info.member_count, Some(0));
    }
} 