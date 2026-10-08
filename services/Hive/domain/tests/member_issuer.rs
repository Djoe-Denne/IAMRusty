//! Hive domain unit tests for member issuer uniqueness (ADR-0305).

use chrono::Utc;
use hive_domain::entity::OrganizationMember;
use hive_events::{MemberJoinedEvent, MemberRemovedEvent};
use uuid::Uuid;

#[test]
fn member_carries_issuer_for_principal_uniqueness() {
    let org = Uuid::new_v4();
    let user = Uuid::new_v4();
    let a = OrganizationMember::new(org, user, "http://127.0.0.1/iam", None);
    let b = OrganizationMember::new(org, user, "iamrusty", None);
    assert_eq!(a.organization_id, b.organization_id);
    assert_eq!(a.user_id, b.user_id);
    assert_ne!(a.issuer, b.issuer);
}

#[test]
fn member_events_serialize_optional_issuer() {
    let org = Uuid::new_v4();
    let user = Uuid::new_v4();
    let mut joined = MemberJoinedEvent::new(org, "Acme".into(), user, vec![], Utc::now());
    joined.issuer = Some("http://127.0.0.1/iam".into());
    let json = serde_json::to_value(&joined).expect("serialize");
    assert_eq!(json["issuer"], "http://127.0.0.1/iam");

    let legacy = MemberRemovedEvent::new(
        org,
        "Acme".into(),
        user,
        "u@example.com".into(),
        Uuid::new_v4(),
        Utc::now(),
    );
    let legacy_json = serde_json::to_value(&legacy).expect("serialize");
    assert!(legacy_json.get("issuer").is_none());
}

fn member_matches_principal(
    stored: &OrganizationMember,
    org: Uuid,
    principal_iss: &str,
    user: Uuid,
) -> bool {
    stored.organization_id == org
        && stored.user_id == user
        && hive_domain::platform_issuer_aliases(principal_iss).contains(&stored.issuer)
}

#[test]
fn member_stored_iamrusty_found_when_querying_platform_url() {
    let org = Uuid::new_v4();
    let user = Uuid::new_v4();
    let stored = OrganizationMember::new(org, user, hive_domain::HISTORICAL_PLATFORM_ISSUER, None);
    let query_iss = "http://127.0.0.1:8081/iam";
    assert!(member_matches_principal(&stored, org, query_iss, user));
}

#[test]
fn member_stored_other_org_issuer_not_found_for_platform_url() {
    let org = Uuid::new_v4();
    let user = Uuid::new_v4();
    let stored = OrganizationMember::new(org, user, "http://127.0.0.1:8081/iam/orgs/other", None);
    let query_iss = "http://127.0.0.1:8081/iam";
    assert!(!member_matches_principal(&stored, org, query_iss, user));
}

#[test]
fn add_member_treats_either_platform_issuer_as_already_member() {
    let requested = "http://127.0.0.1:8081/iam";
    let issuers = hive_domain::membership_lookup_issuers(requested);
    assert_eq!(issuers[0], requested);
    assert!(issuers
        .iter()
        .any(|iss| iss == hive_domain::HISTORICAL_PLATFORM_ISSUER));
}

#[test]
fn platform_url_issuer_aliases_include_iamrusty() {
    let aliases = hive_domain::platform_issuer_aliases("http://127.0.0.1/iam");
    assert!(aliases.iter().any(|iss| iss == "http://127.0.0.1/iam"));
    assert!(aliases.iter().any(|iss| iss == "iamrusty"));
}

#[test]
fn org_managed_issuer_has_no_platform_alias() {
    let aliases = hive_domain::platform_issuer_aliases("http://127.0.0.1/iam/orgs/acme");
    assert_eq!(aliases, vec!["http://127.0.0.1/iam/orgs/acme".to_string()]);
    assert!(!aliases.iter().any(|iss| iss == "iamrusty"));

    let path_only = hive_domain::platform_issuer_aliases("/iam/orgs/acme");
    assert!(!path_only.iter().any(|iss| iss == "iamrusty"));
}
