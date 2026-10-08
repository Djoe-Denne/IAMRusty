//! Initial Hive schema, flattened on 2026-10-04. Incremental ALTERs are folded into CREATEs.
//! This migration initializes an empty database; it is not an upgrade of a historical ledger.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        Box::pin(organizations::create(manager)).await?;
        Box::pin(organization_members::create(manager)).await?;
        Box::pin(organization_invitations::create(manager)).await?;
        Box::pin(external_providers::create(manager)).await?;
        Box::pin(external_links::create(manager)).await?;
        Box::pin(sync_jobs::create(manager)).await?;
        Box::pin(permissions::create(manager)).await?;
        Box::pin(resources::create(manager)).await?;
        Box::pin(role_permissions::create(manager)).await?;
        Box::pin(organization_member_role_permissions::create(manager)).await?;
        Box::pin(outbox::create(manager)).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        outbox::drop(manager).await?;
        organization_member_role_permissions::drop(manager).await?;
        role_permissions::drop(manager).await?;
        resources::drop(manager).await?;
        permissions::drop(manager).await?;
        sync_jobs::drop(manager).await?;
        external_links::drop(manager).await?;
        external_providers::drop(manager).await?;
        organization_invitations::drop(manager).await?;
        organization_members::drop(manager).await?;
        organizations::drop(manager).await?;
        Ok(())
    }
}

mod organizations {
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Organizations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Organizations::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(Organizations::Name)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Organizations::Slug)
                            .string_len(100)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Organizations::Description).text().null())
                    .col(
                        ColumnDef::new(Organizations::AvatarUrl)
                            .string_len(500)
                            .null(),
                    )
                    .col(ColumnDef::new(Organizations::OwnerUserId).uuid().not_null())
                    .col(
                        ColumnDef::new(Organizations::Settings)
                            .json_binary()
                            .not_null()
                            .default(Expr::cust("'{}'::jsonb")),
                    )
                    .col(
                        ColumnDef::new(Organizations::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Organizations::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Organizations::SigningProfileId)
                            .uuid()
                            .null(),
                    )
                    .col(ColumnDef::new(Organizations::SigningStatus).string().null())
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organizations_owner_user_id")
                    .table(Organizations::Table)
                    .col(Organizations::OwnerUserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organizations_slug")
                    .table(Organizations::Table)
                    .col(Organizations::Slug)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organizations_created_at")
                    .table(Organizations::Table)
                    .col(Organizations::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    pub(super) async fn drop(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Organizations::Table).to_owned())
            .await
    }

    #[derive(DeriveIden)]
    enum Organizations {
        Table,
        Id,
        Name,
        Slug,
        Description,
        AvatarUrl,
        OwnerUserId,
        Settings,
        CreatedAt,
        UpdatedAt,
        SigningProfileId,
        SigningStatus,
    }
}

mod organization_members {
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        create_organization_members_table(manager).await?;
        create_organization_members_indexes(manager).await?;
        Ok(())
    }

    pub(super) async fn drop(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(OrganizationMembers::Table).to_owned())
            .await
    }

    async fn create_organization_members_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OrganizationMembers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OrganizationMembers::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(OrganizationMembers::OrganizationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationMembers::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationMembers::Status)
                            .string_len(20)
                            .not_null()
                            .default("active"),
                    )
                    .col(
                        ColumnDef::new(OrganizationMembers::InvitedByUserId)
                            .uuid()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationMembers::InvitedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationMembers::JoinedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationMembers::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(OrganizationMembers::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(OrganizationMembers::Issuer)
                            .string()
                            .not_null()
                            .default("iamrusty"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_organization_members_organization_id")
                            .from(
                                OrganizationMembers::Table,
                                OrganizationMembers::OrganizationId,
                            )
                            .to(Organizations::Table, Organizations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn create_organization_members_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organization_members_user_id")
                    .table(OrganizationMembers::Table)
                    .col(OrganizationMembers::UserId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organization_members_organization_id")
                    .table(OrganizationMembers::Table)
                    .col(OrganizationMembers::OrganizationId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organization_members_status")
                    .table(OrganizationMembers::Table)
                    .col(OrganizationMembers::Status)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organization_members_org_issuer_user")
                    .table(OrganizationMembers::Table)
                    .col(OrganizationMembers::OrganizationId)
                    .col(OrganizationMembers::Issuer)
                    .col(OrganizationMembers::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    #[derive(DeriveIden)]
    enum Organizations {
        Table,
        Id,
    }

    #[derive(DeriveIden)]
    enum OrganizationRoles {
        Table,
        Id,
    }

    #[derive(DeriveIden)]
    enum OrganizationMembers {
        Table,
        Id,
        OrganizationId,
        UserId,
        Status,
        InvitedByUserId,
        InvitedAt,
        JoinedAt,
        CreatedAt,
        UpdatedAt,
        Issuer,
    }
}

mod organization_invitations {
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        create_organization_invitations_table(manager).await?;
        create_organization_invitations_indexes(manager).await?;
        Ok(())
    }

    pub(super) async fn drop(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(OrganizationInvitations::Table)
                    .to_owned(),
            )
            .await
    }

    async fn create_organization_invitations_table(
        manager: &SchemaManager<'_>,
    ) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OrganizationInvitations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OrganizationInvitations::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::OrganizationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::AggregateId)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::InvitedByUserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::RolePermissions)
                            .json()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::Token)
                            .string_len(100)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::Status)
                            .string_len(20)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::AcceptedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::Message)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationInvitations::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_organization_invitations_organization_id")
                            .from(
                                OrganizationInvitations::Table,
                                OrganizationInvitations::OrganizationId,
                            )
                            .to(Organizations::Table, Organizations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn create_organization_invitations_indexes(
        manager: &SchemaManager<'_>,
    ) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organization_invitations_token")
                    .table(OrganizationInvitations::Table)
                    .col(OrganizationInvitations::Token)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organization_invitations_aggregate_id_status")
                    .table(OrganizationInvitations::Table)
                    .col(OrganizationInvitations::AggregateId)
                    .col(OrganizationInvitations::Status)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organization_invitations_organization_id")
                    .table(OrganizationInvitations::Table)
                    .col(OrganizationInvitations::OrganizationId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organization_invitations_expires_at")
                    .table(OrganizationInvitations::Table)
                    .col(OrganizationInvitations::ExpiresAt)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organization_invitations_org_aggregate_id_status")
                    .table(OrganizationInvitations::Table)
                    .col(OrganizationInvitations::OrganizationId)
                    .col(OrganizationInvitations::AggregateId)
                    .col(OrganizationInvitations::Status)
                    .unique()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    #[derive(DeriveIden)]
    enum Organizations {
        Table,
        Id,
    }

    #[derive(DeriveIden)]
    enum OrganizationRoles {
        Table,
        Id,
    }

    #[derive(DeriveIden)]
    enum OrganizationInvitations {
        Table,
        Id,
        OrganizationId,
        AggregateId,
        InvitedByUserId,
        RolePermissions,
        Token,
        Status,
        ExpiresAt,
        AcceptedAt,
        Message,
        CreatedAt,
    }
}

mod external_providers {
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        create_external_providers_table(manager).await?;
        create_external_providers_indexes(manager).await?;
        seed_external_providers(manager).await?;
        Ok(())
    }

    pub(super) async fn drop(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ExternalProviders::Table).to_owned())
            .await
    }

    async fn create_external_providers_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ExternalProviders::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ExternalProviders::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(ExternalProviders::ProviderType)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ExternalProviders::Name)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ExternalProviders::ConfigSchema)
                            .json_binary()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ExternalProviders::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(ExternalProviders::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn create_external_providers_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_external_providers_provider_type")
                    .table(ExternalProviders::Table)
                    .col(ExternalProviders::ProviderType)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_external_providers_is_active")
                    .table(ExternalProviders::Table)
                    .col(ExternalProviders::IsActive)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    // DDL SeaORM
    #[allow(clippy::expect_used)]
    fn ddl(raw: &str) -> serde_json::Value {
        serde_json::from_str(raw).expect("DDL")
    }

    async fn seed_external_providers(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        let github_json = ddl(r#"{
                "type": "object",
                "properties": {
                    "org_name": { "type": "string", "description": "GitHub organization name" },
                    "access_token": { "type": "string", "description": "GitHub access token" },
                    "base_url": {
                        "type": "string",
                        "description": "GitHub API base URL (for GitHub Enterprise)",
                        "default": "https://api.github.com"
                    }
                },
                "required": ["org_name", "access_token"]
            }"#);
        let gitlab_json = ddl(r#"{
                "type": "object",
                "properties": {
                    "group_id": { "type": "string", "description": "GitLab group ID" },
                    "access_token": { "type": "string", "description": "GitLab access token" },
                    "base_url": {
                        "type": "string",
                        "description": "GitLab instance URL",
                        "default": "https://gitlab.com"
                    }
                },
                "required": ["group_id", "access_token"]
            }"#);
        let confluence_json = ddl(r#"{
                "type": "object",
                "properties": {
                    "space_key": { "type": "string", "description": "Confluence space key" },
                    "api_token": { "type": "string", "description": "Confluence API token" },
                    "username": { "type": "string", "description": "Confluence username" },
                    "base_url": { "type": "string", "description": "Confluence instance URL" }
                },
                "required": ["space_key", "api_token", "username", "base_url"]
            }"#);

        manager
            .exec_stmt(
                Query::insert()
                    .into_table(ExternalProviders::Table)
                    .columns([
                        ExternalProviders::ProviderType,
                        ExternalProviders::Name,
                        ExternalProviders::ConfigSchema,
                    ])
                    .values_panic(["github".into(), "GitHub".into(), github_json.into()])
                    .values_panic(["gitlab".into(), "GitLab".into(), gitlab_json.into()])
                    .values_panic([
                        "confluence".into(),
                        "Confluence".into(),
                        confluence_json.into(),
                    ])
                    .to_owned(),
            )
            .await
    }

    #[derive(DeriveIden)]
    enum ExternalProviders {
        Table,
        Id,
        ProviderType,
        Name,
        ConfigSchema,
        IsActive,
        CreatedAt,
    }
}

mod external_links {
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        Box::pin(create_external_links_table(manager)).await?;
        Box::pin(create_external_links_indexes(manager)).await?;
        Ok(())
    }

    pub(super) async fn drop(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        Box::pin(async {
            manager
                .drop_table(Table::drop().table(ExternalLinks::Table).to_owned())
                .await
        })
        .await
    }

    async fn create_external_links_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ExternalLinks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ExternalLinks::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(ExternalLinks::OrganizationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ExternalLinks::ProviderId).uuid().not_null())
                    .col(
                        ColumnDef::new(ExternalLinks::ProviderConfig)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ExternalLinks::SyncEnabled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(ExternalLinks::SyncSettings)
                            .json_binary()
                            .not_null()
                            .default(Expr::cust("'{}'::jsonb")),
                    )
                    .col(
                        ColumnDef::new(ExternalLinks::LastSyncAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ExternalLinks::LastSyncStatus)
                            .string_len(20)
                            .null(),
                    )
                    .col(ColumnDef::new(ExternalLinks::SyncError).text().null())
                    .col(
                        ColumnDef::new(ExternalLinks::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ExternalLinks::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_external_links_organization_id")
                            .from(ExternalLinks::Table, ExternalLinks::OrganizationId)
                            .to(Organizations::Table, Organizations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_external_links_provider_id")
                            .from(ExternalLinks::Table, ExternalLinks::ProviderId)
                            .to(ExternalProviders::Table, ExternalProviders::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn create_external_links_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_external_links_organization_id")
                    .table(ExternalLinks::Table)
                    .col(ExternalLinks::OrganizationId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_external_links_sync_enabled_last_sync")
                    .table(ExternalLinks::Table)
                    .col(ExternalLinks::SyncEnabled)
                    .col(ExternalLinks::LastSyncAt)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_external_links_org_provider")
                    .table(ExternalLinks::Table)
                    .col(ExternalLinks::OrganizationId)
                    .col(ExternalLinks::ProviderId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    #[derive(DeriveIden)]
    enum Organizations {
        Table,
        Id,
    }

    #[derive(DeriveIden)]
    enum ExternalProviders {
        Table,
        Id,
    }

    #[derive(DeriveIden)]
    enum ExternalLinks {
        Table,
        Id,
        OrganizationId,
        ProviderId,
        ProviderConfig,
        SyncEnabled,
        SyncSettings,
        LastSyncAt,
        LastSyncStatus,
        SyncError,
        CreatedAt,
        UpdatedAt,
    }
}

mod sync_jobs {
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        Box::pin(create_sync_jobs_table(manager)).await?;
        Box::pin(create_sync_jobs_indexes(manager)).await?;
        Ok(())
    }

    pub(super) async fn drop(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        Box::pin(async {
            manager
                .drop_table(Table::drop().table(SyncJobs::Table).to_owned())
                .await
        })
        .await
    }

    async fn create_sync_jobs_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SyncJobs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SyncJobs::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(SyncJobs::OrganizationExternalLinkId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(SyncJobs::JobType).string_len(50).not_null())
                    .col(ColumnDef::new(SyncJobs::Status).string_len(20).not_null())
                    .col(
                        ColumnDef::new(SyncJobs::ItemsProcessed)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(SyncJobs::ItemsCreated)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(SyncJobs::ItemsUpdated)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(SyncJobs::ItemsFailed)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(SyncJobs::StartedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(SyncJobs::CompletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(ColumnDef::new(SyncJobs::ErrorMessage).text().null())
                    .col(
                        ColumnDef::new(SyncJobs::Details)
                            .json_binary()
                            .not_null()
                            .default(Expr::cust("'{}'::jsonb")),
                    )
                    .col(
                        ColumnDef::new(SyncJobs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_sync_jobs_external_link_id")
                            .from(SyncJobs::Table, SyncJobs::OrganizationExternalLinkId)
                            .to(ExternalLinks::Table, ExternalLinks::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn create_sync_jobs_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_sync_jobs_external_link_id")
                    .table(SyncJobs::Table)
                    .col(SyncJobs::OrganizationExternalLinkId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_sync_jobs_status_started_at")
                    .table(SyncJobs::Table)
                    .col(SyncJobs::Status)
                    .col(SyncJobs::StartedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_sync_jobs_created_at")
                    .table(SyncJobs::Table)
                    .col(SyncJobs::CreatedAt)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    #[derive(DeriveIden)]
    enum ExternalLinks {
        Table,
        Id,
    }

    #[derive(DeriveIden)]
    enum SyncJobs {
        Table,
        Id,
        OrganizationExternalLinkId,
        JobType,
        Status,
        ItemsProcessed,
        ItemsCreated,
        ItemsUpdated,
        ItemsFailed,
        StartedAt,
        CompletedAt,
        ErrorMessage,
        Details,
        CreatedAt,
    }
}

mod permissions {
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Permissions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Permissions::Id)
                            .string_len(36)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Permissions::Level).string_len(20).not_null())
                    .col(ColumnDef::new(Permissions::Description).text().null())
                    .col(
                        ColumnDef::new(Permissions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique index on level
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_permissions_level")
                    .table(Permissions::Table)
                    .col(Permissions::Level)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Insert default permission levels
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Permissions::Table)
                    .columns([
                        Permissions::Id,
                        Permissions::Level,
                        Permissions::Description,
                    ])
                    .values_panic([
                        "read".into(),
                        "read".into(),
                        "Read-only access to resources".into(),
                    ])
                    .values_panic([
                        "write".into(),
                        "write".into(),
                        "Read and write access to resources".into(),
                    ])
                    .values_panic([
                        "admin".into(),
                        "admin".into(),
                        "Full administrative access to resources".into(),
                    ])
                    .values_panic([
                        "owner".into(),
                        "owner".into(),
                        "Full administrative access to resources and ownership".into(),
                    ])
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    pub(super) async fn drop(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Permissions::Table).to_owned())
            .await
    }

    #[derive(DeriveIden)]
    enum Permissions {
        Table,
        Id,
        Level,
        Description,
        CreatedAt,
    }
}

mod resources {
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Resources::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Resources::Id)
                            .string_len(36)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Resources::ResourceType)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Resources::Name).string_len(100).not_null())
                    .col(ColumnDef::new(Resources::Description).text().null())
                    .col(
                        ColumnDef::new(Resources::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique index on resource_type
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_resources_type")
                    .table(Resources::Table)
                    .col(Resources::ResourceType)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Insert default resource types with IDs matching names used by permission fetcher
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Resources::Table)
                    .columns([
                        Resources::Id,
                        Resources::ResourceType,
                        Resources::Name,
                        Resources::Description,
                    ])
                    .values_panic([
                        "organization".into(),
                        "organization".into(),
                        "organization".into(),
                        "Organization management resources".into(),
                    ])
                    .values_panic([
                        "member".into(),
                        "member".into(),
                        "member".into(),
                        "Organization member management".into(),
                    ])
                    .values_panic([
                        "external_link".into(),
                        "external_link".into(),
                        "external_link".into(),
                        "External link management".into(),
                    ])
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    pub(super) async fn drop(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Resources::Table).to_owned())
            .await
    }

    #[derive(DeriveIden)]
    enum Resources {
        Table,
        Id,
        ResourceType,
        Name,
        Description,
        CreatedAt,
    }
}

mod role_permissions {
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        create_role_permissions_table(manager).await?;
        create_role_permissions_indexes(manager).await?;
        Ok(())
    }

    pub(super) async fn drop(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RolePermissions::Table).to_owned())
            .await
    }

    async fn create_role_permissions_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RolePermissions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RolePermissions::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(RolePermissions::Name)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(ColumnDef::new(RolePermissions::Description).text().null())
                    .col(
                        ColumnDef::new(RolePermissions::OrganizationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RolePermissions::PermissionId)
                            .string_len(36)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RolePermissions::ResourceId)
                            .string_len(36)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RolePermissions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_role_permissions_permission_id")
                            .from(RolePermissions::Table, RolePermissions::PermissionId)
                            .to(Permissions::Table, Permissions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_role_permissions_resource_id")
                            .from(RolePermissions::Table, RolePermissions::ResourceId)
                            .to(Resources::Table, Resources::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_role_permissions_organization_id")
                            .from(RolePermissions::Table, RolePermissions::OrganizationId)
                            .to(Organizations::Table, Organizations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn create_role_permissions_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_role_permissions_permission_id")
                    .table(RolePermissions::Table)
                    .col(RolePermissions::PermissionId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_role_permissions_resource_id")
                    .table(RolePermissions::Table)
                    .col(RolePermissions::ResourceId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_role_permissions_organization_id")
                    .table(RolePermissions::Table)
                    .col(RolePermissions::OrganizationId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_role_permissions_org_unique_combo")
                    .table(RolePermissions::Table)
                    .col(RolePermissions::OrganizationId)
                    .col(RolePermissions::PermissionId)
                    .col(RolePermissions::ResourceId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    #[derive(DeriveIden)]
    enum RolePermissions {
        Table,
        Id,
        Name,
        Description,
        OrganizationId,
        PermissionId,
        ResourceId,
        CreatedAt,
    }

    #[derive(DeriveIden)]
    enum Permissions {
        Table,
        Id,
    }

    #[derive(DeriveIden)]
    enum Resources {
        Table,
        Id,
    }

    #[derive(DeriveIden)]
    enum Organizations {
        Table,
        Id,
    }
}

mod organization_member_role_permissions {
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OrganizationMemberRolePermissions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OrganizationMemberRolePermissions::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(OrganizationMemberRolePermissions::MemberId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationMemberRolePermissions::RolePermissionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationMemberRolePermissions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_org_member_role_permissions_organization_id")
                            .from(
                                OrganizationMemberRolePermissions::Table,
                                OrganizationMemberRolePermissions::MemberId,
                            )
                            .to(OrganizationMembers::Table, OrganizationMembers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_org_member_role_permissions_role_permission_id")
                            .from(
                                OrganizationMemberRolePermissions::Table,
                                OrganizationMemberRolePermissions::RolePermissionId,
                            )
                            .to(RolePermissions::Table, RolePermissions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on organization_id and user_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_org_member_role_permissions_org_user")
                    .table(OrganizationMemberRolePermissions::Table)
                    .col(OrganizationMemberRolePermissions::MemberId)
                    .to_owned(),
            )
            .await?;

        // Create index on role_permission_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_org_member_role_permissions_role_permission_id")
                    .table(OrganizationMemberRolePermissions::Table)
                    .col(OrganizationMemberRolePermissions::RolePermissionId)
                    .to_owned(),
            )
            .await?;

        // Create unique constraint to prevent duplicate role permission assignments
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_org_member_role_permissions_unique")
                    .table(OrganizationMemberRolePermissions::Table)
                    .col(OrganizationMemberRolePermissions::MemberId)
                    .col(OrganizationMemberRolePermissions::RolePermissionId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    pub(super) async fn drop(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(OrganizationMemberRolePermissions::Table)
                    .to_owned(),
            )
            .await
    }

    #[derive(DeriveIden)]
    enum OrganizationMemberRolePermissions {
        Table,
        Id,
        MemberId,
        RolePermissionId,
        CreatedAt,
    }

    #[derive(DeriveIden)]
    enum OrganizationMembers {
        Table,
        Id,
    }

    #[derive(DeriveIden)]
    enum RolePermissions {
        Table,
        Id,
    }
}

mod outbox {
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OutboxEvents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OutboxEvents::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(OutboxEvents::EventId)
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(OutboxEvents::EventType).text().not_null())
                    .col(ColumnDef::new(OutboxEvents::AggregateId).uuid().not_null())
                    .col(ColumnDef::new(OutboxEvents::Version).integer().not_null())
                    .col(
                        ColumnDef::new(OutboxEvents::OccurredAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OutboxEvents::PayloadJson)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OutboxEvents::MetadataJson)
                            .json_binary()
                            .not_null(),
                    )
                    .col(ColumnDef::new(OutboxEvents::Status).text().not_null())
                    .col(
                        ColumnDef::new(OutboxEvents::Attempts)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(OutboxEvents::NextAttemptAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(ColumnDef::new(OutboxEvents::LockedBy).text())
                    .col(ColumnDef::new(OutboxEvents::LockedUntil).timestamp_with_time_zone())
                    .col(ColumnDef::new(OutboxEvents::LastError).text())
                    .col(
                        ColumnDef::new(OutboxEvents::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(OutboxEvents::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_rustycog_outbox_events_claim")
                    .table(OutboxEvents::Table)
                    .col(OutboxEvents::Status)
                    .col(OutboxEvents::NextAttemptAt)
                    .col(OutboxEvents::CreatedAt)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_rustycog_outbox_events_aggregate")
                    .table(OutboxEvents::Table)
                    .col(OutboxEvents::AggregateId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await
    }

    pub(super) async fn drop(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(OutboxEvents::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }

    enum OutboxEvents {
        Table,
        Id,
        EventId,
        EventType,
        AggregateId,
        Version,
        OccurredAt,
        PayloadJson,
        MetadataJson,
        Status,
        Attempts,
        NextAttemptAt,
        LockedBy,
        LockedUntil,
        LastError,
        CreatedAt,
        UpdatedAt,
    }

    impl Iden for OutboxEvents {
        fn unquoted(&self, s: &mut dyn std::fmt::Write) {
            let ident = match self {
                Self::Table => "rustycog_outbox_events",
                Self::Id => "id",
                Self::EventId => "event_id",
                Self::EventType => "event_type",
                Self::AggregateId => "aggregate_id",
                Self::Version => "version",
                Self::OccurredAt => "occurred_at",
                Self::PayloadJson => "payload_json",
                Self::MetadataJson => "metadata_json",
                Self::Status => "status",
                Self::Attempts => "attempts",
                Self::NextAttemptAt => "next_attempt_at",
                Self::LockedBy => "locked_by",
                Self::LockedUntil => "locked_until",
                Self::LastError => "last_error",
                Self::CreatedAt => "created_at",
                Self::UpdatedAt => "updated_at",
            };

            if s.write_str(ident).is_err() {}
        }
    }
}
