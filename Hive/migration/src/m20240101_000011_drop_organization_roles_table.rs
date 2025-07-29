use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // First drop the foreign key constraints that reference organization_roles
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_organization_members_role_id")
                    .table(OrganizationMembers::Table)
                    .to_owned()
            )
            .await?;

        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_organization_invitations_role_id") 
                    .table(OrganizationInvitations::Table)
                    .to_owned()
            )
            .await?;

        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_role_permissions_organization_role_id")
                    .table(RolePermissions::Table)
                    .to_owned()
            )
            .await?;

        // Drop the organization_roles table
        manager
            .drop_table(Table::drop().table(OrganizationRoles::Table).to_owned())
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Recreate the organization_roles table
        manager
            .create_table(
                Table::create()
                    .table(OrganizationRoles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OrganizationRoles::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(OrganizationRoles::OrganizationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationRoles::Name)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationRoles::Description)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(OrganizationRoles::Permissions)
                            .json_binary()
                            .not_null()
                            .default("'[]'".to_owned()),
                    )
                    .col(
                        ColumnDef::new(OrganizationRoles::IsSystemDefault)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(OrganizationRoles::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum OrganizationRoles {
    Table,
    Id,
    OrganizationId,
    Name,
    Description,
    Permissions,
    IsSystemDefault,
    CreatedAt,
}

#[derive(DeriveIden)]
enum OrganizationMembers {
    Table,
}

#[derive(DeriveIden)]
enum OrganizationInvitations {
    Table,
}

#[derive(DeriveIden)]
enum RolePermissions {
    Table,
} 