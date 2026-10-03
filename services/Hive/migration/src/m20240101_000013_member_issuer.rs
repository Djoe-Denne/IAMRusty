use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(OrganizationMembers::Table)
                    .add_column(
                        ColumnDef::new(OrganizationMembers::Issuer)
                            .string()
                            .not_null()
                            .default("iamrusty"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_organization_members_org_user")
                    .table(OrganizationMembers::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_organization_members_org_issuer_user")
                    .table(OrganizationMembers::Table)
                    .col(OrganizationMembers::OrganizationId)
                    .col(OrganizationMembers::Issuer)
                    .col(OrganizationMembers::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // UX-only org signing metadata (no secrets).
        manager
            .alter_table(
                Table::alter()
                    .table(Organizations::Table)
                    .add_column(
                        ColumnDef::new(Organizations::SigningProfileId)
                            .uuid()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Organizations::Table)
                    .add_column(ColumnDef::new(Organizations::SigningStatus).string().null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Organizations::Table)
                    .drop_column(Organizations::SigningStatus)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Organizations::Table)
                    .drop_column(Organizations::SigningProfileId)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_organization_members_org_issuer_user")
                    .table(OrganizationMembers::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_organization_members_org_user")
                    .table(OrganizationMembers::Table)
                    .col(OrganizationMembers::OrganizationId)
                    .col(OrganizationMembers::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(OrganizationMembers::Table)
                    .drop_column(OrganizationMembers::Issuer)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum OrganizationMembers {
    Table,
    OrganizationId,
    UserId,
    Issuer,
}

#[derive(Iden)]
enum Organizations {
    Table,
    SigningProfileId,
    SigningStatus,
}
