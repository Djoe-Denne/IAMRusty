//! InProcess Hive → IAM organization-signer adapter (ADR-0306).
//!
//! Lives only in `oodhive-monolith`. Maps Hive port DTOs ↔ IAM façade DTOs
//! and `iam_domain::DomainError` → `rustycog::core::error::DomainError`.
//! No HTTP, no JSON, no internal token on this path.

use std::sync::Arc;

use async_trait::async_trait;
use hive_domain::port::service::{
    ConfigureOrganizationSignerRequest, IamOrganizationSignerClient, OrganizationSignerResponse,
};
use iam_application::usecase::{
    ConfigureOrganizationSignerInput, OrganizationSignerFacade, OrganizationSignerResult,
};
use iam_domain::error::DomainError as IamDomainError;
use rustycog::core::error::DomainError;
use uuid::Uuid;

/// Capability injected into Hive via `AppBuilder::with_iam_organization_signer_client`.
pub struct InProcessIamOrganizationSignerClient {
    facade: Arc<dyn OrganizationSignerFacade>,
}

impl InProcessIamOrganizationSignerClient {
    #[must_use]
    pub fn new(facade: Arc<dyn OrganizationSignerFacade>) -> Self {
        Self { facade }
    }
}

#[async_trait]
impl IamOrganizationSignerClient for InProcessIamOrganizationSignerClient {
    async fn configure_organization_signer(
        &self,
        org_id: Uuid,
        request: &ConfigureOrganizationSignerRequest,
    ) -> Result<OrganizationSignerResponse, DomainError> {
        let input = ConfigureOrganizationSignerInput {
            provider_type: request.provider_type.clone(),
            provider_key_ref: request.provider_key_ref.clone(),
            credential_ref: request.credential_ref.clone(),
            public_key: request.public_key.clone(),
            org_slug: request.org_slug.clone(),
        };
        let result = self
            .facade
            .configure(org_id, &input)
            .await
            .map_err(map_iam_error)?;
        Ok(map_result(result))
    }

    async fn test_organization_signer(
        &self,
        org_id: Uuid,
    ) -> Result<OrganizationSignerResponse, DomainError> {
        let result = self.facade.test(org_id).await.map_err(map_iam_error)?;
        Ok(map_result(result))
    }

    async fn rotate_organization_signer(
        &self,
        org_id: Uuid,
    ) -> Result<OrganizationSignerResponse, DomainError> {
        let result = self.facade.rotate(org_id).await.map_err(map_iam_error)?;
        Ok(map_result(result))
    }

    async fn disable_organization_signer(
        &self,
        org_id: Uuid,
    ) -> Result<OrganizationSignerResponse, DomainError> {
        let result = self.facade.disable(org_id).await.map_err(map_iam_error)?;
        Ok(map_result(result))
    }
}

fn map_result(result: OrganizationSignerResult) -> OrganizationSignerResponse {
    OrganizationSignerResponse {
        signing_profile_id: result.signing_profile_id,
        kid: result.kid,
        status: result.status,
        issuer: result.issuer,
    }
}

fn map_iam_error(err: IamDomainError) -> DomainError {
    match err {
        IamDomainError::SigningKeyAdmissionDenied { reason, .. } => {
            use iam_domain::entity::signing_key::SigningKeyAdmissionReason;
            let message = match reason {
                SigningKeyAdmissionReason::EpochConflict => "signing_epoch_conflict",
                SigningKeyAdmissionReason::Capacity
                | SigningKeyAdmissionReason::TenantEpochLimit
                | SigningKeyAdmissionReason::ChurnRate => "signing_admission_throttled",
            };
            DomainError::external_service_error("iam_service", message)
        }
        IamDomainError::UserNotFound | IamDomainError::TokenNotFound => {
            DomainError::entity_not_found("organization_signer", "missing")
        }
        IamDomainError::AuthorizationError(message)
            if message.contains("no active organization signing key") =>
        {
            DomainError::entity_not_found("organization_signer", "active")
        }
        IamDomainError::ProviderNotSupported(provider) => {
            DomainError::invalid_input(&format!("provider not supported: {provider}"))
        }
        IamDomainError::BusinessRuleViolation(rule) => DomainError::business_rule_violation(&rule),
        IamDomainError::AuthorizationError(message)
        | IamDomainError::TokenValidationFailed(message) => DomainError::invalid_input(&message),
        IamDomainError::InvalidToken
        | IamDomainError::TokenExpired
        | IamDomainError::InvalidUsername => DomainError::invalid_input("invalid input"),
        IamDomainError::ExternalServiceError { service, message } => {
            DomainError::external_service_error(&service, &message)
        }
        IamDomainError::RepositoryError(message) => DomainError::internal_error(&message),
        other => DomainError::internal_error(&other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use iam_application::usecase::organization_signer::NO_ACTIVE_ORGANIZATION_SIGNING_KEY;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeFacade {
        configure_calls: Mutex<u32>,
        rotate_calls: Mutex<u32>,
        last_configure: Mutex<Option<(Uuid, ConfigureOrganizationSignerInput)>>,
    }

    #[async_trait]
    impl OrganizationSignerFacade for FakeFacade {
        async fn configure(
            &self,
            organization_id: Uuid,
            request: &ConfigureOrganizationSignerInput,
        ) -> Result<OrganizationSignerResult, IamDomainError> {
            *self.configure_calls.lock().unwrap() += 1;
            *self.last_configure.lock().unwrap() = Some((organization_id, request.clone()));
            Ok(OrganizationSignerResult {
                signing_profile_id: Uuid::nil(),
                kid: "kid-configure".into(),
                status: "active".into(),
                issuer: format!("http://iam/orgs/{}", request.org_slug),
            })
        }

        async fn test(
            &self,
            _organization_id: Uuid,
        ) -> Result<OrganizationSignerResult, IamDomainError> {
            Err(IamDomainError::AuthorizationError(
                NO_ACTIVE_ORGANIZATION_SIGNING_KEY.into(),
            ))
        }

        async fn rotate(
            &self,
            organization_id: Uuid,
        ) -> Result<OrganizationSignerResult, IamDomainError> {
            *self.rotate_calls.lock().unwrap() += 1;
            Ok(OrganizationSignerResult {
                signing_profile_id: organization_id,
                kid: "kid-rotate".into(),
                status: "active".into(),
                issuer: "http://iam/orgs/acme".into(),
            })
        }

        async fn disable(
            &self,
            _organization_id: Uuid,
        ) -> Result<OrganizationSignerResult, IamDomainError> {
            unreachable!("not covered by unit tests")
        }
    }

    #[tokio::test]
    async fn configure_and_rotate_call_facade_without_http() {
        let facade = Arc::new(FakeFacade::default());
        let client = InProcessIamOrganizationSignerClient::new(facade.clone());
        let org_id = Uuid::new_v4();

        let configured = client
            .configure_organization_signer(
                org_id,
                &ConfigureOrganizationSignerRequest {
                    provider_type: "pem_file".into(),
                    provider_key_ref: format!("{org_id}/a.pem"),
                    credential_ref: None,
                    public_key: "pem".into(),
                    org_slug: "acme".into(),
                },
            )
            .await
            .expect("configure");
        assert_eq!(configured.kid, "kid-configure");
        assert_eq!(*facade.configure_calls.lock().unwrap(), 1);
        let (seen_org, seen_input) = facade
            .last_configure
            .lock()
            .unwrap()
            .clone()
            .expect("captured");
        assert_eq!(seen_org, org_id);
        assert_eq!(seen_input.org_slug, "acme");

        let rotated = client
            .rotate_organization_signer(org_id)
            .await
            .expect("rotate");
        assert_eq!(rotated.kid, "kid-rotate");
        assert_eq!(*facade.rotate_calls.lock().unwrap(), 1);
    }

    #[test]
    fn admission_denials_match_http_client_markers_without_internal_details() {
        use iam_domain::entity::signing_key::SigningKeyAdmissionReason as Reason;
        for (reason, expected) in [
            (Reason::Capacity, "signing_admission_throttled"),
            (Reason::TenantEpochLimit, "signing_admission_throttled"),
            (Reason::ChurnRate, "signing_admission_throttled"),
            (Reason::EpochConflict, "signing_epoch_conflict"),
        ] {
            for retry_after_seconds in [None, Some(3500)] {
                let error = map_iam_error(IamDomainError::SigningKeyAdmissionDenied {
                    reason,
                    retry_after_seconds,
                });
                assert!(
                    matches!(error, DomainError::ExternalServiceError { service, message }
                    if service == "iam_service" && message == expected)
                );
            }
        }
    }

    #[test]
    fn map_iam_not_found_to_entity_not_found() {
        let err = map_iam_error(IamDomainError::AuthorizationError(
            NO_ACTIVE_ORGANIZATION_SIGNING_KEY.into(),
        ));
        assert!(matches!(err, DomainError::EntityNotFound { .. }));
    }
}
