//! QA coverage90-local B: no use-case doubles; command validation only.
use hive_application::command::invitation::{AcceptInvitationCommand, GetInvitationByTokenCommand};
use hive_application::command::organization_signer::{
    ConfigureOrganizationSignerCommand, ConfigureOrganizationSignerHttpRequest,
};
use rustycog::command::{Command, CommandError};
use uuid::Uuid;

fn assert_validation(result: Result<(), CommandError>, code: &str) {
    let error = result.expect_err("invalid command must be rejected");
    assert!(
        matches!(error, CommandError::Validation { .. }),
        "{error:?}"
    );
    assert_eq!(error.code(), code);
}

#[test]
fn invitation_accept_and_lookup_reject_missing_tokens() {
    for token in ["", " ", "\t\n"] {
        let accept = AcceptInvitationCommand::new(token.to_owned(), Uuid::new_v4(), "iamrusty");
        assert_validation(accept.validate(), "empty_token");
        let lookup = GetInvitationByTokenCommand::new(token.to_owned());
        assert_validation(lookup.validate(), "empty_token");
    }
}

#[test]
fn invitation_accept_and_lookup_accept_nonempty_opaque_tokens() {
    let token = "hive-invitation-opaque-token";
    assert!(
        AcceptInvitationCommand::new(token.to_owned(), Uuid::new_v4(), "iamrusty")
            .validate()
            .is_ok()
    );
    assert!(GetInvitationByTokenCommand::new(token.to_owned())
        .validate()
        .is_ok());
}

fn signer_request() -> ConfigureOrganizationSignerHttpRequest {
    ConfigureOrganizationSignerHttpRequest {
        provider_type: "vault_transit".to_owned(),
        provider_key_ref: "transit/keys/hive".to_owned(),
        provider_key_version: Some(1),
        credential_ref: None,
        public_key: "test-public-key-placeholder".to_owned(),
        org_slug: String::new(),
    }
}

#[test]
fn signer_configuration_rejects_blank_provider_type_or_public_key() {
    for blank in ["", " ", "\t\n"] {
        let mut request = signer_request();
        request.provider_type = blank.to_owned();
        assert_validation(
            ConfigureOrganizationSignerCommand::new(Uuid::new_v4(), request, Uuid::new_v4())
                .validate(),
            "empty_provider_type",
        );
        let mut request = signer_request();
        request.public_key = blank.to_owned();
        assert_validation(
            ConfigureOrganizationSignerCommand::new(Uuid::new_v4(), request, Uuid::new_v4())
                .validate(),
            "empty_public_key",
        );
    }
}

#[test]
fn signer_configuration_does_not_require_caller_supplied_org_slug() {
    // Slug is supplied by the repository later, not trusted from the caller.
    let request = signer_request();
    assert!(
        ConfigureOrganizationSignerCommand::new(Uuid::new_v4(), request, Uuid::new_v4())
            .validate()
            .is_ok()
    );
}
