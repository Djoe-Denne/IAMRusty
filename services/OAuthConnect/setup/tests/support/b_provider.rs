//! B-1/B-builder seed using the integrated public configuration shape.

use oauth_connect_configuration::ProviderConfig;
use serde_json::json;

/// A declaration only: no vendor endpoint is contacted by secret-resolution tests.
pub fn provider(id: &str) -> ProviderConfig {
    let result = serde_json::from_value(json!({
        "id": id, "client_id": format!("public-{id}"),
        "redirect_uris": [format!("https://iam.example.invalid/{id}/callback"),
            format!("https://iam.example.invalid/{id}/relink-callback")],
        "scopes": ["user:email"], "pkce_supported": true,
        "secrets_mode": "direct", "client_secret": "direct-client-sentinel",
        "hmac_secret": "direct-HMAC-sentinel-sixteen", "token_auth_method": "client_secret_post",
        "profile": {"kind": "oauth",
            "authorization_url": "https://forge.example.invalid/authorize",
            "token_url": "https://forge.example.invalid/token",
            "apis": {"user": {"url": "https://forge.example.invalid/user", "access_token_auth": "bearer"}},
            "claims": {
                "sub": {"api": "user", "json_path": "$.id"},
                "email": {"api": "user", "json_path": "$.email"},
                "email_verified": {"api": "user", "json_path": "$.verified"},
                "preferred_username": {"api": "user", "json_path": "$.login"}
            }
        }
    }));
    match result {
        Ok(value) => value,
        Err(_) => panic!("fixture must match the integrated provider declaration"),
    }
}
