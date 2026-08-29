use async_trait::async_trait;
use chrono::Utc;
use hive_application::{ApplicationError, HiveOutboxUnitOfWork};
use hive_domain::{
    Organization, OrganizationInvitation, OrganizationMember, OrganizationMemberRolePermission,
    RolePermission,
};
use rustycog::core::error::DomainError;
use rustycog::db::DbConnectionPool;
use rustycog::events::DomainEvent;
use rustycog::outbox::OutboxRecorder;
use sea_orm::DatabaseTransaction;
use uuid::Uuid;

use crate::repository::{
    ExternalLinkWriteRepositoryImpl, MemberRoleWriteRepositoryImpl,
    OrganizationInvitationWriteRepositoryImpl, OrganizationMemberReadRepositoryImpl,
    OrganizationMemberWriteRepositoryImpl, OrganizationReadRepositoryImpl,
    OrganizationWriteRepositoryImpl, PermissionReadRepositoryImpl, ResourceReadRepositoryImpl,
    RolePermissionReadRepositoryImpl, RolePermissionWriteRepositoryImpl,
    SyncJobWriteRepositoryImpl,
};

#[derive(Clone)]
pub struct HiveOutboxUnitOfWorkImpl {
    db: DbConnectionPool,
    outbox: OutboxRecorder,
}

impl HiveOutboxUnitOfWorkImpl {
    pub const fn new(db: DbConnectionPool, outbox: OutboxRecorder) -> Self {
        Self { db, outbox }
    }

    async fn begin(&self) -> Result<DatabaseTransaction, ApplicationError> {
        self.db.begin_write_transaction().await.map_err(|e| {
            ApplicationError::internal_error(&format!("failed to begin write transaction: {e}"))
        })
    }

    async fn record(
        &self,
        txn: &DatabaseTransaction,
        event: &dyn DomainEvent,
    ) -> Result<(), ApplicationError> {
        self.outbox.record(txn, event).await.map_err(|e| {
            ApplicationError::internal_error(&format!("failed to record Hive outbox event: {e}"))
        })
    }

    async fn finish<T>(
        txn: DatabaseTransaction,
        result: Result<T, ApplicationError>,
    ) -> Result<T, ApplicationError> {
        match result {
            Ok(value) => {
                txn.commit().await.map_err(|e| {
                    ApplicationError::internal_error(&format!(
                        "failed to commit write transaction: {e}"
                    ))
                })?;
                Ok(value)
            }
            Err(error) => {
                if let Err(rollback_error) = txn.rollback().await {
                    tracing::error!(
                        "failed to rollback Hive write transaction: {}",
                        rollback_error
                    );
                }
                Err(error)
            }
        }
    }

    async fn persist_new_organization(
        txn: &DatabaseTransaction,
        organization: &Organization,
    ) -> Result<Organization, ApplicationError> {
        if OrganizationWriteRepositoryImpl::exists_by_slug_with_connection(txn, &organization.slug)
            .await?
        {
            return Err(ApplicationError::Domain(DomainError::resource_already_exists(
                "Organization",
                &format!("slug={}", organization.slug),
            )));
        }

        let saved_org =
            OrganizationWriteRepositoryImpl::save_with_connection(txn, organization).await?;

        let permissions = PermissionReadRepositoryImpl::find_all_with_connection(txn).await?;
        let resources = ResourceReadRepositoryImpl::find_all_with_connection(txn).await?;
        let mut roles = Vec::new();
        for permission in &permissions {
            for resource in &resources {
                let name = format!("{}:{}", resource.name, permission.level.to_str());
                let role = RolePermission::new(
                    None,
                    Some(name),
                    saved_org.id,
                    permission,
                    resource,
                    Some(Utc::now()),
                );
                roles.push(
                    RolePermissionWriteRepositoryImpl::save_with_connection(
                        txn,
                        &saved_org.id,
                        &role,
                    )
                    .await?,
                );
            }
        }

        let owner_role = roles
            .iter()
            .find(|role| {
                role.resource.name == "organization" && role.permission.level.to_str() == "owner"
            })
            .cloned()
            .ok_or_else(|| {
                ApplicationError::Domain(DomainError::entity_not_found(
                    "RolePermission",
                    "resource_type=organization, permission=owner",
                ))
            })?;

        let member = OrganizationMember::new(saved_org.id, organization.owner_user_id, None);
        let saved_member =
            OrganizationMemberWriteRepositoryImpl::save_with_connection(txn, &member).await?;
        let member_id = saved_member.id.ok_or_else(|| {
            ApplicationError::internal_error("saved owner member is missing an id")
        })?;
        let member_role = OrganizationMemberRolePermission::new(
            None,
            &saved_org.id,
            &member_id,
            &owner_role,
            Utc::now(),
        );
        MemberRoleWriteRepositoryImpl::save_with_connection(txn, &member_role).await?;

        Ok(saved_org)
    }

    async fn persist_new_member(
        txn: &DatabaseTransaction,
        organization_id: Uuid,
        user_id: Uuid,
        roles: Vec<RolePermission>,
        added_by_user_id: Option<Uuid>,
    ) -> Result<OrganizationMember, ApplicationError> {
        OrganizationReadRepositoryImpl::find_by_id_with_connection(txn, &organization_id)
            .await?
            .ok_or_else(|| {
                ApplicationError::Domain(DomainError::entity_not_found(
                    "Organization",
                    &organization_id.to_string(),
                ))
            })?;

        if OrganizationMemberReadRepositoryImpl::find_by_organization_and_user_with_connection(
            txn,
            &organization_id,
            &user_id,
        )
        .await?
        .is_some()
        {
            return Err(ApplicationError::Domain(DomainError::resource_already_exists(
                "OrganizationMember",
                &format!("user_id={user_id}, organization_id={organization_id}"),
            )));
        }

        let member = OrganizationMember::new(organization_id, user_id, added_by_user_id);
        let mut saved_member =
            OrganizationMemberWriteRepositoryImpl::save_with_connection(txn, &member).await?;
        let resolved_roles =
            RolePermissionReadRepositoryImpl::find_by_organization_roles_with_connection(
                txn,
                &organization_id,
                &roles,
            )
            .await?;
        let member_id = saved_member.id.ok_or_else(|| {
            ApplicationError::Domain(DomainError::invalid_input("Member ID is required"))
        })?;

        let mut assigned_roles = Vec::new();
        for role in resolved_roles {
            let new_role = OrganizationMemberRolePermission::new(
                None,
                &organization_id,
                &member_id,
                &role,
                Utc::now(),
            );
            assigned_roles.push(
                MemberRoleWriteRepositoryImpl::save_with_connection(txn, &new_role)
                    .await
                    .map_err(|_| {
                        ApplicationError::Domain(DomainError::BusinessRuleViolation {
                            rule: "Trying to add roles to a unexisting member".to_string(),
                        })
                    })?,
            );
        }
        saved_member.update_roles(assigned_roles);
        let saved_member =
            OrganizationMemberWriteRepositoryImpl::save_with_connection(txn, &saved_member).await?;
        Ok(saved_member)
    }
}

#[async_trait]
impl HiveOutboxUnitOfWork for HiveOutboxUnitOfWorkImpl {
    async fn create_organization(
        &self,
        organization: Organization,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<Organization, ApplicationError> {
        let txn = self.begin().await?;
        let result = async {
            let saved = Self::persist_new_organization(&txn, &organization).await?;
            self.record(&txn, event.as_ref()).await?;
            Ok(saved)
        }
        .await;
        Self::finish(txn, result).await
    }

    async fn update_organization(
        &self,
        organization: Organization,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<Organization, ApplicationError> {
        let txn = self.begin().await?;
        let result = async {
            let saved =
                OrganizationWriteRepositoryImpl::save_with_connection(&txn, &organization).await?;
            self.record(&txn, event.as_ref()).await?;
            Ok(saved)
        }
        .await;
        Self::finish(txn, result).await
    }

    async fn delete_organization(
        &self,
        organization_id: Uuid,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<(), ApplicationError> {
        let txn = self.begin().await?;
        let result = async {
            MemberRoleWriteRepositoryImpl::delete_by_organization_with_connection(
                &txn,
                &organization_id,
            )
            .await?;
            OrganizationMemberWriteRepositoryImpl::delete_by_organization_with_connection(
                &txn,
                &organization_id,
            )
            .await?;
            OrganizationWriteRepositoryImpl::delete_by_id_with_connection(&txn, &organization_id)
                .await?;
            self.record(&txn, event.as_ref()).await?;
            Ok(())
        }
        .await;
        Self::finish(txn, result).await
    }

    async fn add_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        roles: Vec<RolePermission>,
        added_by_user_id: Option<Uuid>,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<OrganizationMember, ApplicationError> {
        let txn = self.begin().await?;
        let result = async {
            let saved = Self::persist_new_member(
                &txn,
                organization_id,
                user_id,
                roles,
                added_by_user_id,
            )
            .await?;
            self.record(&txn, event.as_ref()).await?;
            Ok(saved)
        }
        .await;
        Self::finish(txn, result).await
    }

    async fn remove_member(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<(), ApplicationError> {
        let txn = self.begin().await?;
        let result = async {
            OrganizationReadRepositoryImpl::find_by_id_with_connection(&txn, &organization_id)
                .await?
                .ok_or_else(|| {
                    ApplicationError::Domain(DomainError::entity_not_found(
                        "Organization",
                        &organization_id.to_string(),
                    ))
                })?;
            let member =
                OrganizationMemberReadRepositoryImpl::find_by_organization_and_user_with_connection(
                    &txn,
                    &organization_id,
                    &user_id,
                )
                .await?
                .ok_or_else(|| {
                    ApplicationError::Domain(DomainError::entity_not_found(
                        "OrganizationMember",
                        &format!("user_id={user_id}, organization_id={organization_id}"),
                    ))
                })?;
            let member_id = member.id.ok_or_else(|| {
                ApplicationError::Domain(DomainError::invalid_input("Member ID is required"))
            })?;
            OrganizationMemberWriteRepositoryImpl::delete_by_id_with_connection(&txn, &member_id)
                .await?;
            self.record(&txn, event.as_ref()).await?;
            Ok(())
        }
        .await;
        Self::finish(txn, result).await
    }

    async fn save_invitation(
        &self,
        invitation: OrganizationInvitation,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<OrganizationInvitation, ApplicationError> {
        let txn = self.begin().await?;
        let result = async {
            let saved =
                OrganizationInvitationWriteRepositoryImpl::save_with_connection(&txn, &invitation)
                    .await?;
            self.record(&txn, event.as_ref()).await?;
            Ok(saved)
        }
        .await;
        Self::finish(txn, result).await
    }

    async fn save_external_link(
        &self,
        link: hive_domain::ExternalLink,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<hive_domain::ExternalLink, ApplicationError> {
        let txn = self.begin().await?;
        let result = async {
            let saved = ExternalLinkWriteRepositoryImpl::save_with_connection(&txn, &link).await?;
            self.record(&txn, event.as_ref()).await?;
            Ok(saved)
        }
        .await;
        Self::finish(txn, result).await
    }

    async fn save_sync_job(
        &self,
        job: hive_domain::SyncJob,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<hive_domain::SyncJob, ApplicationError> {
        let txn = self.begin().await?;
        let result = async {
            let saved = SyncJobWriteRepositoryImpl::save_with_connection(&txn, &job).await?;
            self.record(&txn, event.as_ref()).await?;
            Ok(saved)
        }
        .await;
        Self::finish(txn, result).await
    }

    async fn record_event(
        &self,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<(), ApplicationError> {
        let txn = self.begin().await?;
        let result = self.record(&txn, event.as_ref()).await;
        Self::finish(txn, result).await
    }
}
