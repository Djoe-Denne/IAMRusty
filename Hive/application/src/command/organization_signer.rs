//! Organization signer configuration commands (ADR-0306).

use async_trait::async_trait;
use hive_domain::port::service::{
    ConfigureOrganizationSignerRequest, IamOrganizationSignerClient, OrganizationSignerResponse,
};
use hive_domain::OrganizationRepository;
use rustycog::command::{Command, CommandError, CommandErrorMapper, CommandHandler};
use rustycog::core::error::DomainError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

use crate::ApplicationError;

/// HTTP/DTO request for configuring an org signer (no secrets in Hive events).
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ConfigureOrganizationSignerHttpRequest {
    #[validate(length(min = 1))]
    pub provider_type: String,
    #[validate(length(min = 1))]
    pub provider_key_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_ref: Option<String>,
    #[validate(length(min = 1))]
    pub public_key: String,
    /// Ignored: Hive overwrites with `Organization.slug` from the database.
    #[serde(default)]
    pub org_slug: String,
}

/// UX-only response persisted metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationSignerHttpResponse {
    pub signing_profile_id: Uuid,
    pub kid: String,
    pub status: String,
    pub issuer: String,
}

impl From<OrganizationSignerResponse> for OrganizationSignerHttpResponse {
    fn from(value: OrganizationSignerResponse) -> Self {
        Self {
            signing_profile_id: value.signing_profile_id,
            kid: value.kid,
            status: value.status,
            issuer: value.issuer,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigureOrganizationSignerCommand {
    pub command_id: Uuid,
    pub organization_id: Uuid,
    pub request: ConfigureOrganizationSignerHttpRequest,
    pub requesting_user_id: Uuid,
}

impl ConfigureOrganizationSignerCommand {
    #[must_use]
    pub fn new(
        organization_id: Uuid,
        request: ConfigureOrganizationSignerHttpRequest,
        requesting_user_id: Uuid,
    ) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            organization_id,
            request,
            requesting_user_id,
        }
    }
}

#[async_trait]
impl Command for ConfigureOrganizationSignerCommand {
    type Result = OrganizationSignerHttpResponse;

    fn command_type(&self) -> &'static str {
        "configure_organization_signer"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.request.provider_type.trim().is_empty() {
            return Err(CommandError::validation(
                "empty_provider_type",
                "provider_type is required",
            ));
        }
        if self.request.public_key.trim().is_empty() {
            return Err(CommandError::validation(
                "empty_public_key",
                "public_key is required",
            ));
        }
        Ok(())
    }
}

pub struct ConfigureOrganizationSignerCommandHandler {
    iam_client: Arc<dyn IamOrganizationSignerClient>,
    organization_repo: Arc<dyn OrganizationRepository>,
}

impl ConfigureOrganizationSignerCommandHandler {
    pub fn new(
        iam_client: Arc<dyn IamOrganizationSignerClient>,
        organization_repo: Arc<dyn OrganizationRepository>,
    ) -> Self {
        Self {
            iam_client,
            organization_repo,
        }
    }
}

#[async_trait]
impl CommandHandler<ConfigureOrganizationSignerCommand>
    for ConfigureOrganizationSignerCommandHandler
{
    async fn handle(
        &self,
        command: ConfigureOrganizationSignerCommand,
    ) -> Result<OrganizationSignerHttpResponse, CommandError> {
        let mut org = self
            .organization_repo
            .find_by_id(&command.organization_id)
            .await
            .map_err(|e| CommandError::infrastructure("org_lookup_failed", e.to_string()))?
            .ok_or_else(|| {
                CommandError::business(
                    "organization_not_found",
                    format!("Organization {} not found", command.organization_id),
                )
            })?;

        let iam_request = ConfigureOrganizationSignerRequest {
            provider_type: command.request.provider_type,
            provider_key_ref: command.request.provider_key_ref,
            credential_ref: command.request.credential_ref,
            public_key: command.request.public_key,
            org_slug: org.slug.clone(),
        };

        let response = self
            .iam_client
            .configure_organization_signer(command.organization_id, &iam_request)
            .await
            .map_err(|e| {
                CommandError::infrastructure("iam_signer_configure_failed", e.to_string())
            })?;

        org.signing_profile_id = Some(response.signing_profile_id);
        org.signing_status = Some(response.status.clone());
        self.organization_repo
            .save(&org)
            .await
            .map_err(|e| CommandError::infrastructure("org_persist_failed", e.to_string()))?;

        Ok(response.into())
    }
}

macro_rules! org_signer_action_command {
    ($name:ident, $type_str:ident, $method:ident) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            pub command_id: Uuid,
            pub organization_id: Uuid,
            pub requesting_user_id: Uuid,
        }

        impl $name {
            #[must_use]
            pub fn new(organization_id: Uuid, requesting_user_id: Uuid) -> Self {
                Self {
                    command_id: Uuid::new_v4(),
                    organization_id,
                    requesting_user_id,
                }
            }
        }

        #[async_trait]
        impl Command for $name {
            type Result = OrganizationSignerHttpResponse;

            fn command_type(&self) -> &'static str {
                stringify!($type_str)
            }

            fn command_id(&self) -> Uuid {
                self.command_id
            }

            fn validate(&self) -> Result<(), CommandError> {
                Ok(())
            }
        }
    };
}

org_signer_action_command!(
    TestOrganizationSignerCommand,
    test_organization_signer,
    test_organization_signer
);
org_signer_action_command!(
    RotateOrganizationSignerCommand,
    rotate_organization_signer,
    rotate_organization_signer
);
org_signer_action_command!(
    DisableOrganizationSignerCommand,
    disable_organization_signer,
    disable_organization_signer
);

pub struct OrganizationSignerActionHandler {
    iam_client: Arc<dyn IamOrganizationSignerClient>,
    organization_repo: Arc<dyn OrganizationRepository>,
}

impl OrganizationSignerActionHandler {
    pub fn new(
        iam_client: Arc<dyn IamOrganizationSignerClient>,
        organization_repo: Arc<dyn OrganizationRepository>,
    ) -> Self {
        Self {
            iam_client,
            organization_repo,
        }
    }

    async fn persist_status(
        &self,
        organization_id: Uuid,
        response: &OrganizationSignerResponse,
    ) -> Result<(), CommandError> {
        let mut org = self
            .organization_repo
            .find_by_id(&organization_id)
            .await
            .map_err(|e| CommandError::infrastructure("org_lookup_failed", e.to_string()))?
            .ok_or_else(|| {
                CommandError::business(
                    "organization_not_found",
                    format!("Organization {organization_id} not found"),
                )
            })?;
        org.signing_profile_id = Some(response.signing_profile_id);
        org.signing_status = Some(response.status.clone());
        self.organization_repo
            .save(&org)
            .await
            .map_err(|e| CommandError::infrastructure("org_persist_failed", e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl CommandHandler<TestOrganizationSignerCommand> for OrganizationSignerActionHandler {
    async fn handle(
        &self,
        command: TestOrganizationSignerCommand,
    ) -> Result<OrganizationSignerHttpResponse, CommandError> {
        let response = self
            .iam_client
            .test_organization_signer(command.organization_id)
            .await
            .map_err(|e| CommandError::infrastructure("iam_signer_test_failed", e.to_string()))?;
        self.persist_status(command.organization_id, &response)
            .await?;
        Ok(response.into())
    }
}

#[async_trait]
impl CommandHandler<RotateOrganizationSignerCommand> for OrganizationSignerActionHandler {
    async fn handle(
        &self,
        command: RotateOrganizationSignerCommand,
    ) -> Result<OrganizationSignerHttpResponse, CommandError> {
        let response = self
            .iam_client
            .rotate_organization_signer(command.organization_id)
            .await
            .map_err(|e| CommandError::infrastructure("iam_signer_rotate_failed", e.to_string()))?;
        self.persist_status(command.organization_id, &response)
            .await?;
        Ok(response.into())
    }
}

#[async_trait]
impl CommandHandler<DisableOrganizationSignerCommand> for OrganizationSignerActionHandler {
    async fn handle(
        &self,
        command: DisableOrganizationSignerCommand,
    ) -> Result<OrganizationSignerHttpResponse, CommandError> {
        let response = self
            .iam_client
            .disable_organization_signer(command.organization_id)
            .await
            .map_err(|e| {
                CommandError::infrastructure("iam_signer_disable_failed", e.to_string())
            })?;
        self.persist_status(command.organization_id, &response)
            .await?;
        Ok(response.into())
    }
}

pub struct OrganizationSignerErrorMapper;

impl CommandErrorMapper for OrganizationSignerErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(domain) = error.downcast_ref::<DomainError>() {
            return CommandError::business("domain_error", domain.to_string());
        }
        if let Some(app) = error.downcast_ref::<ApplicationError>() {
            return CommandError::business("application_error", app.to_string());
        }
        CommandError::infrastructure("unknown_error", error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hive_domain::{
        Organization, OrganizationReadRepository, OrganizationRepository,
        OrganizationWriteRepository,
    };
    use rustycog::command::CommandHandler;
    use std::sync::Mutex;

    struct FakeOrgRepo {
        org: Organization,
    }

    #[async_trait]
    impl OrganizationReadRepository for FakeOrgRepo {
        async fn find_by_id(&self, id: &Uuid) -> Result<Option<Organization>, DomainError> {
            if self.org.id == *id {
                Ok(Some(self.org.clone()))
            } else {
                Ok(None)
            }
        }

        async fn find_by_slug(&self, _: &str) -> Result<Option<Organization>, DomainError> {
            Ok(None)
        }

        async fn find_by_owner(&self, _: &Uuid) -> Result<Vec<Organization>, DomainError> {
            Ok(vec![])
        }

        async fn find_by_user_membership(
            &self,
            _: &Uuid,
            _: u32,
            _: u32,
        ) -> Result<Vec<Organization>, DomainError> {
            Ok(vec![])
        }

        async fn search_by_name(
            &self,
            _: Option<Uuid>,
            _: &str,
            _: u32,
            _: u32,
        ) -> Result<Vec<Organization>, DomainError> {
            Ok(vec![])
        }

        async fn count(&self) -> Result<i64, DomainError> {
            Ok(0)
        }
    }

    #[async_trait]
    impl OrganizationWriteRepository for FakeOrgRepo {
        async fn exists_by_slug(&self, _: &str) -> Result<bool, DomainError> {
            Ok(false)
        }

        async fn save(&self, organization: &Organization) -> Result<Organization, DomainError> {
            Ok(organization.clone())
        }

        async fn delete_by_id(&self, _: &Uuid) -> Result<(), DomainError> {
            Ok(())
        }
    }

    impl OrganizationRepository for FakeOrgRepo {}

    struct CapturingIam {
        captured_slug: Mutex<Option<String>>,
    }

    #[async_trait]
    impl IamOrganizationSignerClient for CapturingIam {
        async fn configure_organization_signer(
            &self,
            _org_id: Uuid,
            request: &ConfigureOrganizationSignerRequest,
        ) -> Result<OrganizationSignerResponse, DomainError> {
            *self.captured_slug.lock().unwrap() = Some(request.org_slug.clone());
            Ok(OrganizationSignerResponse {
                signing_profile_id: Uuid::new_v4(),
                kid: "kid".into(),
                status: "active".into(),
                issuer: format!("http://127.0.0.1/iam/orgs/{}", request.org_slug),
            })
        }

        async fn test_organization_signer(
            &self,
            _org_id: Uuid,
        ) -> Result<OrganizationSignerResponse, DomainError> {
            Err(DomainError::internal_error("unused"))
        }

        async fn rotate_organization_signer(
            &self,
            _org_id: Uuid,
        ) -> Result<OrganizationSignerResponse, DomainError> {
            Err(DomainError::internal_error("unused"))
        }

        async fn disable_organization_signer(
            &self,
            _org_id: Uuid,
        ) -> Result<OrganizationSignerResponse, DomainError> {
            Err(DomainError::internal_error("unused"))
        }
    }

    #[tokio::test]
    async fn configure_overwrites_client_org_slug_from_db() {
        let org_id = Uuid::new_v4();
        let mut org = Organization::new("Acme".into(), "acme-from-db".into(), None, Uuid::new_v4())
            .expect("org");
        org.id = org_id;
        let iam = Arc::new(CapturingIam {
            captured_slug: Mutex::new(None),
        });
        let handler = ConfigureOrganizationSignerCommandHandler::new(
            iam.clone(),
            Arc::new(FakeOrgRepo { org }),
        );
        let command = ConfigureOrganizationSignerCommand::new(
            org_id,
            ConfigureOrganizationSignerHttpRequest {
                provider_type: "pem_file".into(),
                provider_key_ref: "opaque".into(),
                credential_ref: None,
                public_key: "-----BEGIN PUBLIC KEY-----\nMIIB\n-----END PUBLIC KEY-----".into(),
                org_slug: "spoofed-other-org".into(),
            },
            Uuid::new_v4(),
        );
        handler.handle(command).await.expect("configure");
        assert_eq!(
            iam.captured_slug.lock().unwrap().as_deref(),
            Some("acme-from-db")
        );
    }
}
