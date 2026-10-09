//! Initial Manifesto schema, flattened on 2026-10-04. Incremental ALTERs are folded into CREATEs.
//! This migration initializes an empty database; it is not an upgrade of a historical ledger.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        Box::pin(projects::create(manager)).await?;
        Box::pin(project_components::create(manager)).await?;
        Box::pin(project_members::create(manager)).await?;
        Box::pin(permissions::create(manager)).await?;
        Box::pin(resources::create(manager)).await?;
        Box::pin(role_permissions::create(manager)).await?;
        Box::pin(project_member_role_permissions::create(manager)).await?;
        Box::pin(catalog_seed::create(manager)).await?;
        Box::pin(apparatus::create(manager)).await?;
        Box::pin(outbox::create(manager)).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        outbox::drop(manager).await?;
        apparatus::drop(manager).await?;
        catalog_seed::drop(manager).await?;
        project_member_role_permissions::drop(manager).await?;
        role_permissions::drop(manager).await?;
        resources::drop(manager).await?;
        permissions::drop(manager).await?;
        project_members::drop(manager).await?;
        project_components::drop(manager).await?;
        projects::drop(manager).await?;
        Ok(())
    }
}

mod projects {
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        create_projects_table(manager).await?;
        create_projects_indexes(manager).await?;
        Ok(())
    }

    pub(super) async fn drop(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Projects::Table).to_owned())
            .await
    }

    async fn create_projects_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Projects::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Projects::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(Projects::Name).string_len(255).not_null())
                    .col(ColumnDef::new(Projects::Description).text().null())
                    .col(
                        ColumnDef::new(Projects::Status)
                            .string_len(50)
                            .not_null()
                            .default("draft")
                            .check(Expr::col(Projects::Status).is_in([
                                "draft",
                                "active",
                                "archived",
                                "suspended",
                            ])),
                    )
                    .col(
                        ColumnDef::new(Projects::OwnerType)
                            .string_len(50)
                            .not_null()
                            .check(
                                Expr::col(Projects::OwnerType).is_in(["personal", "organization"]),
                            ),
                    )
                    .col(ColumnDef::new(Projects::OwnerId).uuid().not_null())
                    .col(ColumnDef::new(Projects::CreatedBy).uuid().not_null())
                    .col(
                        ColumnDef::new(Projects::Visibility)
                            .string_len(50)
                            .not_null()
                            .default("private")
                            .check(
                                Expr::col(Projects::Visibility)
                                    .is_in(["private", "internal", "public"]),
                            ),
                    )
                    .col(
                        ColumnDef::new(Projects::ExternalCollaborationEnabled)
                            .boolean()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Projects::DataClassification)
                            .string_len(50)
                            .default("internal"),
                    )
                    .col(
                        ColumnDef::new(Projects::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Projects::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Projects::PublishedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Projects::Revision)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn create_projects_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_projects_owner")
                    .table(Projects::Table)
                    .col(Projects::OwnerType)
                    .col(Projects::OwnerId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_projects_status")
                    .table(Projects::Table)
                    .col(Projects::Status)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_projects_created_by")
                    .table(Projects::Table)
                    .col(Projects::CreatedBy)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    #[derive(DeriveIden)]
    pub enum Projects {
        Table,
        Id,
        Name,
        Description,
        Status,
        OwnerType,
        OwnerId,
        CreatedBy,
        Visibility,
        ExternalCollaborationEnabled,
        DataClassification,
        CreatedAt,
        UpdatedAt,
        PublishedAt,
        Revision,
    }
}

mod project_components {
    use super::projects::Projects;
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        create_project_components_table(manager).await?;
        create_project_components_indexes(manager).await?;
        Ok(())
    }

    async fn create_project_components_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ProjectComponents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProjectComponents::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(ProjectComponents::ProjectId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProjectComponents::ComponentType)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProjectComponents::Status)
                            .string_len(50)
                            .not_null()
                            .default("pending")
                            .check(Expr::col(ProjectComponents::Status).is_in([
                                "pending",
                                "configured",
                                "active",
                                "disabled",
                            ])),
                    )
                    .col(
                        ColumnDef::new(ProjectComponents::AddedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ProjectComponents::ConfiguredAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ProjectComponents::ActivatedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ProjectComponents::DisabledAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_project_components_project")
                            .from(ProjectComponents::Table, ProjectComponents::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn create_project_components_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("project_components_unique")
                    .table(ProjectComponents::Table)
                    .col(ProjectComponents::ProjectId)
                    .col(ProjectComponents::ComponentType)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_components_project")
                    .table(ProjectComponents::Table)
                    .col(ProjectComponents::ProjectId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_components_status")
                    .table(ProjectComponents::Table)
                    .col(ProjectComponents::ProjectId)
                    .col(ProjectComponents::Status)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    pub(super) async fn drop(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ProjectComponents::Table).to_owned())
            .await
    }

    #[derive(DeriveIden)]
    enum ProjectComponents {
        Table,
        Id,
        ProjectId,
        ComponentType,
        Status,
        AddedAt,
        ConfiguredAt,
        ActivatedAt,
        DisabledAt,
    }
}

mod project_members {
    use super::projects::Projects;
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        create_project_members_table(manager).await?;
        create_project_members_indexes(manager).await?;
        Ok(())
    }

    pub(super) async fn drop(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ProjectMembers::Table).to_owned())
            .await
    }

    async fn create_project_members_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ProjectMembers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProjectMembers::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(ProjectMembers::ProjectId).uuid().not_null())
                    .col(ColumnDef::new(ProjectMembers::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(ProjectMembers::Source)
                            .string_len(50)
                            .not_null()
                            .default("direct")
                            .check(Expr::col(ProjectMembers::Source).is_in([
                                "direct",
                                "org_cascade",
                                "invitation",
                                "third_party_sync",
                            ])),
                    )
                    .col(ColumnDef::new(ProjectMembers::AddedBy).uuid().null())
                    .col(
                        ColumnDef::new(ProjectMembers::AddedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ProjectMembers::RemovedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ProjectMembers::RemovalReason)
                            .string_len(100)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ProjectMembers::GracePeriodEndsAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ProjectMembers::LastAccessAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ProjectMembers::IsOwner)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_project_members_project")
                            .from(ProjectMembers::Table, ProjectMembers::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn create_project_members_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_members_project")
                    .table(ProjectMembers::Table)
                    .col(ProjectMembers::ProjectId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_members_user")
                    .table(ProjectMembers::Table)
                    .col(ProjectMembers::UserId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_members_active")
                    .table(ProjectMembers::Table)
                    .col(ProjectMembers::ProjectId)
                    .col(ProjectMembers::RemovedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .get_connection()
            .execute_unprepared(
                r"
    CREATE UNIQUE INDEX IF NOT EXISTS project_members_one_active
        ON project_members (project_id, user_id) WHERE removed_at IS NULL;
    CREATE UNIQUE INDEX IF NOT EXISTS project_members_one_owner
        ON project_members (project_id) WHERE is_owner AND removed_at IS NULL;
    CREATE INDEX IF NOT EXISTS idx_project_members_restorable
        ON project_members (project_id, user_id, grace_period_ends_at) WHERE removed_at IS NOT NULL;
    ",
            )
            .await?;
        Ok(())
    }

    #[derive(DeriveIden)]
    pub enum ProjectMembers {
        Table,
        Id,
        ProjectId,
        UserId,
        Source,
        AddedBy,
        AddedAt,
        RemovedAt,
        RemovalReason,
        GracePeriodEndsAt,
        LastAccessAt,
        IsOwner,
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
                    .columns([Permissions::Id, Permissions::Level])
                    .values_panic(["read".into(), "read".into()])
                    .values_panic(["write".into(), "write".into()])
                    .values_panic(["admin".into(), "admin".into()])
                    .values_panic(["owner".into(), "owner".into()])
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
                    .col(
                        ColumnDef::new(Resources::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Resource types can repeat; the final index is non-unique.
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_resources_type_non_unique")
                    .table(Resources::Table)
                    .col(Resources::ResourceType)
                    .to_owned(),
            )
            .await?;

        // Insert default internal resource types
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Resources::Table)
                    .columns([Resources::Id, Resources::ResourceType, Resources::Name])
                    .values_panic(["project".into(), "project".into(), "project".into()])
                    .values_panic(["component".into(), "component".into(), "component".into()])
                    .values_panic(["member".into(), "member".into(), "member".into()])
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
        CreatedAt,
    }
}

mod role_permissions {
    use super::projects::Projects;
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
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
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(RolePermissions::Name).string_len(100).null())
                    .col(ColumnDef::new(RolePermissions::ProjectId).uuid().not_null())
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
                            .name("fk_role_permissions_project_id")
                            .from(RolePermissions::Table, RolePermissions::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on permission_id
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

        // Create index on resource_id
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

        // Create index on project_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_role_permissions_project_id")
                    .table(RolePermissions::Table)
                    .col(RolePermissions::ProjectId)
                    .to_owned(),
            )
            .await?;

        // Create unique constraint to prevent duplicate permission-resource combinations per project
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_role_permissions_unique_combo")
                    .table(RolePermissions::Table)
                    .col(RolePermissions::ProjectId)
                    .col(RolePermissions::PermissionId)
                    .col(RolePermissions::ResourceId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    pub(super) async fn drop(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RolePermissions::Table).to_owned())
            .await
    }

    #[derive(DeriveIden)]
    enum RolePermissions {
        Table,
        Id,
        Name,
        ProjectId,
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
}

mod project_member_role_permissions {
    use super::project_members::ProjectMembers;
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ProjectMemberRolePermissions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProjectMemberRolePermissions::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(ProjectMemberRolePermissions::MemberId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProjectMemberRolePermissions::RolePermissionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProjectMemberRolePermissions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_project_member_role_permissions_member_id")
                            .from(
                                ProjectMemberRolePermissions::Table,
                                ProjectMemberRolePermissions::MemberId,
                            )
                            .to(ProjectMembers::Table, ProjectMembers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_project_member_role_permissions_role_permission_id")
                            .from(
                                ProjectMemberRolePermissions::Table,
                                ProjectMemberRolePermissions::RolePermissionId,
                            )
                            .to(RolePermissions::Table, RolePermissions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on member_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_member_role_permissions_member_id")
                    .table(ProjectMemberRolePermissions::Table)
                    .col(ProjectMemberRolePermissions::MemberId)
                    .to_owned(),
            )
            .await?;

        // Create index on role_permission_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_member_role_permissions_role_permission_id")
                    .table(ProjectMemberRolePermissions::Table)
                    .col(ProjectMemberRolePermissions::RolePermissionId)
                    .to_owned(),
            )
            .await?;

        // Create unique constraint to prevent duplicate role permission assignments
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_project_member_role_permissions_unique")
                    .table(ProjectMemberRolePermissions::Table)
                    .col(ProjectMemberRolePermissions::MemberId)
                    .col(ProjectMemberRolePermissions::RolePermissionId)
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
                    .table(ProjectMemberRolePermissions::Table)
                    .to_owned(),
            )
            .await
    }

    #[derive(DeriveIden)]
    enum ProjectMemberRolePermissions {
        Table,
        Id,
        MemberId,
        RolePermissionId,
        CreatedAt,
    }

    #[derive(DeriveIden)]
    enum RolePermissions {
        Table,
        Id,
    }
}

mod catalog_seed {
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        // Seed permissions table with default permission levels
        // Use ON CONFLICT DO NOTHING to make this idempotent
        let insert_permissions = Query::insert()
            .into_table(Permissions::Table)
            .columns([Permissions::Id, Permissions::Level])
            .values_panic(["read".into(), "read".into()])
            .values_panic(["write".into(), "write".into()])
            .values_panic(["admin".into(), "admin".into()])
            .values_panic(["owner".into(), "owner".into()])
            .on_conflict(OnConflict::column(Permissions::Id).do_nothing().to_owned())
            .to_owned();

        manager.exec_stmt(insert_permissions).await?;

        // Seed resources table with default internal resources
        // Use ON CONFLICT DO NOTHING to make this idempotent
        let insert_resources = Query::insert()
            .into_table(Resources::Table)
            .columns([Resources::Id, Resources::ResourceType, Resources::Name])
            .values_panic(["project".into(), "internal".into(), "Project".into()])
            .values_panic(["member".into(), "internal".into(), "Member".into()])
            .on_conflict(OnConflict::column(Resources::Id).do_nothing().to_owned())
            .to_owned();

        manager.exec_stmt(insert_resources).await?;

        Ok(())
    }

    pub(super) async fn drop(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        // Delete seeded resources
        let delete_resources = Query::delete()
            .from_table(Resources::Table)
            .and_where(Expr::col(Resources::Id).is_in(["project", "member"]))
            .to_owned();

        manager.exec_stmt(delete_resources).await?;

        // Delete seeded permissions
        let delete_permissions = Query::delete()
            .from_table(Permissions::Table)
            .and_where(Expr::col(Permissions::Id).is_in(["read", "write", "admin", "owner"]))
            .to_owned();

        manager.exec_stmt(delete_permissions).await?;

        Ok(())
    }

    #[derive(DeriveIden)]
    enum Permissions {
        Table,
        Id,
        Level,
    }

    #[derive(DeriveIden)]
    enum Resources {
        Table,
        Id,
        ResourceType,
        Name,
    }
}

mod apparatus {
    use sea_orm_migration::prelude::*;

    pub(super) async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(r"
    CREATE TABLE IF NOT EXISTS apparatus_bindings (
        id BIGSERIAL NOT NULL PRIMARY KEY,
        component_id UUID NOT NULL UNIQUE,
        digest VARCHAR(128),
        source VARCHAR(20) NOT NULL CHECK (source IN ('legacy', 'managed')),
        desired_generation BIGINT NOT NULL DEFAULT 0 CHECK (desired_generation >= 0),
        observed_generation BIGINT NOT NULL DEFAULT 0 CHECK (observed_generation >= 0),
        observed_digest VARCHAR(128),
        lease_epoch BIGINT NOT NULL DEFAULT 0 CHECK (lease_epoch >= 0),
        lease_owner VARCHAR(64) NOT NULL DEFAULT '',
        lease_expires_at TIMESTAMPTZ,
        next_retry_at TIMESTAMPTZ,
        retry_count INTEGER NOT NULL DEFAULT 0 CHECK (retry_count >= 0),
        last_error_code VARCHAR(64),
        grant_revision BIGINT NOT NULL DEFAULT 0,
        declared_capabilities JSONB NOT NULL DEFAULT '[]'::jsonb,
        CONSTRAINT fk_apparatus_bindings_component FOREIGN KEY (component_id)
            REFERENCES project_components (id) ON DELETE CASCADE,
        CONSTRAINT chk_apparatus_bindings_observed_le_desired CHECK (observed_generation <= desired_generation),
        CONSTRAINT chk_apparatus_bindings_lease_owned CHECK (
            (lease_owner = '' AND lease_expires_at IS NULL)
            OR (lease_owner <> '' AND lease_expires_at IS NOT NULL)),
        CONSTRAINT chk_apparatus_bindings_grant_revision_non_negative CHECK (grant_revision >= 0)
    );
    CREATE INDEX IF NOT EXISTS idx_apparatus_bindings_managed_due
        ON apparatus_bindings (next_retry_at)
        WHERE source = 'managed' AND desired_generation > observed_generation;
    CREATE INDEX IF NOT EXISTS idx_apparatus_bindings_lease_expiry
        ON apparatus_bindings (lease_expires_at) WHERE source = 'managed' AND lease_owner <> '';
    CREATE TABLE IF NOT EXISTS apparatus_cleanup_jobs (
        id BIGSERIAL NOT NULL PRIMARY KEY,
        component_id UUID NOT NULL,
        project_id UUID NOT NULL,
        desired_generation BIGINT NOT NULL,
        digest VARCHAR(128),
        next_retry_at TIMESTAMPTZ,
        retry_count INTEGER NOT NULL DEFAULT 0 CHECK (retry_count >= 0),
        last_error_code VARCHAR(64),
        completed_at TIMESTAMPTZ,
        created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE UNIQUE INDEX IF NOT EXISTS uq_apparatus_cleanup_jobs_open
        ON apparatus_cleanup_jobs (component_id) WHERE completed_at IS NULL;
    CREATE INDEX IF NOT EXISTS idx_apparatus_cleanup_jobs_due
        ON apparatus_cleanup_jobs (next_retry_at) WHERE completed_at IS NULL;
    CREATE TABLE IF NOT EXISTS apparatus_capability_consents (
        id BIGSERIAL NOT NULL PRIMARY KEY,
        component_id UUID NOT NULL,
        capability TEXT NOT NULL,
        status TEXT NOT NULL,
        grant_revision BIGINT NOT NULL,
        CONSTRAINT fk_apparatus_capability_consents_component FOREIGN KEY (component_id)
            REFERENCES project_components (id) ON DELETE CASCADE,
        CONSTRAINT chk_apparatus_capability_consents_status CHECK (status IN ('consented', 'revoked')),
        CONSTRAINT chk_apparatus_capability_consents_grant_revision_non_negative CHECK (grant_revision >= 0),
        CONSTRAINT uq_apparatus_capability_consents_component_capability UNIQUE (component_id, capability)
    );
    ").await?;
        Ok(())
    }

    pub(super) async fn drop(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r"
    DROP TABLE IF EXISTS apparatus_capability_consents;
    DROP TABLE IF EXISTS apparatus_cleanup_jobs;
    DROP TABLE IF EXISTS apparatus_bindings;
    ",
            )
            .await?;
        Ok(())
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
