pub use sea_orm_migration::prelude::*;

mod m20241015_000001_initial_schema;
mod m20261020_000001_apparatus_runtime_control;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20241015_000001_initial_schema::Migration),
            Box::new(m20261020_000001_apparatus_runtime_control::Migration),
        ]
    }
}
