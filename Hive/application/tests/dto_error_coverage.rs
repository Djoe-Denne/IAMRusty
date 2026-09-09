use hive_application::{
    ApiErrorResponse, ApplicationError, MemberRole, MemberRolePermission, OrganizationListResponse,
    OrganizationResponse, OrganizationSearchRequest, OrganizationStatsResponse, PaginationRequest,
    PaginationResponse, ValidationError,
};
use hive_domain::{Organization, PermissionLevel, RolePermission};
use rustycog::core::error::DomainError;
use std::str::FromStr;
use uuid::Uuid;

fn sample_org() -> Organization {
    Organization::new(
        "Hive Org".to_string(),
        "hive-org".to_string(),
        Some("desc".to_string()),
        Uuid::new_v4(),
    )
    .unwrap()
}

#[test]
fn pagination_defaults_and_cursors() {
    let empty = PaginationResponse::new(1, 0, Some(10));
    assert_eq!(empty.total_pages, Some(0));
    let none = PaginationResponse::new(1, 20, None);
    assert!(!none.has_next);
    let first = PaginationResponse::new(1, 10, Some(25));
    assert!(first.has_next);
    assert!(!first.has_previous);
    assert_eq!(first.total_pages, Some(3));
    let later = PaginationResponse::with_cursors(
        3,
        10,
        Some(25),
        Some("next".to_string()),
        Some("prev".to_string()),
    );
    assert!(!later.has_next);
    assert!(later.has_previous);
    assert_eq!(later.next_cursor.as_deref(), Some("next"));

    let default = PaginationRequest::default();
    assert_eq!(default.page(), 1);
    assert_eq!(default.page_size(), 20);
    let capped = PaginationRequest {
        page: None,
        page_size: Some(250),
        cursor: None,
    };
    assert_eq!(capped.page(), 1);
    assert_eq!(capped.page_size(), 100);
}

#[test]
fn organization_dto_conversions() {
    let org = sample_org();
    let response = OrganizationResponse::from(org.clone());
    assert_eq!(response.slug, "hive-org");
    let detailed = OrganizationResponse::with_details(
        org,
        Some(4),
        Some(2),
        Some(true),
        Some("owner".to_string()),
    );
    assert_eq!(detailed.member_count, Some(4));
    let list =
        OrganizationListResponse::new(vec![detailed], PaginationResponse::new(1, 20, Some(1)));
    assert_eq!(list.organizations.len(), 1);
    let search = OrganizationSearchRequest::default();
    assert_eq!(search.page, Some(1));
    let stats = OrganizationStatsResponse {
        organization_id: Uuid::new_v4(),
        total_members: 1,
        active_members: 1,
        pending_members: 0,
        suspended_members: 0,
        total_roles: 1,
        custom_roles: 0,
        pending_invitations: 0,
        external_links: 0,
        active_sync_jobs: 0,
    };
    assert_eq!(stats.active_members, 1);
}

#[test]
fn api_error_response_maps_all_application_errors() {
    let domain_cases = [
        DomainError::entity_not_found("Org", "1"),
        DomainError::invalid_input("bad"),
        DomainError::business_rule_violation("rule"),
        DomainError::unauthorized("op"),
        DomainError::resource_already_exists("Org", "slug"),
        DomainError::external_service_error("github", "down"),
        DomainError::permission_denied("nope"),
        DomainError::internal_error("boom"),
    ];
    for error in domain_cases {
        let mapped = ApiErrorResponse::from(ApplicationError::Domain(error));
        assert!(!mapped.error_type.is_empty());
    }

    let validation = ApiErrorResponse::from(ApplicationError::validation_error(vec![
        ValidationError::new(
            "name".to_string(),
            "required".to_string(),
            Some("len".to_string()),
        ),
        ValidationError::simple("slug", "required"),
        ValidationError::with_code("slug", "invalid", "format"),
    ]));
    assert_eq!(validation.error_type, "validation_error");
    assert_eq!(validation.validation_errors.as_ref().unwrap().len(), 3);

    let external = ApiErrorResponse::from(ApplicationError::external_service_error(
        "github", "timeout",
    ));
    assert_eq!(external.error_type, "external_service_error");
    let rate = ApiErrorResponse::from(ApplicationError::rate_limit("slow down"));
    assert_eq!(rate.error_type, "rate_limit");
    let internal = ApiErrorResponse::from(ApplicationError::internal_error("oops"));
    assert_eq!(internal.error_type, "internal_error");
    let single = ApplicationError::single_validation_error("email", "invalid");
    assert!(matches!(single, ApplicationError::ValidationError(_)));
}

#[test]
fn member_role_permission_conversions() {
    assert_eq!(<&str>::from(MemberRolePermission::Read), "read");
    assert_eq!(String::from(MemberRolePermission::Write), "write");
    assert_eq!(String::from(MemberRolePermission::Delete), "delete");
    assert_eq!(String::from(MemberRolePermission::Admin), "admin");
    assert_eq!(
        String::from(MemberRolePermission::from_str("READ").unwrap()),
        "read"
    );
    assert_eq!(
        String::from(MemberRolePermission::try_from("write".to_string()).unwrap()),
        "write"
    );
    assert!(MemberRolePermission::from_str("nope").is_err());
    assert_eq!(
        PermissionLevel::try_from(MemberRolePermission::Admin).unwrap(),
        PermissionLevel::Admin
    );
    assert!(PermissionLevel::try_from(MemberRolePermission::Delete).is_err());

    let org_id = Uuid::new_v4();
    let member_role = MemberRole {
        organization_id: org_id,
        resource: "organization".to_string(),
        permissions: MemberRolePermission::Read,
    };
    let role_permission = RolePermission::try_from(&member_role).unwrap();
    let round_trip = MemberRole::from(role_permission);
    assert_eq!(round_trip.organization_id, org_id);
    assert_eq!(String::from(round_trip.permissions), "read");

    let owner_role = RolePermission::new(
        None,
        None,
        org_id,
        &hive_domain::Permission::new(PermissionLevel::Owner, None, None),
        &hive_domain::Resource::from("organization".to_string()),
        None,
    );
    let mapped = MemberRole::from(owner_role);
    assert_eq!(String::from(mapped.permissions), "admin");
}
