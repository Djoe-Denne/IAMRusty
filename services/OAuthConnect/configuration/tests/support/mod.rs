//! Service-local declarative seeds and process isolation for G-1/T1.

use serde_json::{json, Value};

/// Minimal vendor-neutral OAuth declaration; expected claims are independent of production.
pub fn oauth(id: &str) -> Value {
    json!({
        "id": id,
        "client_id": format!("public-{id}"),
        "redirect_uris": [
            format!("https://iam.example.invalid/iam/api/auth/{id}/callback"),
            format!("https://iam.example.invalid/iam/api/auth/{id}/relink-callback")
        ],
        "scopes": ["user:email", "ReadProfile"],
        "pkce_supported": true,
        "secrets_mode": "direct",
        "client_secret": format!("client-sentinel-{id}"),
        "hmac_secret": format!("hmac-sentinel-at-least-sixteen-{id}"),
        "token_auth_method": "client_secret_post",
        "profile": {
            "kind": "oauth",
            "authorization_url": "https://forge.example.invalid/oauth/authorize",
            "token_url": "https://forge.example.invalid/oauth/token",
            "apis": {
                "user": {
                    "url": "https://forge.example.invalid/api/user",
                    "access_token_auth": "bearer",
                    "headers": {"Accept": "application/json", "User-Agent": "OAuthConnect-test",
                        "X-GitHub-Api-Version": "2022-11-28"}
                }
            },
            "claims": {
                "sub": {"api": "user", "json_path": "$.id"},
                "email": {"api": "user", "json_path": "$.email"},
                "email_verified": {"api": "user", "json_path": "$.verified"},
                "preferred_username": {"api": "user", "json_path": "$.login"}
            }
        }
    })
}

/// Single-instance direct catalogue, deliberately not a usable credential.
pub fn direct() -> Value {
    json!({"providers": [oauth("forge")]})
}

/// Declarative OIDC authority; no discovery or JWT material is provided.
pub fn oidc(id: &str) -> Value {
    let mut provider = oauth(id);
    provider["scopes"] = json!(["openid", "profile", "email"]);
    provider["profile"] = json!({
        "kind": "oidc", "issuer": "https://identity.example.invalid/tenant",
        "allowed_endpoint_origins": ["https://identity.example.invalid"]
    });
    provider
}

/// Unresolvable test references and absent token file must still load declaratively.
pub fn openbao() -> Value {
    let mut value = direct();
    let provider = value["providers"][0].as_object_mut().unwrap();
    provider.remove("client_secret");
    provider.remove("hmac_secret");
    provider.insert("secrets_mode".into(), json!("openbao"));
    for (key, field) in [("client_secret_ref", "client"), ("hmac_secret_ref", "hmac")] {
        provider.insert(
            key.into(),
            json!({
                "mount": "secret", "path": "reference-sentinel/oauth", "field": field
            }),
        );
    }
    value["openbao"] = json!({
        "endpoint": "https://bao.example.invalid",
        "auth": {"method": "token_file", "path": "absent-token-sentinel"}
    });
    value
}

/// A single marker covers values, reference components, invalid enum values and token paths.
pub fn assert_redacted(text: &str) {
    assert!(
        !text.contains("sentinel"),
        "G-1: diagnostic leaked a secret/reference marker"
    );
}
