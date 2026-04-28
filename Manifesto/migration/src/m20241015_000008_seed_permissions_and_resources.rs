use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
