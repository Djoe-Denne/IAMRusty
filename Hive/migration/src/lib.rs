pub use sea_orm_migration::prelude::*;

mod m20240101_000001_create_organizations_table;
mod m20240101_000003_create_organization_members_table;
mod m20240101_000004_create_organization_invitations_table;
mod m20240101_000005_create_external_providers_table;
mod m20240101_000006_create_external_links_table;
mod m20240101_000007_create_sync_jobs_table;
mod m20240101_000008_create_permissions_table;
mod m20240101_000009_create_resources_table;
mod m20240101_000010_create_role_permissions_table;
mod m20240101_000012_create_organization_member_role_permissions_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_organizations_table::Migration),
            Box::new(m20240101_000003_create_organization_members_table::Migration),
            Box::new(m20240101_000004_create_organization_invitations_table::Migration),
            Box::new(m20240101_000005_create_external_providers_table::Migration),
            Box::new(m20240101_000006_create_external_links_table::Migration),
            Box::new(m20240101_000007_create_sync_jobs_table::Migration),
            Box::new(m20240101_000008_create_permissions_table::Migration),
            Box::new(m20240101_000009_create_resources_table::Migration),
            Box::new(m20240101_000010_create_role_permissions_table::Migration),
            Box::new(m20240101_000012_create_organization_member_role_permissions_table::Migration),
            rustycog_outbox::outbox_migration(),
        ]
    }
}
