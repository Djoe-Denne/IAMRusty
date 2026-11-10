//! Exercise the actual ops `JSONPath` declarations, without a vendor or a coffer.

use oauth_connect_configuration::{AppConfig, ProviderProfile};
use oauth_connect_infra::{compile_mapping, ApiResponses};
use serde_json::json;

fn catalogue() -> AppConfig {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../configuration/tests/fixtures/ops-connect.toml");
    toml::from_str(&std::fs::read_to_string(path).expect("public ops configuration"))
        .expect("typed ops catalogue")
}

#[test]
fn github_ops_mapping_selects_primary_email_without_a_public_profile_fallback() {
    let config = catalogue();
    let ProviderProfile::Oauth(profile) = &config.providers[0].profile else {
        panic!("OAuth profile required");
    };
    let mapping = compile_mapping(&profile.claims).expect("valid actual GitHub mapping");
    let mut responses = ApiResponses::from([
        (
            "user".into(),
            json!({"id": 42, "login": "synthetic", "email": "public@example.invalid"}),
        ),
        (
            "emails".into(),
            json!([
                {"email": "secondary@example.invalid", "primary": false, "verified": true},
                {"email": "primary@example.invalid", "primary": true, "verified": true}
            ]),
        ),
    ]);
    let profile = mapping
        .project(&responses)
        .expect("projected primary identity");
    assert_eq!(profile.id, "42");
    assert_eq!(profile.email.as_deref(), Some("primary@example.invalid"));
    assert!(profile.email_verified);
    responses.insert("emails".into(), json!([]));
    let profile = mapping
        .project(&responses)
        .expect("optional email is absent");
    assert!(profile.email.is_none());
    assert!(!profile.email_verified);
    responses.insert(
        "emails".into(),
        json!([
            {"email": "one@example.invalid", "primary": true, "verified": true},
            {"email": "two@example.invalid", "primary": true, "verified": true}
        ]),
    );
    assert!(mapping.project(&responses).is_err());
}

#[test]
fn simulated_ops_mapping_requires_a_boolean_verification_claim() {
    let config = catalogue();
    let ProviderProfile::Oauth(profile) = &config.providers[1].profile else {
        panic!("OAuth profile required");
    };
    let mapping = compile_mapping(&profile.claims).expect("valid actual simulated mapping");
    let mut responses = ApiResponses::from([(
        "user".into(),
        json!({"id": 123, "username": "simulated",
        "email": "simulated@example.invalid", "email_verified": true}),
    )]);
    let profile = mapping
        .project(&responses)
        .expect("projected simulated identity");
    assert_eq!(profile.id, "123");
    assert!(profile.email_verified);
    responses.get_mut("user").expect("user response")["email_verified"] = json!("true");
    assert!(mapping.project(&responses).is_err());
}
