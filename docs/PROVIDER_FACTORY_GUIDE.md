# OAuth Provider Factory Guide

This guide explains how to add new OAuth providers (like Google, Microsoft, etc.) to the IAM service using the factory pattern.

## Overview

The provider factory pattern allows easy addition of new OAuth providers while keeping the code maintainable and following DRY principles. The factory centralizes provider-specific logic and provides a unified interface for authentication services.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     AuthProviderFactory                      │
├─────────────────────────────────────────────────────────────┤
│ - github_auth: Arc<GH>                                      │
│ - gitlab_auth: Arc<GL>                                      │
├─────────────────────────────────────────────────────────────┤
│ + get_auth_service(provider) -> Arc<dyn AuthService>        │
└─────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                    AuthServiceWrapper                        │
├─────────────────────────────────────────────────────────────┤
│ Unifies different auth service error types                  │
│ Maps provider-specific errors to AuthError                  │
└─────────────────────────────────────────────────────────────┘
```

## Adding a New Provider

Follow these steps to add a new OAuth provider (e.g., Google):

### 1. Update the Provider Enum

First, add the new provider to the domain entity:

```rust
// domain/src/entity/provider.rs
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Provider {
    GitHub,
    GitLab,
    Google, // New provider
}

impl Provider {
    pub fn from_str(provider: &str) -> Option<Self> {
        match provider.to_lowercase().as_str() {
            "github" => Some(Provider::GitHub),
            "gitlab" => Some(Provider::GitLab),
            "google" => Some(Provider::Google), // Add mapping
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::GitHub => "github",
            Provider::GitLab => "gitlab",
            Provider::Google => "google", // Add string representation
        }
    }
}
```

### 2. Create the Auth Service Implementation

Create a new auth service for your provider:

```rust
// infra/src/auth/google.rs
use application::auth::{AuthService, AuthError};
use domain::entity::provider::{Provider, ProviderTokens, ProviderUserProfile};
use async_trait::async_trait;

pub struct GoogleAuthService {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    // OAuth client or other dependencies
}

impl GoogleAuthService {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri,
        }
    }
}

#[async_trait]
impl AuthService for GoogleAuthService {
    type Error = GoogleAuthError; // Your custom error type

    fn provider(&self) -> Provider {
        Provider::Google
    }

    fn generate_authorize_url(&self) -> String {
        // Generate Google OAuth authorization URL
        format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile",
            self.client_id, self.redirect_uri
        )
    }

    async fn exchange_code(
        &self,
        code: String,
        redirect_uri: String,
    ) -> Result<(ProviderTokens, ProviderUserProfile), Self::Error> {
        // Exchange authorization code for tokens
        // Make HTTP request to Google token endpoint
        // Parse response and extract user profile
        todo!()
    }
}
```

### 3. Update the Factory

Add the new provider to the factory:

```rust
// application/src/usecase/factory/auth_provider.rs
pub struct AuthProviderFactory<GH, GL, GG> // Add generic parameter
where
    GH: AuthService + 'static,
    GL: AuthService + 'static,
    GG: AuthService + 'static, // New constraint
{
    github_auth: Arc<GH>,
    gitlab_auth: Arc<GL>,
    google_auth: Arc<GG>, // New field
}

impl<GH, GL, GG> AuthProviderFactory<GH, GL, GG>
where
    GH: AuthService + 'static,
    GL: AuthService + 'static,
    GG: AuthService + 'static,
{
    pub fn new(
        github_auth: Arc<GH>,
        gitlab_auth: Arc<GL>,
        google_auth: Arc<GG>, // New parameter
    ) -> Self {
        Self {
            github_auth,
            gitlab_auth,
            google_auth,
        }
    }

    pub fn get_auth_service(&self, provider: Provider) -> Arc<dyn AuthService<Error = AuthError>> {
        match provider {
            Provider::GitHub => Arc::new(AuthServiceWrapper::new(self.github_auth.clone())),
            Provider::GitLab => Arc::new(AuthServiceWrapper::new(self.gitlab_auth.clone())),
            Provider::Google => Arc::new(AuthServiceWrapper::new(self.google_auth.clone())), // New case
        }
    }
}
```

### 4. Update Use Cases

Update the use case implementations to include the new provider:

```rust
// application/src/usecase/login.rs
pub struct LoginUseCaseImpl<GH, GL, GG, UR, UER, TR, RR, TS> // Add GG
where
    GH: AuthService + 'static,
    GL: AuthService + 'static,
    GG: AuthService + 'static, // New
    // ... other constraints
{
    auth_factory: Arc<AuthProviderFactory<GH, GL, GG>>, // Updated
    // ... other fields
}

// Update constructor and implementation similarly
```

### 5. Wire Everything Together

Update the dependency injection setup:

```rust
// setup/src/lib.rs or your DI configuration
let google_auth = Arc::new(GoogleAuthService::new(
    config.google_client_id.clone(),
    config.google_client_secret.clone(),
    config.google_redirect_uri.clone(),
));

let auth_factory = Arc::new(AuthProviderFactory::new(
    github_auth.clone(),
    gitlab_auth.clone(),
    google_auth.clone(), // Add new service
));

// Use the factory when creating use cases
let login_use_case = LoginUseCaseImpl::new_with_factory(
    auth_factory.clone(),
    user_repo.clone(),
    user_email_repo.clone(),
    token_repo.clone(),
    refresh_token_repo.clone(),
    token_service.clone(),
);
```

### 6. Update HTTP Routes

Add routes for the new provider:

```rust
// http/src/router.rs
.route("/auth/google/start", get(auth_handler::oauth_start))
.route("/auth/google/callback", get(auth_handler::oauth_callback))
```

## Best Practices

1. **Error Handling**: Map provider-specific errors to the common `AuthError` type in the wrapper.

2. **Configuration**: Store OAuth credentials in environment variables or secure configuration.

3. **Testing**: Create mock implementations of your auth service for testing:
   ```rust
   pub struct MockGoogleAuthService {
       // Mock implementation
   }
   ```

4. **Validation**: Ensure email is returned from the provider, as it's required for user linking.

5. **Token Storage**: The factory pattern ensures all providers store tokens consistently through the `TokenRepository`.

## Common Patterns

### Handling Provider-Specific Data

If providers return different user profile formats, normalize them in your auth service:

```rust
async fn exchange_code(...) -> Result<(ProviderTokens, ProviderUserProfile), Self::Error> {
    let google_response = // ... fetch from Google
    
    // Normalize to common format
    let profile = ProviderUserProfile {
        id: google_response.sub, // Google uses 'sub' for user ID
        username: google_response.email.split('@').next().unwrap_or_default().to_string(),
        email: Some(google_response.email),
        avatar_url: google_response.picture,
    };
    
    Ok((tokens, profile))
}
```

### Adding Provider-Specific Features

If a provider offers unique features, you can extend the auth service trait:

```rust
pub trait AuthServiceExt: AuthService {
    async fn refresh_token(&self, refresh_token: &str) -> Result<ProviderTokens, Self::Error>;
}
```

## Troubleshooting

### Compilation Errors

If you get "the trait bound `GG: AuthService` is not satisfied" errors:
- Ensure your new auth service implements the `AuthService` trait
- Add necessary trait bounds to all generic parameters

### Runtime Errors

If provider authentication fails:
- Check OAuth app configuration (redirect URIs, scopes)
- Verify network connectivity to provider endpoints
- Enable debug logging for HTTP requests

## Example: Complete Google Provider Implementation

Here's a minimal working example:

```rust
// infra/src/auth/google.rs
use application::auth::{AuthService, AuthError};
use domain::entity::provider::{Provider, ProviderTokens, ProviderUserProfile};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum GoogleAuthError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

pub struct GoogleAuthService {
    client: Client,
    client_id: String,
    client_secret: String,
}

#[async_trait]
impl AuthService for GoogleAuthService {
    type Error = GoogleAuthError;

    fn provider(&self) -> Provider {
        Provider::Google
    }

    fn generate_authorize_url(&self) -> String {
        format!(
            "https://accounts.google.com/o/oauth2/v2/auth?\
            client_id={}&\
            redirect_uri=https://localhost:3000/auth/google/callback&\
            response_type=code&\
            scope=openid%20email%20profile&\
            access_type=offline",
            self.client_id
        )
    }

    async fn exchange_code(
        &self,
        code: String,
        redirect_uri: String,
    ) -> Result<(ProviderTokens, ProviderUserProfile), Self::Error> {
        // Token exchange
        let token_response = self.client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("code", &code),
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
                ("redirect_uri", &redirect_uri),
                ("grant_type", &"authorization_code".to_string()),
            ])
            .send()
            .await?
            .json::<TokenResponse>()
            .await?;

        // Get user info
        let user_info = self.client
            .get("https://www.googleapis.com/oauth2/v3/userinfo")
            .bearer_auth(&token_response.access_token)
            .send()
            .await?
            .json::<UserInfo>()
            .await?;

        let tokens = ProviderTokens {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_in: Some(token_response.expires_in),
        };

        let profile = ProviderUserProfile {
            id: user_info.sub,
            username: user_info.email.split('@').next().unwrap_or_default().to_string(),
            email: Some(user_info.email),
            avatar_url: user_info.picture,
        };

        Ok((tokens, profile))
    }
}
```

## Conclusion

The factory pattern makes adding new OAuth providers straightforward:
1. Define the provider in the enum
2. Implement the `AuthService` trait
3. Add it to the factory
4. Update use cases to include the new generic parameter
5. Wire everything in your DI setup

This approach keeps provider-specific code isolated while maintaining a consistent interface across all providers. 