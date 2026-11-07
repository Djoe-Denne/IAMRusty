//! G-1 / T1 declarative contract, not setup/secret-resolution/discovery evidence.

mod support;
#[path = "support/validation.rs"]
mod validation;

use oauth_connect_configuration::{ProviderProfile, SecretsMode, VendorTransportSecurity};
use serde_json::json;
use support::{direct, oauth, oidc, openbao};
use validation::{accepted, change, malformed, parsed, rejected, without};

#[test]
fn direct_oauth_preserves_explicit_callbacks_scopes_and_per_instance_secrets() {
    let value = direct();
    let config = accepted(&value);
    let provider = &config.providers[0];
    assert_eq!(config.providers.len(), 1);
    assert_eq!(provider.id, "forge");
    assert_eq!(provider.client_id, "public-forge");
    assert_eq!(provider.scopes, ["user:email", "ReadProfile"]);
    assert_eq!(
        provider.redirect_uris,
        [
            "https://iam.example.invalid/iam/api/auth/forge/callback",
            "https://iam.example.invalid/iam/api/auth/forge/relink-callback"
        ]
    );
    assert!(
        provider.client_secret.as_deref() == Some("client-sentinel-forge"),
        "G-1: client secret was changed"
    );
    assert!(
        provider.hmac_secret.as_deref() == Some("hmac-sentinel-at-least-sixteen-forge"),
        "G-1: HMAC secret was changed"
    );
    assert_eq!(provider.secrets_mode, SecretsMode::Direct);
    assert_eq!(
        provider.transport_security,
        VendorTransportSecurity::Verified
    );
}

#[test]
fn oidc_and_two_oauth_instances_accept_arbitrary_logical_slugs() {
    let config =
        accepted(&json!({"providers": [oauth("forge"), oauth("enterprise"), oidc("identity")]}));
    assert_eq!(
        config
            .providers
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>(),
        ["forge", "enterprise", "identity"]
    );
    assert!(matches!(
        config.providers[0].profile,
        ProviderProfile::Oauth(_)
    ));
    assert!(matches!(
        config.providers[1].profile,
        ProviderProfile::Oauth(_)
    ));
    assert!(matches!(
        config.providers[2].profile,
        ProviderProfile::Oidc(_)
    ));
    assert!(config.providers[0].client_secret != config.providers[1].client_secret);
    assert!(config.providers[0].hmac_secret != config.providers[1].hmac_secret);
}

#[test]
fn openbao_declaration_does_not_read_the_absent_token_file_or_resolve_secrets() {
    let config = accepted(&openbao());
    let provider = &config.providers[0];
    assert_eq!(provider.secrets_mode, SecretsMode::Openbao);
    assert!(provider.client_secret.is_none() && provider.hmac_secret.is_none());
    assert!(provider.client_secret_ref.is_some() && provider.hmac_secret_ref.is_some());
    assert!(config.openbao.is_some());
}

#[test]
fn verified_https_accepts_both_explicit_token_auth_and_access_token_schemes() {
    for method in ["client_secret_post", "client_secret_basic"] {
        for scheme in ["bearer", "token"] {
            let mut value = direct();
            value["providers"][0]["token_auth_method"] = json!(method);
            value["providers"][0]["profile"]["apis"]["user"]["access_token_auth"] = json!(scheme);
            let config = accepted(&value);
            assert_eq!(
                config.providers[0].transport_security,
                VendorTransportSecurity::Verified
            );
            assert_eq!(config.providers[0].scopes, ["user:email", "ReadProfile"]);
        }
    }
}

#[test]
fn http_requires_an_explicit_transport_exception_not_a_local_hostname() {
    for url in ["http://localhost/authorize", "http://127.0.0.1/authorize"] {
        let mut value = direct();
        value["providers"][0]["profile"]["authorization_url"] = json!(url);
        rejected(&value);
        for mode in ["local_insecure", "isolated_test"] {
            value["providers"][0]["transport_security"] = json!(mode);
            let config = accepted(&value);
            assert_ne!(
                config.providers[0].transport_security,
                VendorTransportSecurity::Verified
            );
        }
    }
}

#[test]
fn slug_boundaries_one_and_fifty_ascii_letters_are_valid() {
    for id in ["z".to_owned(), "z".repeat(50)] {
        let config = accepted(&json!({"providers": [oauth(&id)]}));
        assert_eq!(config.providers[0].id, id);
    }
}

#[test]
fn empty_catalogue_and_casefold_duplicates_are_rejected() {
    rejected(&json!({"providers": []}));
    for duplicate in ["forge", "FORGE", "Forge"] {
        rejected(&json!({"providers": [oauth("forge"), oauth(duplicate)]}));
    }
}

#[test]
fn slug_rejects_empty_nonletters_unicode_and_fifty_one_bytes() {
    for id in [
        "",
        "forge1",
        "my-forge",
        "my_forge",
        "é",
        " forge",
        "forge/other",
        &"a".repeat(51),
    ] {
        rejected(&json!({"providers": [oauth(id)]}));
    }
}

#[test]
fn scopes_require_nonempty_individual_tokens_without_whitespace_or_controls() {
    for scopes in [
        json!([]),
        json!([""]),
        json!(["user email"]),
        json!(["read\twrite"]),
        json!(["read\nwrite"]),
        json!(["read\u{0000}write"]),
        json!([" read"]),
        json!(["read\u{00a0}write"]),
        json!(["read\u{007f}write"]),
    ] {
        let mut value = direct();
        value["providers"][0]["scopes"] = scopes;
        rejected(&value);
    }
}

#[test]
fn url_fields_reject_relative_credentials_fragments_and_non_https_schemes() {
    for pointer in [
        "/providers/0/profile/authorization_url",
        "/providers/0/profile/token_url",
        "/providers/0/profile/apis/user/url",
        "/providers/0/redirect_uris/0",
    ] {
        for url in [
            "/relative",
            "https://name:password-sentinel@forge.example.invalid/path",
            "https://forge.example.invalid/path#fragment",
            "ftp://forge.example.invalid/path",
        ] {
            let mut value = direct();
            change(&mut value, pointer, json!(url));
            rejected(&value);
        }
    }
}

#[test]
fn authorize_endpoint_refuses_every_reserved_oauth_pkce_and_nonce_parameter() {
    for key in [
        "state",
        "redirect_uri",
        "client_id",
        "scope",
        "response_type",
        "code_challenge",
        "code_challenge_method",
        "code_verifier",
        "nonce",
    ] {
        let mut value = direct();
        value["providers"][0]["profile"]["authorization_url"] = json!(format!(
            "https://forge.example.invalid/authorize?{key}=value-sentinel"
        ));
        rejected(&value);
    }
}

#[test]
fn exact_redirect_declarations_and_nonreserved_authorize_parameters_are_preserved() {
    let mut value = direct();
    value["providers"][0]["redirect_uris"] = json!([
        "https://iam.example.invalid/callback?tenant=one",
        "https://iam.example.invalid/relink/"
    ]);
    value["providers"][0]["profile"]["authorization_url"] =
        json!("https://forge.example.invalid/authorize?screen=login");
    let config = accepted(&value);
    assert_eq!(
        config.providers[0].redirect_uris,
        [
            "https://iam.example.invalid/callback?tenant=one",
            "https://iam.example.invalid/relink/"
        ]
    );
    let ProviderProfile::Oauth(profile) = &config.providers[0].profile else {
        panic!("G-1: OAuth profile was changed");
    };
    assert_eq!(
        profile.authorization_url,
        "https://forge.example.invalid/authorize?screen=login"
    );
    value["providers"][0]["redirect_uris"] = json!([]);
    rejected(&value);
}

#[test]
fn oauth_api_references_must_exist_and_every_api_must_be_used() {
    let mut value = direct();
    value["providers"][0]["profile"]["claims"]["sub"]["api"] = json!("absent");
    rejected(&value);
    let mut value = direct();
    value["providers"][0]["profile"]["apis"]["unused"] =
        value["providers"][0]["profile"]["apis"]["user"].clone();
    rejected(&value);
    value["providers"][0]["profile"]["apis"] = json!({});
    rejected(&value);
}

#[test]
fn oauth_claim_sources_are_required_objects_with_exact_fields() {
    for claim in ["sub", "email", "email_verified", "preferred_username"] {
        let mut value = direct();
        without(&mut value, "/providers/0/profile/claims", claim);
        malformed(&value);
        for replacement in [
            json!("$.id"),
            json!({"api": "user"}),
            json!({"json_path": "$.id"}),
            json!({"api": "user", "json_path": "$.id", "extra": true}),
        ] {
            let mut value = direct();
            value["providers"][0]["profile"]["claims"][claim] = replacement;
            malformed(&value);
        }
    }
}

#[test]
fn headers_are_fixed_allowlisted_values_not_credentials_or_templates() {
    for header in [
        "Authorization",
        "authorization",
        "Cookie",
        "Host",
        "X-IAM-HMAC",
        "X-Custom",
    ] {
        let mut value = direct();
        value["providers"][0]["profile"]["apis"]["user"]["headers"][header] =
            json!("header-sentinel");
        rejected(&value);
    }
    for template in [
        "Bearer ${access_token}",
        "{{access_token}}",
        "hello\r\nCookie: bad",
    ] {
        let mut value = direct();
        value["providers"][0]["profile"]["apis"]["user"]["headers"]["Accept"] = json!(template);
        rejected(&value);
    }
    let mut value = direct();
    value["providers"][0]["profile"]["apis"]["user"]["url"] =
        json!("https://forge.example.invalid/users/{user_id}");
    rejected(&value);
}

#[test]
fn oauth_api_does_not_admit_body_pagination_chaining_or_methods() {
    for key in ["body", "pagination", "next", "method", "template"] {
        let mut value = direct();
        value["providers"][0]["profile"]["apis"]["user"][key] = json!("forbidden-sentinel");
        malformed(&value);
    }
}

#[test]
fn oidc_requires_exact_issuer_origin_openid_and_pkce() {
    for (pointer, replacement) in [
        ("/providers/0/scopes", json!(["profile", "email"])),
        ("/providers/0/pkce_supported", json!(false)),
        ("/providers/0/profile/issuer", json!("/relative")),
        (
            "/providers/0/profile/issuer",
            json!("https://user:issuer-sentinel@identity.example.invalid"),
        ),
        (
            "/providers/0/profile/issuer",
            json!("https://identity.example.invalid/tenant#fragment"),
        ),
        ("/providers/0/profile/allowed_endpoint_origins", json!([])),
        (
            "/providers/0/profile/allowed_endpoint_origins",
            json!(["https://other.example.invalid"]),
        ),
        (
            "/providers/0/profile/allowed_endpoint_origins",
            json!(["https://identity.example.invalid/tenant"]),
        ),
    ] {
        let mut value = json!({"providers": [oidc("identity")]});
        change(&mut value, pointer, replacement);
        rejected(&value);
    }
}

#[test]
fn oauth_and_oidc_shapes_are_exclusive_and_kind_has_no_fallback() {
    for kind in ["saml", "OAuth", "unknown-sentinel"] {
        let mut value = direct();
        value["providers"][0]["profile"]["kind"] = json!(kind);
        malformed(&value);
    }
    let mut value = direct();
    value["providers"][0]["profile"]["issuer"] = json!("https://identity.example.invalid");
    malformed(&value);
    let mut value = json!({"providers": [oidc("identity")]});
    value["providers"][0]["profile"]["authorization_url"] =
        json!("https://forge.example.invalid/authorize");
    malformed(&value);
}

#[test]
fn required_modes_pkce_token_auth_and_profile_are_not_defaulted() {
    for key in [
        "secrets_mode",
        "token_auth_method",
        "pkce_supported",
        "profile",
        "client_id",
    ] {
        let mut value = direct();
        without(&mut value, "/providers/0", key);
        malformed(&value);
    }
    for key in ["secrets_mode", "token_auth_method", "transport_security"] {
        let mut value = direct();
        value["providers"][0][key] = json!("unknown-sentinel");
        malformed(&value);
    }
    let mut value = direct();
    value["providers"][0]["ca_cert_file"] = json!("explicit-vendor-roots.pem");
    // Path shape/preservation only: loading and trusting the roots belongs to setup/transport.
    assert_eq!(
        parsed(&value).providers[0].ca_cert_file.as_deref(),
        Some(std::path::Path::new("explicit-vendor-roots.pem"))
    );
    value["providers"][0]["ca_cert_file"] = json!(["not-a-path"]);
    malformed(&value);
}

#[test]
fn direct_secrets_are_required_nonblank_and_hmac_is_at_least_sixteen_utf8_bytes() {
    for key in ["client_secret", "hmac_secret"] {
        let mut value = direct();
        without(&mut value, "/providers/0", key);
        rejected(&value);
        for invalid in ["", " ", "\t\n"] {
            let mut value = direct();
            value["providers"][0][key] = json!(invalid);
            rejected(&value);
        }
    }
    let mut value = direct();
    value["providers"][0]["hmac_secret"] = json!("a".repeat(15));
    rejected(&value);
    for valid in ["a".repeat(16), "é".repeat(8)] {
        value["providers"][0]["hmac_secret"] = json!(valid);
        let config = accepted(&value);
        assert_eq!(config.providers[0].hmac_secret.as_ref().unwrap().len(), 16);
    }
}

#[test]
fn direct_and_openbao_are_exclusive_for_both_secret_slots() {
    for key in ["client_secret_ref", "hmac_secret_ref"] {
        let mut value = direct();
        value["providers"][0][key] =
            json!({"mount": "secret", "path": "reference-sentinel", "field": "key"});
        rejected(&value);
    }
    for key in ["client_secret", "hmac_secret"] {
        let mut value = openbao();
        value["providers"][0][key] = json!("direct-sentinel-valid-length");
        rejected(&value);
    }
}

#[test]
fn openbao_requires_its_config_and_both_valid_references_without_fallback() {
    let mut value = openbao();
    without(&mut value, "", "openbao");
    rejected(&value);
    for key in ["client_secret_ref", "hmac_secret_ref"] {
        let mut value = openbao();
        without(&mut value, "/providers/0", key);
        rejected(&value);
        for field in ["mount", "path", "field"] {
            let mut value = openbao();
            value["providers"][0][key][field] = json!("../reference-sentinel");
            malformed(&value);
        }
    }
}

#[test]
fn openbao_bootstrap_shape_requires_endpoint_and_explicit_authentication() {
    for key in ["endpoint", "auth"] {
        let mut value = openbao();
        without(&mut value, "/openbao", key);
        malformed(&value);
    }
    for replacement in [
        json!({"method": "ambient"}),
        json!({"method": "token_file"}),
        json!({"method": "token_file", "path": "absent-token-sentinel", "token": "ambient-sentinel"}),
    ] {
        let mut value = openbao();
        value["openbao"]["auth"] = replacement;
        malformed(&value);
    }
}

#[test]
fn one_invalid_instance_is_not_rescued_by_another_instances_credentials() {
    let mut bad = oauth("other");
    bad.as_object_mut().unwrap().remove("hmac_secret");
    rejected(&json!({"providers": [oauth("forge"), bad]}));
    let mut bad = oidc("identity");
    bad["pkce_supported"] = json!(false);
    rejected(&json!({"providers": [oauth("forge"), bad]}));
}

#[test]
fn debug_and_serialization_redact_values_and_references_even_in_lists() {
    let mut value = openbao();
    value["providers"]
        .as_array_mut()
        .unwrap()
        .push(oauth("other"));
    let config = parsed(&value);
    support::assert_redacted(&format!("{config:?} {:?}", config.providers));
    let encoded = serde_json::to_value(&config).unwrap();
    support::assert_redacted(&encoded.to_string());
    for provider in encoded["providers"].as_array().unwrap() {
        for key in [
            "client_secret",
            "hmac_secret",
            "client_secret_ref",
            "hmac_secret_ref",
        ] {
            assert!(
                provider.get(key).is_none(),
                "G-1: sensitive field was serialized"
            );
        }
    }
    assert!(encoded.get("openbao").is_none());
    assert_eq!(encoded["providers"][0]["id"], "forge");
    assert_eq!(encoded["providers"][1]["id"], "other");
}
