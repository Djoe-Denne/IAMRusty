use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_organization_roles_organization_id")
                            .from(OrganizationRoles::Table, OrganizationRoles::OrganizationId)
                            .to(Organizations::Table, Organizations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organization_roles_organization_id")
                    .table(OrganizationRoles::Table)
                    .col(OrganizationRoles::OrganizationId)
                    .to_owned(),
            )
            .await?;

        // Unique constraint on organization_id + name
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_organization_roles_org_name")
                    .table(OrganizationRoles::Table)
                    .col(OrganizationRoles::OrganizationId)
                    .col(OrganizationRoles::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(OrganizationRoles::Table).to_owned())
            .await
    }
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
    OrganizationId,
    Name,
    Description,
    Permissions,
    IsSystemDefault,
    CreatedAt,
} 