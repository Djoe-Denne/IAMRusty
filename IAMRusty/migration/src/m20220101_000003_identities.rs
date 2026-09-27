use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_identities_table(manager).await?;
        backfill_platform_identities(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Identities::Table).to_owned())
            .await
    }
}

async fn create_identities_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Identities::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(Identities::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(Identities::UserId).uuid().not_null())
                .col(ColumnDef::new(Identities::Issuer).string().not_null())
                .col(ColumnDef::new(Identities::Subject).string().not_null())
                .col(ColumnDef::new(Identities::Kind).string().not_null())
                .col(
                    ColumnDef::new(Identities::CreatedAt)
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(Identities::UpdatedAt)
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_identities_user_id")
                        .from(Identities::Table, Identities::UserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("idx_identities_issuer_subject")
                .table(Identities::Table)
                .col(Identities::Issuer)
                .col(Identities::Subject)
                .unique()
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("idx_identities_user_id")
                .table(Identities::Table)
                .col(Identities::UserId)
                .to_owned(),
        )
        .await
}

/// Backfill PlatformIdentity rows: subject = user.id, issuer = platform URL placeholder.
///
/// Runtime login also calls `ensure_platform_identity` with the configured
/// `public_base_url`-derived issuer; this SQL uses the historical platform issuer
/// so existing HS256 tokens (`iss=iamrusty`) keep matching until cutover.
async fn backfill_platform_identities(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let db = manager.get_connection();
    db.execute_unprepared(
        r#"
        INSERT INTO identities (id, user_id, issuer, subject, kind, created_at, updated_at)
        SELECT gen_random_uuid(), id, 'iamrusty', id::text, 'platform', NOW(), NOW()
        FROM users
        ON CONFLICT (issuer, subject) DO NOTHING
        "#,
    )
    .await?;
    Ok(())
}

#[derive(Iden)]
enum Identities {
    Table,
    Id,
    UserId,
    Issuer,
    Subject,
    Kind,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}
