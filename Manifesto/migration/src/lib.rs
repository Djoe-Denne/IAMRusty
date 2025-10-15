pub use sea_orm_migration::prelude::*;

mod m20241015_000001_create_projects_table;
mod m20241015_000002_create_project_components_table;
mod m20241015_000003_create_project_members_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20241015_000001_create_projects_table::Migration),
            Box::new(m20241015_000002_create_project_components_table::Migration),
            Box::new(m20241015_000003_create_project_members_table::Migration),
        ]
    }
}


