//! G-1 / T1: actual SDK source assembly, with child-process-only env/CWD.

#[path = "support/loader.rs"]
mod loader;
mod support;

use loader::{isolated, loader_error, loader_ok};
use oauth_connect_configuration::{ProviderProfile, SecretsMode};
use serde_json::json;
use support::{direct, oauth, oidc, openbao};

#[test]
fn real_loader_reads_run_env_file_and_local_dotenv_without_vendor_defaults() {
    isolated(
        "real_loader_reads_run_env_file_and_local_dotenv_without_vendor_defaults",
        "minimal",
        &direct(),
        &[],
        |result| {
            let config = loader_ok(result);
            assert_eq!(config.providers.len(), 1);
            assert_eq!(config.providers[0].id, "forge");
            assert_eq!(config.providers[0].client_id, "public-forge");
            assert_eq!(config.providers[0].scopes, ["user:email", "ReadProfile"]);
            assert_eq!(config.logging.level, "warn");
        },
    );
}

#[test]
fn real_loader_preserves_three_instances_two_same_profile_and_their_exact_callbacks() {
    let value = json!({"providers": [oauth("forge"), oauth("enterprise"), oidc("identity") ]});
    isolated(
        "real_loader_preserves_three_instances_two_same_profile_and_their_exact_callbacks",
        "multi",
        &value,
        &[],
        |result| {
            let config = loader_ok(result);
            assert_eq!(
                config
                    .providers
                    .iter()
                    .map(|p| p.id.as_str())
                    .collect::<Vec<_>>(),
                ["forge", "enterprise", "identity"]
            );
            assert!(matches!(
                config.providers[2].profile,
                ProviderProfile::Oidc(_)
            ));
            assert_eq!(
                config.providers[1].redirect_uris,
                [
                    "https://iam.example.invalid/iam/api/auth/enterprise/callback",
                    "https://iam.example.invalid/iam/api/auth/enterprise/relink-callback"
                ]
            );
            assert!(config.providers[0].hmac_secret != config.providers[1].hmac_secret);
        },
    );
}

#[test]
fn real_loader_openbao_is_only_declarative_and_accepts_an_unreadable_token_path() {
    isolated(
        "real_loader_openbao_is_only_declarative_and_accepts_an_unreadable_token_path",
        "bao",
        &openbao(),
        &[],
        |result| {
            let config = loader_ok(result);
            assert_eq!(config.providers[0].secrets_mode, SecretsMode::Openbao);
            assert!(config.providers[0].client_secret_ref.is_some());
            assert!(config.providers[0].hmac_secret_ref.is_some());
            assert!(config.providers[0].client_secret.is_none());
        },
    );
}

#[test]
fn real_loader_indexed_env_override_changes_only_the_target_instance() {
    let value = json!({"providers": [oauth("forge"), oauth("enterprise")]});
    isolated(
        "real_loader_indexed_env_override_changes_only_the_target_instance",
        "index",
        &value,
        &[("OAUTH_CONNECT_PROVIDERS[0]__CLIENT_ID", "public-overridden")],
        |result| {
            let config = loader_ok(result);
            assert_eq!(config.providers.len(), 2);
            assert_eq!(config.providers[0].client_id, "public-overridden");
            assert_eq!(config.providers[1].client_id, "public-enterprise");
            assert_eq!(config.providers[0].scopes, ["user:email", "ReadProfile"]);
            assert_eq!(config.providers[1].scopes, ["user:email", "ReadProfile"]);
        },
    );
}

#[test]
fn real_loader_double_underscore_nested_override_uses_sdk_boolean_parsing() {
    let value = json!({"providers": [oauth("forge"), oauth("enterprise")]});
    isolated(
        "real_loader_double_underscore_nested_override_uses_sdk_boolean_parsing",
        "bool",
        &value,
        &[("OAUTH_CONNECT_PROVIDERS[1]__PKCE_SUPPORTED", "false")],
        |result| {
            let config = loader_ok(result);
            assert!(config.providers[0].pkce_supported);
            assert!(!config.providers[1].pkce_supported);
            assert_eq!(config.providers[0].client_id, "public-forge");
            assert_eq!(config.providers[1].client_id, "public-enterprise");
        },
    );
}

#[test]
fn sdk_indexed_override_preflight_is_supported_without_service_validation() {
    let value = json!({"providers": [oauth("forge"), oauth("enterprise")]});
    // T0's service validation refuses everything. Probe the *same* SDK loader separately
    // so an unsupported index syntax cannot be misreported as missing service validation.
    isolated(
        "sdk_indexed_override_preflight_is_supported_without_service_validation",
        "sdk-index",
        &value,
        &[
            ("OAUTH_CONNECT_PROVIDERS[0]__CLIENT_ID", "public-overridden"),
            ("OAUTH_CONNECT_PROVIDERS[1]__PKCE_SUPPORTED", "false"),
        ],
        |_| {
            let config = loader_ok(rustycog::config::load_config_fresh_strict::<
                oauth_connect_configuration::AppConfig,
            >());
            assert_eq!(config.providers.len(), 2);
            assert_eq!(config.providers[0].client_id, "public-overridden");
            assert_eq!(config.providers[1].client_id, "public-enterprise");
            assert!(config.providers[0].pkce_supported);
            assert!(!config.providers[1].pkce_supported);
            assert_eq!(config.providers[0].scopes, ["user:email", "ReadProfile"]);
            assert_eq!(config.providers[1].scopes, ["user:email", "ReadProfile"]);
        },
    );
}

#[test]
fn real_loader_rejects_mistyped_and_unknown_indexed_overrides_without_using_file_fallback() {
    let value = json!({"providers": [oauth("forge"), oauth("enterprise")]});
    for (case, key, content) in [
        (
            "bad-bool",
            "OAUTH_CONNECT_PROVIDERS[0]__PKCE_SUPPORTED",
            "mistyped-sentinel",
        ),
        (
            "bad-list",
            "OAUTH_CONNECT_PROVIDERS[0]__SCOPES",
            "read email",
        ),
        (
            "unknown-indexed",
            "OAUTH_CONNECT_PROVIDERS[1]__UNKNOWN",
            "unknown-sentinel",
        ),
    ] {
        isolated("real_loader_rejects_mistyped_and_unknown_indexed_overrides_without_using_file_fallback",
            case, &value, &[(key, content)], loader_error);
    }
}

#[test]
fn real_loader_rejects_unknown_keys_in_files_and_its_own_environment_prefix() {
    for section in ["root", "provider", "profile", "api"] {
        let mut value = direct();
        let target = match section {
            "root" => &mut value,
            "provider" => &mut value["providers"][0],
            "profile" => &mut value["providers"][0]["profile"],
            _ => &mut value["providers"][0]["profile"]["apis"]["user"],
        };
        target["unknown"] = json!("unknown-file-sentinel");
        isolated(
            "real_loader_rejects_unknown_keys_in_files_and_its_own_environment_prefix",
            section,
            &value,
            &[],
            loader_error,
        );
    }
    isolated(
        "real_loader_rejects_unknown_keys_in_files_and_its_own_environment_prefix",
        "env",
        &direct(),
        &[("OAUTH_CONNECT_UNKNOWN", "unknown-env-sentinel")],
        loader_error,
    );
}

#[test]
fn real_loader_rejects_legacy_sections_loaded_from_file_but_ignores_ambient_other_prefix() {
    let mut legacy = direct();
    legacy["github"] = json!({"client_secret": "legacy-sentinel"});
    isolated(
        "real_loader_rejects_legacy_sections_loaded_from_file_but_ignores_ambient_other_prefix",
        "legacy-file",
        &legacy,
        &[],
        loader_error,
    );
    isolated(
        "real_loader_rejects_legacy_sections_loaded_from_file_but_ignores_ambient_other_prefix",
        "ambient",
        &direct(),
        &[("GITHUB_CONNECT_GITHUB__CLIENT_ID", "ignored-public-client")],
        |result| {
            let config = loader_ok(result);
            assert_eq!(config.providers.len(), 1);
            assert_eq!(config.providers[0].client_id, "public-forge");
        },
    );
}

#[test]
fn real_loader_canonicalizes_ascii_slug_case_without_reinterpreting_scopes() {
    let value = json!({"providers": [oauth("FoRgE")]});
    isolated(
        "real_loader_canonicalizes_ascii_slug_case_without_reinterpreting_scopes",
        "casefold",
        &value,
        &[],
        |result| {
            let config = loader_ok(result);
            assert_eq!(config.providers[0].id, "forge");
            assert_eq!(config.providers[0].scopes, ["user:email", "ReadProfile"]);
            assert_eq!(config.providers[0].client_id, "public-FoRgE");
        },
    );
}

#[test]
fn real_loader_never_publishes_duplicate_scopes_whether_rejected_or_deduplicated() {
    let mut value = direct();
    value["providers"][0]["scopes"] = json!(["ReadProfile", "ReadProfile", "user:email"]);
    // Contract requires a deduplicated catalogue; it does not prescribe rejection vs normalization.
    isolated(
        "real_loader_never_publishes_duplicate_scopes_whether_rejected_or_deduplicated",
        "dedup",
        &value,
        &[],
        |result| match result {
            Ok(config) => {
                let scopes = &config.providers[0].scopes;
                assert_eq!(scopes.len(), 2);
                assert_eq!(
                    scopes
                        .iter()
                        .filter(|scope| scope.as_str() == "ReadProfile")
                        .count(),
                    1
                );
                assert_eq!(
                    scopes
                        .iter()
                        .filter(|scope| scope.as_str() == "user:email")
                        .count(),
                    1
                );
            }
            Err(error) => loader_error(Err(error)),
        },
    );
}

#[test]
fn real_loader_invalid_instance_refuses_the_whole_catalogue_without_secret_fallback() {
    let mut value = openbao();
    value["providers"]
        .as_array_mut()
        .unwrap()
        .push(oauth("other"));
    for key in ["client_secret", "hmac_secret"] {
        let mut mixed = value.clone();
        mixed["providers"][0][key] = json!("mixed-direct-sentinel-valid-length");
        isolated(
            "real_loader_invalid_instance_refuses_the_whole_catalogue_without_secret_fallback",
            key,
            &mixed,
            &[],
            loader_error,
        );
    }
    value["providers"][0]["client_secret_ref"]["path"] = json!("../invalid-reference-sentinel");
    isolated(
        "real_loader_invalid_instance_refuses_the_whole_catalogue_without_secret_fallback",
        "reference",
        &value,
        &[],
        loader_error,
    );
}

#[test]
fn real_loader_rejects_unknown_enum_missing_required_field_and_foreign_profile_fields_redacted() {
    for case in ["mode", "kind", "token-auth", "missing", "foreign"] {
        let mut value = direct();
        match case {
            "mode" => value["providers"][0]["secrets_mode"] = json!("unknown-mode-sentinel"),
            "kind" => value["providers"][0]["profile"]["kind"] = json!("unknown-profile-sentinel"),
            "token-auth" => {
                value["providers"][0]["token_auth_method"] = json!("unknown-auth-sentinel")
            }
            "missing" => {
                value["providers"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("pkce_supported");
            }
            _ => {
                value["providers"][0]["profile"]["issuer"] =
                    json!("https://foreign-sentinel.invalid")
            }
        }
        isolated("real_loader_rejects_unknown_enum_missing_required_field_and_foreign_profile_fields_redacted",
            case, &value, &[], loader_error);
    }
}
