use hive_infra::HiveErrorMapper;
use rustycog::core::error::{DomainError, ServiceError};
use rustycog::events::adapter::ErrorMapper;

#[test]
fn hive_error_mapper_covers_domain_and_service_variants() {
    let mapper = HiveErrorMapper;
    let domain_errors = [
        DomainError::entity_not_found("Org", "1"),
        DomainError::invalid_input("bad"),
        DomainError::business_rule_violation("rule"),
        DomainError::unauthorized("op"),
        DomainError::resource_already_exists("Org", "slug"),
        DomainError::external_service_error("github", "down"),
        DomainError::permission_denied("no"),
        DomainError::internal_error("boom"),
    ];
    for error in domain_errors {
        let _ = mapper.to_service_error(error);
    }

    let service_errors = [
        ServiceError::authentication("x"),
        ServiceError::authorization("x"),
        ServiceError::not_found("missing"),
        ServiceError::infrastructure("down"),
        ServiceError::validation("bad"),
        ServiceError::business("rule"),
        ServiceError::Timeout {
            message: "late".into(),
            operation: None,
        },
        ServiceError::ServiceUnavailable {
            message: "off".into(),
            retry_after: None,
        },
        ServiceError::internal("boom"),
        ServiceError::conflict("dup"),
        ServiceError::RateLimit {
            message: "slow".into(),
            retry_after: None,
        },
    ];
    for error in service_errors {
        let _ = mapper.from_service_error(error);
    }
}
