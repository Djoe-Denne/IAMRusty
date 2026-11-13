//! QA coverage90-local B: closed vocabularies at project/member input boundaries.
use manifesto_domain::{DataClassification, DomainError, MemberRole, MemberSource, Visibility};
use std::str::FromStr;

#[test]
fn member_source_parsing_accepts_supported_sources_and_rejects_unknown_provenance() {
    for (input, expected) in [
        ("DIRECT", MemberSource::Direct),
        ("Org_Cascade", MemberSource::OrgCascade),
        ("invitation", MemberSource::Invitation),
        ("THIRD_PARTY_SYNC", MemberSource::ThirdPartySync),
    ] {
        assert_eq!(MemberSource::from_str(input).unwrap(), expected);
    }
    for invalid in ["", "inherited", "external"] {
        assert!(
            matches!(
                MemberSource::from_str(invalid),
                Err(DomainError::InvalidInput { .. })
            ),
            "{invalid}"
        );
    }
}

#[test]
fn visibility_parsing_does_not_make_private_or_internal_projects_public() {
    for (input, expected, public) in [
        ("PRIVATE", Visibility::Private, false),
        ("Internal", Visibility::Internal, false),
        ("PUBLIC", Visibility::Public, true),
    ] {
        let visibility = Visibility::from_str(input).unwrap();
        assert_eq!(visibility, expected);
        assert_eq!(visibility.is_public(), public, "{input}");
    }
    for invalid in ["", "everyone", "protected"] {
        assert!(
            matches!(
                Visibility::from_str(invalid),
                Err(DomainError::InvalidInput { .. })
            ),
            "{invalid}"
        );
    }
}

#[test]
fn data_classification_parsing_rejects_unknown_instead_of_defaulting_to_public() {
    for (input, expected) in [
        ("PUBLIC", DataClassification::Public),
        ("Internal", DataClassification::Internal),
        ("CONFIDENTIAL", DataClassification::Confidential),
        ("Restricted", DataClassification::Restricted),
    ] {
        assert_eq!(DataClassification::from_str(input).unwrap(), expected);
    }
    for invalid in ["", "secret", "unclassified"] {
        assert!(
            matches!(
                DataClassification::from_str(invalid),
                Err(DomainError::InvalidInput { .. })
            ),
            "{invalid}"
        );
    }
}

#[test]
fn member_role_parser_accepts_documented_ranks_and_rejects_unsupported_actions() {
    for (input, expected) in [
        ("READ", MemberRole::Read),
        ("Write", MemberRole::Write),
        ("ADMIN", MemberRole::Admin),
        ("Owner", MemberRole::Owner),
    ] {
        assert_eq!(MemberRole::from_str(input).unwrap(), expected);
    }
    for invalid in ["", "delete", "superuser"] {
        assert!(
            matches!(
                MemberRole::from_str(invalid),
                Err(DomainError::InvalidInput { .. })
            ),
            "{invalid}"
        );
    }
}

#[test]
fn equal_rank_can_use_permissions_but_cannot_manage_peer() {
    for role in [
        MemberRole::Read,
        MemberRole::Write,
        MemberRole::Admin,
        MemberRole::Owner,
    ] {
        assert!(role.has_permission_of(&role));
        assert!(
            !role.can_manage(&role),
            "peer management must require strict superiority: {role:?}"
        );
    }
}
