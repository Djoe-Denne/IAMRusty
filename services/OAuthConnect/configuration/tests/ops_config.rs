//! Validate the actual public ops declaration without network or secret access.

use oauth_connect_configuration::{validate_config, AppConfig, ProviderProfile};

#[test]
fn local_ops_github_and_simulated_catalogue_matches_the_typed_service_contract() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../ops/deploy/charts/local-full/files/config/oauth-connect.toml");
    let source = std::fs::read_to_string(path).expect("public ops configuration");
    let source = source
        .replace("__GITHUB_CLIENT_ID__", "synthetic-github")
        .replace("__SIMULATED_CLIENT_ID__", "synthetic-simulated");
    let config: AppConfig = toml::from_str(&source).expect("typed ops declaration");
    validate_config(&config).expect("valid offline catalogue");
    assert_eq!(config.providers.len(), 2);
    assert_eq!(config.providers[0].id, "github");
    assert_eq!(config.providers[1].id, "simulated");
    assert!(!config.providers[0].pkce_supported);
    assert!(config.providers[1].pkce_supported);
    for provider in &config.providers {
        assert!(matches!(provider.profile, ProviderProfile::Oauth(_)));
        assert!(provider.client_secret.is_none());
        assert!(provider.hmac_secret.is_none());
    }
}
